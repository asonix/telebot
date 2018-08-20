//! This is the actual Bot module. For ergonomic reasons there is a Bot which uses the real bot
//! as an underlying field. You should always use Bot.

use error::{ErrorKind, TelegramError};
use failure::{Error, Fail, ResultExt};
use file::File;
use functions::FunctionGetMe;
use objects;

use std::{
    collections::HashMap,
    str,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};

use futures::{
    stream,
    sync::mpsc::{self, UnboundedSender},
    Future, IntoFuture, Stream,
};
use hyper::{
    client::{HttpConnector, ResponseFuture},
    header::CONTENT_TYPE,
    Body, Client, Request, Uri,
};
use hyper_multipart::client::multipart;
use hyper_tls::HttpsConnector;
use serde_json::{self, value::Value};
use tokio::{self, timer::Interval};

pub struct BotBuilder {
    pub key: String,
    pub name: Option<String>,
    pub last_id: Arc<AtomicU32>,
    pub update_interval: u64,
    pub timeout: u64,
    pub handlers: HashMap<String, UnboundedSender<(Bot, objects::Message)>>,
    pub unknown_handler: Option<UnboundedSender<(Bot, objects::Message)>>,
}

impl BotBuilder {
    pub fn new(key: &str) -> BotBuilder {
        debug!("Create a new bot with the key {}", key);

        BotBuilder {
            key: key.into(),
            name: None,
            last_id: Arc::new(AtomicU32::new(0)),
            update_interval: 1000,
            timeout: 30,
            handlers: HashMap::new(),
            unknown_handler: None,
        }
    }

    pub fn set_name(&mut self, name: &str) -> &mut Self {
        self.name = Some(name.to_owned());
        self
    }

    pub fn set_last_id(&mut self, last_id: u32) -> &mut Self {
        self.last_id.store(last_id, Ordering::Relaxed);
        self
    }

    pub fn set_timeout(&mut self, timeout: u64) -> &mut Self {
        self.timeout = timeout;
        self
    }

    /// Creates a new command and returns a stream which will yield a message when the command is send
    pub fn new_cmd(
        &mut self,
        cmd: &str,
        handler: impl Fn((Bot, objects::Message)) -> Box<dyn Future<Item = (), Error = ()> + Send>
            + Send
            + 'static,
    ) -> &mut Self {
        let (sender, receiver) = mpsc::unbounded();

        let cmd = if cmd.starts_with("/") {
            cmd.into()
        } else {
            format!("/{}", cmd)
        };

        self.handlers.insert(cmd.into(), sender);

        tokio::executor::spawn(
            receiver
                .map_err(|_| ())
                .and_then(handler)
                .or_else(|_| Ok(()))
                .for_each(|_| Ok(())),
        );

        self
    }

    /// Returns a stream which will yield a message when none of previously registered commands matches
    pub fn unknown_cmd(
        &mut self,
        handler: impl Fn((Bot, objects::Message)) -> Box<dyn Future<Item = (), Error = ()> + Send>
            + Send
            + 'static,
    ) -> &mut Self {
        let (sender, receiver) = mpsc::unbounded();

        self.unknown_handler = Some(sender);

        tokio::executor::spawn(
            receiver
                .map_err(|_| ())
                .and_then(handler)
                .or_else(|_| Ok(()))
                .for_each(|_| Ok(())),
        );

        self
    }

    pub fn build(&mut self) -> Bot {
        Bot {
            key: self.key.clone(),
            name: self.name.clone(),
            last_id: Arc::clone(&self.last_id),
            update_interval: self.update_interval,
            timeout: self.timeout,
            handlers: self.handlers.clone(),
            unknown_handler: self.unknown_handler.clone(),
        }
    }
}

/// The main bot structure
#[derive(Clone)]
pub struct Bot {
    key: String,
    name: Option<String>,
    last_id: Arc<AtomicU32>,
    update_interval: u64,
    timeout: u64,
    handlers: HashMap<String, UnboundedSender<(Bot, objects::Message)>>,
    unknown_handler: Option<UnboundedSender<(Bot, objects::Message)>>,
}

