extern crate futures;
extern crate telebot;

use futures::Future;
use std::env;
use telebot::{functions::*, Bot};

static TEXT: &'static str = r"
Dearest creature in creation,
Study English pronunciation.
I will teach you in my verse
Sounds like corpse, corps, horse, and worse.
I will keep you, Suzy, busy,
Make your head with heat grow dizzy.
Tear in eye, your dress will tear.
So shall I! Oh hear my prayer.

Just compare heart, beard, and heard,
Dies and diet, lord and word,
Sword and sward, retain and Britain.
(Mind the latter, how it's written.)
Now I surely will not plague you
With such words as plaque and ague.
But be careful how you speak:
Say break and steak, but bleak and streak;
Cloven, oven, how and low,
Script, receipt, show, poem, and toe.

Hear me say, devoid of trickery,
Daughter, laughter, and Terpsichore,
Typhoid, measles, topsails, aisles,
Exiles, similes, and reviles;
Scholar, vicar, and cigar,
Solar, mica, war and far;
One, anemone, Balmoral,
Kitchen, lichen, laundry, laurel;
Gertrude, German, wind and mind,
Scene, Melpomene, mankind.

...";

fn main() {
    // Create the bot
    let mut bot = Bot::builder(&env::var("TELEGRAM_BOT_KEY").unwrap());

    bot.update_interval(200)
        .new_cmd("/send", move |(bot, msg)| {
            let fut = bot
                .document(msg.chat.id)
                .file(("poem.txt", TEXT.as_bytes()))
                .caption("The Chaos")
                .send()
                .map(|_| ())
                .map_err(|_| ());

            Box::new(fut)
        });

    // enter the main loop
    bot.run();
}
