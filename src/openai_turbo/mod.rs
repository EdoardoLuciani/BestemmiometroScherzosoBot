mod serde_structs;

use reqwest::header::{HeaderMap, HeaderValue};
use reqwest::tls;

pub struct OpenaiTurbo {
    client: reqwest::Client,
    initial_prompt: String,
}

impl OpenaiTurbo {
    pub fn new(initial_prompt: &str) -> Self {
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
            initial_prompt: initial_prompt.to_string(),
            client: reqwest::Client::builder()
                .https_only(true)
                .min_tls_version(tls::Version::TLS_1_2)
                .default_headers(default_headers)
                .build()
                .unwrap(),
        }
    }

    pub async fn list_models(&self) {
        let res = self
            .client
            .get("https://api.openai.com/v1/models")
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        dbg!(res);
    }

    pub async fn chat(&self) {
        let res = self
            .client
            .post("https://api.openai.com/v1/chat/completions")
            .body()
            .send()
            .await
            .unwrap()
            .text()
            .await
            .unwrap();
        dbg!(res);
    }
}
