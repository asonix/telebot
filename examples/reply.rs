extern crate futures;
extern crate telebot;

use futures::Future;
use std::env;
use telebot::{functions::*, Bot};

fn main() {
    // Create the bot
    let mut bot = Bot::builder(&env::var("TELEGRAM_BOT_KEY").unwrap());

    bot
        .update_interval(200)
        // Register a reply command which answers a message
        .new_cmd("/reply", |(bot, msg)| {
            let mut text = msg.text.unwrap().clone();
            if text.is_empty() {
                text = "<empty>".into();
            }

            let fut = bot
                .message(msg.chat.id, text)
                .send()
                .map(|_| ())
                .map_err(|_| ());

            Box::new(fut)
        });

    // Enter the main loop
    bot.run();
}