impl Bot {
    /// Creates a new request and adds a JSON message to it. The returned Future contains a the
    /// reply as a string.  This method should be used if no file is added becontext a JSON msg is
    /// always compacter than a formdata one.
    pub fn fetch_json(
        &self,
        func: &'static str,
        msg: &str,
    ) -> impl Future<Item = String, Error = Error> {
        debug!("Send JSON: {}", msg);

        let request = self.build_json(func, String::from(msg));

        request
            .into_future()
            .and_then(|(client, request)| _fetch(client.request(request)))
    }

    /// Builds the HTTP header for a JSON request. The JSON is already converted to a str and is
    /// appended to the POST header.
    fn build_json(
        &self,
        func: &'static str,
        msg: String,
    ) -> Result<(Client<HttpsConnector<HttpConnector>, Body>, Request<Body>), Error> {
        let url: Result<Uri, _> =
            format!("https://api.telegram.org/bot{}/{}", self.key, func).parse();

        let client = Client::builder()
            .build(HttpsConnector::new(2).context(ErrorKind::HttpsInitializeError)?);

        let req = Request::post(url.context(ErrorKind::Uri)?)
            .header(CONTENT_TYPE, "application/json")
            .body(msg.into())
            .context(ErrorKind::Hyper)?;

        Ok((client, req))
    }

    /// Creates a new request with some byte content (e.g. a file). The method properties have to be
    /// in the formdata setup and cannot be sent as JSON.
    pub fn fetch_formdata(
        &self,
        func: &'static str,
        msg: &Value,
        file: File,
        kind: &str,
    ) -> impl Future<Item = String, Error = Error> {
        debug!("Send formdata: {}", msg.to_string());

        let request = self.build_formdata(func, msg, file, kind);

        request
            .into_future()
            .and_then(|(client, request)| _fetch(client.request(request)))
    }

    /// Builds the HTTP header for a formdata request. The file content is read and then append to
    /// the formdata. Each key-value pair has a own line.
    fn build_formdata(
        &self,
        func: &'static str,
        msg: &Value,
        file: File,
        kind: &str,
    ) -> Result<(Client<HttpsConnector<HttpConnector>, Body>, Request<Body>), Error> {
        let client: Client<HttpsConnector<_>, Body> = Client::builder()
            .keep_alive(true)
            .build(HttpsConnector::new(4).context(ErrorKind::HttpsInitializeError)?);

        let url: Result<Uri, _> =
            format!("https://api.telegram.org/bot{}/{}", self.key, func).parse();

        let mut req_builder = Request::post(url.context(ErrorKind::Uri)?);
        let mut form = multipart::Form::default();

        let msg = msg.as_object().ok_or(ErrorKind::JsonNotMap)?;

        // add properties
        for (key, val) in msg.iter() {
            let val = match val {
                &Value::String(ref val) => format!("{}", val),
                etc => format!("{}", etc),
            };

            form.add_text(key, val.as_ref());
        }

        match file {
            File::Memory { name, source } => {
                form.add_reader_file(kind, source, name);
            }
            File::Disk { path } => {
                form.add_file(kind, path).context(ErrorKind::NoFile)?;
            }
        }

        let req = form.set_body(&mut req_builder).context(ErrorKind::Hyper)?;

        Ok((client, req))
    }

