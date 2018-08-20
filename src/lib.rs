//! # Write a telegram bot in Rust
//!
//! This library allows you to write a Telegram Bot in Rust. It's an almost complete wrapper for the Telegram Bot API and uses hyper to send a request to the Telegram server. Each Telegram function call returns a future and carries the actual bot and the answer.
//! You can find all available functions in src/functions.rs. The crate telebot-derive implements all
//! required getter, setter and send functions automatically.
//!
//! # Example usage
//!
//! ```
//! extern crate telebot;
//! extern crate futures;
//! extern crate tokio;
//!
//! use futures::Future;
//! use telebot::{functions::*, Bot};
//! use tokio::runtime::Runtime;
//!
//! fn main() {
//!     // init the bot with the bot key and an update interval of 200ms
//!     let bot = Bot::builder("<TELEGRAM-BOT-TOKEN>")
//!         .update_interval(200)
//!         // register a new command "reply" which replies all received messages
//!         .new_cmd("/reply", |(bot, msg)| {
//!             let mut text = msg.text.unwrap().clone();
//!
//!             // when the text is empty send a dummy text
//!             if text.is_empty() {
//!                 text = "<empty>".into();
//!             }
//!
//!             // construct a message and return a new future which will be resolved by tokio
//!             Box::new(
//!                 bot
//!                     .message(msg.chat.id, text)
//!                     .send()
//!                     .map(|_| ())
//!                     .map_err(|_| ())
//!             )
//!         })
//!         // start the event loop
//!         .run();
//! }
//! ```

#![feature(custom_attribute)]
#![feature(try_from)]
#![feature(integer_atomics)]
#![feature(atomic_min_max)]
#![allow(unused_attributes)]

#[macro_use]
extern crate telebot_derive;

#[macro_use]
extern crate log;

#[macro_use]
extern crate serde_derive;

extern crate erased_serde;
extern crate futures;
extern crate hyper;
extern crate hyper_multipart_rfc7578 as hyper_multipart;
extern crate hyper_tls;
extern crate native_tls;
extern crate serde;
extern crate serde_json;
extern crate tokio;
extern crate uuid;

#[macro_use]
extern crate failure;

pub use bot::Bot;
//pub use error::Error;
pub use file::File;

pub mod bot;
pub mod error;
pub mod file;
pub mod functions;
pub mod objects;
