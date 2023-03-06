use dotenv;
use std::sync::{Mutex, Arc};
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default, Debug)]
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

type Conversation = Arc<Mutex<Vec<String>>>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
    .dependencies(dptree::deps![InMemStorage::<State>::new(), Conversation::new(Mutex::new(Vec::new()))])
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
        .filter(|msg: Message| msg.chat.id.0 == -619090504)
        .branch(command_handler)
        .branch(case![State::Start].endpoint(chatbot_answer))
        .branch(case![State::CurrentlyAnswering].endpoint(chatbot_answer));

    dialogue::enter::<Update, InMemStorage<State>, State, _>()
        .chain(message_handler)
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

async fn chatbot_answer(bot: Bot, msg: Message, dialogue: MyDialogue, state: State, conversation: Conversation) -> HandlerResult {
    match state {
        State::Start => {
            bot.send_message(msg.chat.id, "conversation just started").await?;
            dialogue.update(State::CurrentlyAnswering).await?;
        }
        State::CurrentlyAnswering => {
            bot.send_message(msg.chat.id, "currently answering").await?;
        }
    }

    let conversation_arc = conversation.clone();
    let mut conversation = conversation_arc.lock().unwrap();
    conversation.push(msg.text().unwrap().to_string());

    Ok(())
}