    /// The main update loop, the update function is called every update_interval milliseconds
    /// When an update is available the last_id will be updated and the message is filtered
    /// for commands
    /// The message is forwarded to the returned stream if no command was found
    pub fn get_stream(self) -> impl Stream<Item = (Bot, objects::Update), Error = Error> {
        use functions::*;

        let bot1 = self.clone();
        let bot2 = self.clone();
        let bot3 = self.clone();

        let duration = Duration::from_millis(self.update_interval);
        Interval::new(Instant::now(), duration)
            .map_err(|x| Error::from(x.context(ErrorKind::IntervalTimer)))
            .and_then(move |_| {
                bot1.clone()
                    .get_updates()
                    .offset(self.last_id.load(Ordering::Relaxed))
                    .timeout(self.timeout as i64)
                    .send()
            }).map(|(_, x)| {
                stream::iter_result(
                    x.0.into_iter()
                        .map(|x| Ok(x))
                        .collect::<Vec<Result<objects::Update, Error>>>(),
                )
            }).flatten()
            .and_then(move |x| {
                bot2.last_id
                    .fetch_max(x.update_id as u32 + 1, Ordering::Relaxed);

                Ok(x)
            }).filter_map(move |mut val| {
                debug!("Got an update from Telegram: {:?}", val);

                let mut sndr: Option<UnboundedSender<(Bot, objects::Message)>> = None;

                if let Some(ref mut message) = val.message {
                    if let Some(text) = message.text.clone() {
                        let mut content = text.split_whitespace();
                        if let Some(mut cmd) = content.next() {
                            if cmd.starts_with("/") {
                                if let Some(name) = bot3.name.as_ref() {
                                    if cmd.ends_with(name.as_str()) {
                                        cmd = cmd.rsplitn(2, '@').skip(1).next().unwrap();
                                    }
                                }
                                if let Some(sender) = bot3.handlers.get(cmd) {
                                    sndr = Some(sender.clone());
                                    message.text = Some(content.collect::<Vec<&str>>().join(" "));
                                } else if let Some(ref sender) = bot3.unknown_handler {
                                    sndr = Some(sender.clone());
                                }
                            }
                        }
                    }
                }

                if let Some(sender) = sndr {
                    sender
                        .unbounded_send((bot3.clone(), val.message.unwrap()))
                        .unwrap_or_else(|e| error!("Error: {}", e));
                    return None;
                } else {
                    return Some((bot3.clone(), val));
                }
            })
    }

    /// helper function to start the event loop
    pub fn run<'a>(self) {
        // create a local copy of the bot to circumvent lifetime issues
        let bot1 = self.clone();
        // create a new task which resolves the bot name and then set it in the struct
        let resolve_name = self.get_me().send().map(move |user| {
            if let Some(new_name) = user.1.username {
                Bot {
                    key: bot1.key,
                    name: Some(format!("@{}", new_name)),
                    last_id: bot1.last_id,
                    update_interval: bot1.update_interval,
                    timeout: bot1.timeout,
                    handlers: bot1.handlers,
                    unknown_handler: bot1.unknown_handler,
                }
            } else {
                bot1
            }
        });
        // spawn the task
        let fut = resolve_name.and_then(|bot| bot.get_stream().for_each(|_| Ok(())));

        tokio::run(fut.map(|_| ()).map_err(|_| ()));
    }
}

/// Calls the Telegram API for the function and awaits the result. The result is then converted
/// to a String and returned in a Future.
pub fn _fetch(fut_res: ResponseFuture) -> impl Future<Item = String, Error = Error> {
    fut_res
        .and_then(move |res| res.into_body().concat2())
        .map_err(|e| Error::from(e.context(ErrorKind::Hyper)))
        .and_then(move |response_chunks| {
            let s = str::from_utf8(&response_chunks)?;

            debug!("Got a result from telegram: {}", s);
            // try to parse the result as a JSON and find the OK field.
            // If the ok field is true, then the string in "result" will be returned
            let req = serde_json::from_str::<Value>(&s).context(ErrorKind::JsonParse)?;

            let ok = req
                .get("ok")
                .and_then(Value::as_bool)
                .ok_or(ErrorKind::Json)?;

            if ok {
                if let Some(result) = req.get("result") {
                    return Ok(serde_json::to_string(result).context(ErrorKind::JsonSerialize)?);
                }
            }

            let e = match req.get("description").and_then(Value::as_str) {
                Some(err) => {
                    Error::from(TelegramError::new(err.into()).context(ErrorKind::Telegram))
                }
                None => Error::from(ErrorKind::Telegram),
            };

            Err(Error::from(e.context(ErrorKind::Telegram)))
        })
}
