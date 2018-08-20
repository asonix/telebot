#![recursion_limit = "128"]

extern crate futures;
extern crate telebot;
extern crate tokio;

use futures::{Future, Stream};
use std::env;
use telebot::Bot;

fn main() {
    // Create the bot
    let fut = Bot::builder(&env::var("TELEGRAM_BOT_KEY").unwrap())
        .update_interval(200)
        .build()
        .and_then(|bot| {
            bot.get_stream().for_each(|(_, msg)| {
                println!("Received: {:#?}", msg);

                Ok(())
            })
        });

    // enter the main loop
    tokio::run(fut.map_err(|_| ()));
}
