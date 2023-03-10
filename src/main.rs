mod openai_client;

use openai_client::OpenaiClient;
use std::fs::File;
use std::ops::{Deref, DerefMut};

use rand::Rng;
use std::sync::Arc;
use teloxide::{
    dispatching::{dialogue, dialogue::InMemStorage, UpdateHandler},
    prelude::*,
    utils::command::BotCommands,
};
use tokio::sync::RwLock;

type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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

#[derive(Clone, Default, Debug)]
pub enum State {
    #[default]
    Start,
    CurrentlyAnswering {
        conversation: Vec<String>,
    },
}

type DialogueStorage = Dialogue<State, InMemStorage<State>>;

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
        .dependencies(dptree::deps![
            InMemStorage::<State>::new(),
            Arc::new(RwLock::new(OpenaiClient::new()))
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct AllowList {
    pub allowed_ids: Vec<i64>,
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let allowlist = File::open("allowlist.json").expect("Unable to open allowlist.json");
    let allowlist_json: AllowList =
        serde_json::from_reader(allowlist).expect("Cannot parse allowlist.json");

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Stop].endpoint(stop));

    let message_handler = Update::filter_message()
        .filter(move |msg: Message| {
            println!("Received message from chat: {}", msg.chat.id);
            allowlist_json.allowed_ids.contains(&msg.chat.id.0)
        })
        .branch(command_handler)
        .branch(case![State::Start].endpoint(handle_message))
        .branch(case![State::CurrentlyAnswering { conversation }].endpoint(handle_message));

    dialogue::enter::<Update, InMemStorage<State>, State, _>().chain(message_handler)
}

async fn start(bot: Bot, msg: Message, dialogue: DialogueStorage) -> HandlerResult {
    bot.send_message(msg.chat.id, "At your service master!")
        .await?;
    dialogue
        .update(State::CurrentlyAnswering {
            conversation: Vec::new(),
        })
        .await?;
    Ok(())
}

async fn help(bot: Bot, msg: Message) -> HandlerResult {
    bot.send_message(msg.chat.id, Command::descriptions().to_string())
        .await?;
    Ok(())
}

async fn stop(bot: Bot, msg: Message, dialogue: DialogueStorage) -> HandlerResult {
    bot.send_message(msg.chat.id, "Ok I will shut up").await?;
    dialogue.exit().await?;
    Ok(())
}

async fn monitor_and_reply(
    bot: &Bot,
    msg: &Message,
    openai_client: &OpenaiClient,
) -> HandlerResult {
    if let Ok(categories) = openai_client.is_inappropriate(msg.text().unwrap()).await {
        if categories.is_flagged() {
            bot.send_message(msg.chat.id, categories.to_string())
                .reply_to_message_id(msg.id)
                .await?;
        }
    }
    Ok(())
}

async fn send_response(
    bot: &Bot,
    msg: &Message,
    conversation: &mut Vec<String>,
    openai_client: &mut OpenaiClient,
) -> HandlerResult {
    let initial_prompt = "You are a funny friend talking to a bunch of nerds";
    let msg_text = msg.text().unwrap();

    match openai_client
        .chat(initial_prompt, &conversation, &msg_text)
        .await
    {
        Ok(response) => {
            bot.send_message(msg.chat.id, response.to_owned())
                .reply_to_message_id(msg.id)
                .await?;
            conversation.push(msg_text.to_owned());
            conversation.push(response);
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("Sorry, but due to: {:?}, I could not answer", e),
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    dialogue: DialogueStorage,
    openai_client: Arc<RwLock<OpenaiClient>>,
) -> HandlerResult {
    monitor_and_reply(&bot, &msg, openai_client.read().await.deref()).await?;

    match dialogue.get_or_default().await {
        Ok(State::Start) => {
            if rand::thread_rng().gen_range(0..10) == 7 {
                let mut conversation = Vec::new();
                send_response(
                    &bot,
                    &msg,
                    &mut conversation,
                    openai_client.write().await.deref_mut(),
                )
                .await?;
                dialogue
                    .update(State::CurrentlyAnswering { conversation })
                    .await?;
            }
        }
        Ok(State::CurrentlyAnswering { mut conversation }) => {
            if conversation.len() < 10 {
                send_response(
                    &bot,
                    &msg,
                    &mut conversation,
                    openai_client.write().await.deref_mut(),
                )
                .await?;
                dialogue
                    .update(State::CurrentlyAnswering { conversation })
                    .await?;
            } else {
                dialogue.exit().await?;
            }
        }
        _ => {}
    };
    Ok(())
}
