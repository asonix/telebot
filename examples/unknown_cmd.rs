extern crate futures;
extern crate telebot;

use std::env;

use futures::Future;
// import all available functions
use telebot::{functions::*, Bot};

fn main() {
    // Create the bot
    let mut bot = Bot::builder(&env::var("TELEGRAM_BOT_KEY").unwrap());

    bot
        .update_interval(200)
        // Every possible command is unknown
        .unknown_cmd(|(bot, msg)| {
            let fut = bot
                .message(msg.chat.id, "Unknown command".into())
                .send()
                .map(|_| ())
                .map_err(|_| ());

            Box::new(fut)
        });

    // Enter the main loop
    bot.run();
}
