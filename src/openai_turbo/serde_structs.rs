use serde::{Deserialize, Serialize};
use std::fmt::Display;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompetitionRequest {
    pub model: String,
    pub messages: Vec<Message>,
    pub temperature: f32,
    pub max_tokens: u32,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

// -------------------------------------

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ChatCompetitionResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub choices: Vec<Choice>,
    pub usage: Usage,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Choice {
    pub index: u32,
    pub message: Message,
    pub finish_reason: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

// -------------------------------------

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationRequest {
    pub input: String,
}

// -------------------------------------

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ModerationResponse {
    pub id: String,
    pub model: String,
    pub results: Vec<Result>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Result {
    pub categories: Categories,
    pub category_scores: CategoryScores,
    pub flagged: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Categories {
    pub hate: bool,
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: bool,
    #[serde(rename = "self-harm")]
    pub self_harm: bool,
    pub sexual: bool,
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: bool,
    pub violence: bool,
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: bool,
}

impl Categories {
    pub fn is_flagged(&self) -> bool {
        self.hate
            || self.hate_threatening
            || self.self_harm
            || self.sexual
            || self.sexual_minors
            || self.violence
            || self.violence_graphic
    }
}

impl Display for Categories {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut s = String::from("The thing you said is");
        if self.hate {
            s.push_str(" hateful,");
        }
        if self.hate_threatening {
            s.push_str(" threatening,");
        }
        if self.self_harm {
            s.push_str(" suicidal,");
        }
        if self.sexual {
            s.push_str(" sexual,");
        }
        if self.sexual_minors {
            s.push_str(" involving minors,");
        }
        if self.violence || self.violence_graphic {
            s.push_str(" violent,");
        }
        s.push_str(".Jesus is not happy with you");
        write!(f, "{}", s)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CategoryScores {
    pub hate: f64,
    #[serde(rename = "hate/threatening")]
    pub hate_threatening: f64,
    #[serde(rename = "self-harm")]
    pub self_harm: f64,
    pub sexual: f64,
    #[serde(rename = "sexual/minors")]
    pub sexual_minors: f64,
    pub violence: f64,
    #[serde(rename = "violence/graphic")]
    pub violence_graphic: f64,
}
