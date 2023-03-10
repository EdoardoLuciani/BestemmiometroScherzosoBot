BestemmiometroScherzosoBot (name is in italian, translated is: PlayfulBlasphemyMeterBot) is a Telegram bot written in Rust
with the following features:
- It detects when a message contains sexual, hateful and violent words and warns the user
- It engages in a conversation with the user on a 1/10 chance per message or when the user
  sends the command /start

### How to run
The bot requires a Telegram bot token and an OpenAI API key. After cloning the repository, create a .env file in the main
directory with the variables ```TELOXIDE_TOKEN``` and ```OPENAI_TOKEN``` set with the corresponding tokens.
Then, run the bot with ```cargo run``` and you're good to go!
Depending on cargo asking for it, you might need to install the openssl development libraries on your system.