mod serde_structs;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{tls, StatusCode};
use serde::{Deserialize, Serialize};
use serde_structs::*;
use std::fs::File;
use std::io::{BufWriter, Read, Seek};

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CreditBudget {
    pub tokens_left: i64,
}

struct TokenDispenser {
    file_writer: BufWriter<File>,
    tokens_left: i64,
}

impl TokenDispenser {
    pub fn is_deductible(&self, credits_needed: u64) -> bool {
        self.tokens_left >= credits_needed as i64
    }

    pub fn subtract_credits(&mut self, credits_needed: u64) {
        if self.tokens_left <= 0 {
            panic!("No more credits are available");
        }

        self.tokens_left -= credits_needed as i64;

        self.file_writer.rewind().unwrap();
        serde_json::to_writer(
            &mut self.file_writer,
            &CreditBudget {
                tokens_left: self.tokens_left,
            },
        )
        .unwrap();
    }
}

pub struct OpenaiTurbo {
    client: reqwest::Client,
    token_dispenser: TokenDispenser,
}

impl OpenaiTurbo {
    pub fn new() -> Self {
        let token_dispenser = match File::options()
            .write(true)
            .read(true)
            .open("../../credits_budget.json")
        {
            Ok(mut file) => {
                let mut string = String::new();
                file.read_to_string(&mut string).unwrap();

                let json: CreditBudget = serde_json::from_str(&string).unwrap();

                TokenDispenser {
                    file_writer: BufWriter::new(file),
                    tokens_left: json.tokens_left,
                }
            }
            Err(_) => {
                let file = File::create("../../credits_budget.json").unwrap();

                // This is 5$ worth of credits
                let initial_credits = 2500000;

                serde_json::to_writer(
                    &file,
                    &CreditBudget {
                        tokens_left: initial_credits,
                    },
                )
                .unwrap();

                TokenDispenser {
                    file_writer: BufWriter::new(file),
                    tokens_left: initial_credits,
                }
            }
        };

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
                .min_tls_version(tls::Version::TLS_1_2)
                .default_headers(default_headers)
                .build()
                .unwrap(),
            token_dispenser,
        }
    }

    pub async fn chat(&mut self, initial_prompt: &str, conversation: &[String]) -> Option<String> {
        let messages: Vec<Message> = std::iter::once(Message {
            role: "system".to_owned(),
            content: initial_prompt.to_string(),
        })
        .chain(conversation.iter().enumerate().map(|(i, prompt)| Message {
            role: if i % 2 == 0 { "user" } else { "system" }.to_owned(),
            content: prompt.clone(),
        }))
        .collect();

        let max_response_token_length = 60;
        let approximate_token_cost: u64 = messages.iter().fold(0, |acc: u64, message: &Message| {
            acc + (message.content.len() as u64 / 4u64)
        }) + max_response_token_length;

        if !self.token_dispenser.is_deductible(approximate_token_cost) {
            return None;
        }

        let json = ChatCompetitionRequest {
            model: "gpt-3.5-turbo".to_owned(),
            messages,
            temperature: 0.8,
            max_tokens: max_response_token_length as u32,
        };

        let res = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .json(&json)
            .send()
            .await
            .ok()?;
        match res.status() {
            StatusCode::OK => match res.json::<ChatCompetitionResponse>().await {
                Ok(parsed) => {
                    self.token_dispenser
                        .subtract_credits(parsed.usage.total_tokens as u64);
                    dbg!(self.token_dispenser.tokens_left);
                    let text = parsed.choices[0].message.content.clone();
                    Some(text)
                }
                Err(_) => None,
            },
            _ => None,
        }
    }

    pub async fn is_inappropriate(&self, sentence: &str) -> Result<Categories, reqwest::Error> {
        let json = ModerationRequest {
            input: sentence.to_owned(),
        };

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
