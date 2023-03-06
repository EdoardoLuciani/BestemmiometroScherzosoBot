use dotenv;
use std::sync::RwLock;
use std::sync::Arc;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    types::{InlineKeyboardButton, InlineKeyboardMarkup},
    utils::command::BotCommands,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default)]
pub enum State {
    #[default]
    Start,
    CurrentlyAnswering
}

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "This is a bot that tries to be helpful and active during conversations:")]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "start the conversation manually")]
    Start,
    #[command(description = "stop the conversation manually")]
    Stop
}


#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();

    let chat_history: RwLock<Vec<String>> = RwLock::new(Vec::new());

    Dispatcher::builder(bot, schema())
    .dependencies(dptree::deps![InMemStorage::<State>::new()])
    .enable_ctrlc_handler()
    .build()
    .dispatch()
    .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Stop].endpoint(stop));

    let message_handler = Update::filter_message()
        .branch(command_handler)
        .branch(case![State::Start].endpoint(chatbot_answer))
        .branch(case![State::CurrentlyAnswering].endpoint(chatbot_answer));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .branch(message_handler)
        //.branch(callback_query_handler)
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "At your service master!").await?;
    dialogue.update(State::CurrentlyAnswering).await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string()).await?;
    Ok(())
}

async fn stop(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "Ok I will shut up").await?;
    dialogue.exit().await?;
    Ok(())
}

async fn chatbot_answer(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    match dialogue.get().await? {
        Some(State::Start) => {
            bot.send_message(msg.chat.id, "conversation just started").await?;
            dialogue.update(State::CurrentlyAnswering).await?;
        }
        Some(State::CurrentlyAnswering) => {
            bot.send_message(msg.chat.id, "currently answering").await?;
        }
        _ => {}
    }
    Ok(())
}