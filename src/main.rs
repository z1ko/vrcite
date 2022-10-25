use teloxide::dispatching::dialogue::InMemStorage;
use teloxide::prelude::*;

mod cmds;
mod types;

use crate::cmds::*;

#[tokio::main]
async fn main() {
    let bot = Bot::from_env();
    teloxide::dispatching::Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![InMemStorage::<State>::new()])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}
