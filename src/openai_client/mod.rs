mod http_requests_structs;

use http_requests_structs::*;
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufWriter, Read, Seek};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct TokensLeft {
    pub tokens_left: i64,
}

struct TokenDispenser {
    file_writer: BufWriter<File>,
    tokens_left: i64,
}

impl TokenDispenser {
    pub fn new(file_path: &str, initial_tokens: u64) -> Self {
        let (file, tokens) = match File::options().write(true).read(true).open(file_path) {
            Ok(mut file) => {
                let mut string = String::new();
                file.read_to_string(&mut string)
                    .expect(&format!("Could not read file {}", file_path));

                let json: TokensLeft = serde_json::from_str(&string)
                    .expect(&format!("Could not parse {} json", file_path));
                (file, json.tokens_left)
            }
            Err(_) => {
                let file =
                    File::create(file_path).expect(&format!("Could not create file {}", file_path));

                serde_json::to_writer(
                    &file,
                    &TokensLeft {
                        tokens_left: initial_tokens as i64,
                    },
                )
                .unwrap();
                (file, initial_tokens as i64)
            }
        };

        Self {
            file_writer: BufWriter::new(file),
            tokens_left: tokens,
        }
    }

    pub fn is_deductible(&self, credits_needed: u64) -> bool {
        self.tokens_left >= credits_needed as i64
    }

    pub fn subtract_credits(&mut self, credits_needed: u64) {
        if self.tokens_left <= 0 {
            panic!("No more credits available");
        }

        self.tokens_left -= credits_needed as i64;

        self.file_writer.rewind().unwrap();
        serde_json::to_writer(
            &mut self.file_writer,
            &TokensLeft {
                tokens_left: self.tokens_left,
            },
        )
        .unwrap();
    }
}

#[derive(Debug)]
pub enum ChatError {
    InsufficientCredits,
    RequestFailed,
    ResponseParsingFailed,
}

pub struct OpenaiClient {
    client: reqwest::Client,
    token_dispenser: TokenDispenser,
}

impl OpenaiClient {
    pub fn new() -> Self {
        let mut default_headers = HeaderMap::new();

        let bearer_string = format!("Bearer {}", std::env::var("OPENAI_TOKEN").unwrap());
        default_headers.insert(
            "Authorization",
            HeaderValue::from_str(&bearer_string).unwrap(),
        );

        default_headers.insert(
            "Content-Type",
            HeaderValue::from_str("application/json").unwrap(),
        );

        Self {
            client: reqwest::Client::builder()
                .https_only(true)
                .min_tls_version(reqwest::tls::Version::TLS_1_2)
                .default_headers(default_headers)
                .build()
                .unwrap(),
            token_dispenser: TokenDispenser::new("credits_budget.json", 2500000),
        }
    }

    pub async fn chat(
        &mut self,
        initial_prompt: &str,
        conversation_history: &[String],
        prompt: &str,
    ) -> Result<String, ChatError> {
        let messages_iterator = std::iter::once(initial_prompt)
            .chain(conversation_history.iter().map(|msg| msg.as_str()))
            .chain(std::iter::once(prompt));

        let messages: Vec<MessageRef> = messages_iterator
            .enumerate()
            .map(|(i, message)| {
                let role = match i {
                    0 => "system",
                    n if n % 2 == 1 => "user",
                    _ => "assistant",
                };
                MessageRef {
                    role,
                    content: message,
                }
            })
            .collect::<Vec<MessageRef>>();

        let max_response_token_length = 120;
        let approximate_token_cost: u64 = messages
            .iter()
            .fold(max_response_token_length, |acc: u64, message| {
                acc + (message.content.len() as u64 / 4u64)
            });

        if !self.token_dispenser.is_deductible(approximate_token_cost) {
            return Err(ChatError::InsufficientCredits);
        }

        let json = ChatCompetitionRequest {
            model: "gpt-3.5-turbo".to_owned(),
            messages,
            temperature: 0.8,
            max_tokens: max_response_token_length as u32,
        };

        let response = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&json)
            .send()
            .await
            .map_err(|_| ChatError::RequestFailed)?
            .json::<ChatCompetitionResponse>()
            .await
            .map_err(|_| ChatError::ResponseParsingFailed)?;

        self.token_dispenser
            .subtract_credits(response.usage.total_tokens as u64);
        Ok(response.choices[0].message.content.to_owned())
    }

    pub async fn is_inappropriate(&self, sentence: &str) -> Result<Categories, reqwest::Error> {
        let json = ModerationRequest { input: sentence };

        Ok(self
            .client
            .post("https://api.openai.com/v1/moderations")
            .json(&json)
            .send()
            .await?
            .json::<ModerationResponse>()
            .await?
            .results[0]
            .categories
            .clone())
    }
}
