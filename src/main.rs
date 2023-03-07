mod openai_turbo;
use openai_turbo::OpenaiTurbo;

use dotenv;
use rand::Rng;
use std::fmt::format;
use std::sync::Arc;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use tokio::sync::Mutex;

type MyDialogue = Dialogue<State, InMemStorage<State>>;
type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    CurrentlyAnswering,
}

#[derive(BotCommands, Clone)]
#[command(
    rename_rule = "lowercase",
    description = "This is a bot that tries to be helpful and active during conversations"
)]
enum Command {
    #[command(description = "display this text")]
    Help,
    #[command(description = "start the conversation manually")]
    Start,
    #[command(description = "stop the conversation manually")]
    Stop,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    pretty_env_logger::init();
    log::info!("Starting throw dice bot...");

    let bot = Bot::from_env();

    let mut dependancy_map = dptree::di::DependencyMap::new();
    dependancy_map.insert(InMemStorage::<State>::new());
    dependancy_map.insert(Arc::new(Mutex::new(Vec::<String>::new())));
    dependancy_map.insert(Arc::new(Mutex::new(OpenaiTurbo::new())));

    Dispatcher::builder(bot, schema())
        .dependencies(dependancy_map)
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

    dialogue::enter::<Update, InMemStorage<State>, State, _>().chain(message_handler)
}

async fn start(bot: Bot, dialogue: MyDialogue, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, "At your service master!")
        .await?;
    dialogue.update(State::CurrentlyAnswering).await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn stop(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    conversation: Arc<Mutex<Vec<String>>>,
) -> HandlerResult {
    bot.send_message(msg.chat.id, "Ok I will shut up").await?;
    conversation.lock().await.clear();
    dialogue.exit().await?;
    Ok(())
}

async fn chatbot_answer(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    state: State,
    conversation: Arc<Mutex<Vec<String>>>,
    openai_turbo: Arc<Mutex<OpenaiTurbo>>,
) -> HandlerResult {
    let msg_text = msg.text().unwrap();

    let openai_turbo = openai_turbo.lock().await;
    if let Some(response) = openai_turbo.is_unappropriate(msg_text).await {
        bot.send_message(msg.chat.id, response)
            .reply_to_message_id(msg.id)
            .await?;
    }

    let mut conversation = conversation.lock().await;

    match state {
        State::Start => {
            if rand::thread_rng().gen_range(0..10) == 7 {
                dialogue.update(State::CurrentlyAnswering).await?;
            } else {
                return Ok(());
            }
        }
        State::CurrentlyAnswering => {
            if conversation.len() == 10 {
                conversation.clear();
                dialogue.exit().await?;
                return Ok(());
            }
        }
    }

    conversation.push(msg_text.to_string());

    if let Some(response) = openai_turbo
        .chat(
            "You are a funny friend talking to a bunch of nerds",
            &conversation,
        )
        .await
    {
        conversation.push(response.to_string());

        bot.send_message(msg.chat.id, response)
            .reply_to_message_id(msg.id)
            .await?;
    } else {
        conversation.pop();
    }

    Ok(())
}
