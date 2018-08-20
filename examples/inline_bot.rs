#![recursion_limit = "128"]

extern crate erased_serde;
extern crate futures;
extern crate telebot;
extern crate tokio;

use erased_serde::Serialize;
use futures::{Future, Stream};
use std::env;
use telebot::{functions::*, objects::*, Bot};

fn main() {
    // Create the bot
    let fut = Bot::builder(&env::var("TELEGRAM_BOT_KEY").unwrap())
        .update_interval(200)
        .build()
        .and_then(|bot| {
            bot.get_stream()
                .filter_map(|(bot, msg)| msg.inline_query.map(|query| (bot, query)))
                .for_each(|(bot, query)| {
                    let result: Vec<Box<Serialize + Send>> = vec![Box::new(
                        InlineQueryResultArticle::new(
                            "Test".into(),
                            Box::new(input_message_content::Text::new("This is a test".into())),
                        ).reply_markup(InlineKeyboardMarkup::new(vec![
                            vec![
                                InlineKeyboardButton::new("Wikipedia".into())
                                    .url("http://wikipedia.org"),
                            ],
                        ])),
                    )];

                    bot.answer_inline_query(query.id, result)
                        .is_personal(true)
                        .send()
                        .map(|_| ())
                })
        });

    // enter the main loop
    tokio::run(fut.map_err(|_| ()));
}
