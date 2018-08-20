extern crate futures;
extern crate telebot;
extern crate tokio;

use futures::Future;
use std::env;
use telebot::Bot;

// import all available functions
use telebot::functions::*;

fn main() {
    // Create the bot
    let mut bot = Bot::builder(&env::var("TELEGRAM_BOT_KEY").unwrap());

    bot.update_interval(200)
        .new_cmd("/send_self", |(bot, msg)| {
            let fut = bot
                .document(msg.chat.id)
                .file("examples/send_self.rs")
                .send()
                .map(|_| ())
                .map_err(|err| println!("{:?}", err.as_fail()));

            Box::new(fut)
        });

    // enter the main loop
    bot.run();
}
