mod serde_structs;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::{tls, StatusCode};
use serde_structs::*;
use teloxide::types::Me;

pub struct OpenaiTurbo {
    client: reqwest::Client,
}

impl OpenaiTurbo {
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
                .min_tls_version(tls::Version::TLS_1_2)
                .default_headers(default_headers)
                .build()
                .unwrap(),
        }
    }

    pub async fn chat(&self, initial_prompt: &str, conversation: &[String]) -> Option<String> {
        let messages: Vec<Message> = std::iter::once(Message {
            role: "system".to_string(),
            content: initial_prompt.to_string(),
        })
        .chain(conversation.iter().enumerate().map(|(i, prompt)| Message {
            role: if i % 2 == 0 { "user" } else { "system" }.to_string(),
            content: prompt.clone(),
        }))
        .collect();

        let json = ChatCompetitionRequest {
            model: "gpt-3.5-turbo".to_string(),
            messages,
            temperature: 0.8,
            max_tokens: 60,
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
                    let text = parsed.choices[0].message.content.clone();
                    //text.tr
                    Some(text)
                }
                Err(_) => None,
            },
            _ => None,
        }
    }

    pub async fn is_unappropriate(&self, sentence: &str) -> Option<String> {
        let json = ModerationRequest {
            input: sentence.to_string(),
        };

        let res = self
            .client
            .post("https://api.openai.com/v1/moderations")
            .json(&json)
            .send()
            .await
            .unwrap();
        match res.status() {
            StatusCode::OK => match res.json::<ModerationResponse>().await {
                Ok(parsed) => {
                    let categories = &parsed.results[0].categories;
                    categories.is_flagged().then_some(categories.to_string())
                }
                Err(_) => None,
            },
            _ => None,
        }
    }
}
