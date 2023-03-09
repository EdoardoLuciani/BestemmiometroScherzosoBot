mod openai_client;

use openai_client::OpenaiClient;
use std::fs::File;

use rand::Rng;
use std::sync::Arc;
use teloxide::dptree::deps;
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

#[derive(Default, Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
struct WhiteList {
    pub whitelisted_ids: Vec<i64>,
}

#[tokio::main]
async fn main() {
    dotenv::dotenv().ok();

    let bot = Bot::from_env();

    Dispatcher::builder(bot, schema())
        .dependencies(deps![
            InMemStorage::<State>::new(),
            Arc::new(Mutex::new(Vec::<String>::new())),
            Arc::new(Mutex::new(OpenaiClient::new()))
        ])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

fn schema() -> UpdateHandler<Box<dyn std::error::Error + Send + Sync + 'static>> {
    use dptree::case;

    let whitelist = File::open("whitelist.json").expect("Unable to open whitelist.json");
    let whitelist_json: WhiteList =
        serde_json::from_reader(whitelist).expect("Cannot parse whitelist.json");

    let command_handler = teloxide::filter_command::<Command, _>()
        .branch(case![Command::Start].endpoint(start))
        .branch(case![Command::Help].endpoint(help))
        .branch(case![Command::Stop].endpoint(stop));

    let message_handler = Update::filter_message()
        .filter(move |msg: Message| {
            dbg!(msg.chat.id);
            whitelist_json.whitelisted_ids.contains(&msg.chat.id.0)
        })
        .branch(command_handler)
        .branch(case![State::Start].endpoint(handle_message))
        .branch(case![State::CurrentlyAnswering].endpoint(handle_message));

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
                format!("Sorry, but due: {:?}, I could not answer", e),
            )
            .await?;
        }
    }
    Ok(())
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    dialogue: MyDialogue,
    state: State,
    conversation: Arc<Mutex<Vec<String>>>,
    openai_client: Arc<Mutex<OpenaiClient>>,
) -> HandlerResult {
    let mut openai_client = openai_client.lock().await;

    monitor_and_reply(&bot, &msg, &openai_client).await?;

    let mut conversation = conversation.lock().await;

    bot.send_message(msg.chat.id, format!("{:?}", conversation))
        .await?;

    let should_reply = match state {
        State::Start => {
            let start_replying = rand::thread_rng().gen_range(0..10) == 7;
            if start_replying {
                dialogue.update(State::CurrentlyAnswering).await?;
            }
            start_replying
        }
        State::CurrentlyAnswering => {
            let stop_replying = conversation.len() >= 10;
            if stop_replying {
                conversation.clear();
                dialogue.exit().await?;
            }
            !stop_replying
        }
    };
    if should_reply {
        send_response(&bot, &msg, &mut conversation, &mut openai_client).await?
    }
    Ok(())
}
