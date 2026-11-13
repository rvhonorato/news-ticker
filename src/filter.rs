//! Content filtering module using an Ollama decision model for offensive content detection

use feed_rs::model::Entry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::error::Error;
use tracing::{debug, info};
use url::Url;

/// Titles with P(triggering) at or above this value are marked offensive
const TRIGGER_THRESHOLD: f64 = 0.5;

/// Name of the question in the decision request, used to find the answer
const QUESTION_KEY: &str = "triggering";

/// Classification result
#[derive(Debug, Clone, PartialEq)]
pub enum ContentClassification {
    Safe,
    Offensive,
    Unknown,
}

impl std::fmt::Display for ContentClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentClassification::Safe => write!(f, "SAFE"),
            ContentClassification::Offensive => write!(f, "OFFENSIVE"),
            ContentClassification::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Request body for `POST /v1/systemone`
#[derive(Debug, Serialize)]
struct DecisionRequest {
    model: String,
    state: String,
    questions: HashMap<&'static str, Question>,
}

#[derive(Debug, Serialize)]
struct Question {
    #[serde(rename = "type")]
    kind: &'static str,
    instructions: &'static str,
    criteria: HashMap<&'static str, &'static str>,
}

#[derive(Debug, Deserialize)]
struct DecisionResponse {
    answers: HashMap<String, Answer>,
}

#[derive(Debug, Deserialize)]
struct Answer {
    /// Probability that the statement is true
    noul: f64,
}

/// Content filter using an Ollama decision model (for example tev1)
#[derive(Debug, Clone)]
pub struct ContentFilter {
    client: reqwest::Client,
    base_url: Url,
    model: String,
}

impl ContentFilter {
    /// Create a new content filter with default settings
    pub fn new(model: String) -> Result<Self, Box<dyn Error>> {
        info!("Initializing content filter with default settings");

        Ok(Self {
            client: reqwest::Client::new(),
            base_url: Url::parse("http://localhost:11434")?,
            model,
        })
    }

    /// Check if Ollama is running and accessible
    pub async fn check_health(&self) -> Result<bool, Box<dyn Error>> {
        debug!("Checking Ollama health at {}", self.endpoint());
        let response = self
            .client
            .get(self.base_url.join("api/tags")?)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await;
        let is_healthy = response.is_ok();
        debug!(
            "Ollama health check: {}",
            if is_healthy { "OK" } else { "FAILED" }
        );
        Ok(is_healthy)
    }

    /// Classify text as safe or offensive using the decision model
    pub async fn classify(&self, entry: &Entry) -> Result<ContentClassification, Box<dyn Error>> {
        let Some(title_text) = &entry.title else {
            return Ok(ContentClassification::Unknown);
        };
        let request = self.build_request(&title_text.content);

        let response: DecisionResponse = self
            .client
            .post(self.base_url.join("v1/systemone")?)
            .json(&request)
            .send()
            .await?
            .error_for_status()?
            .json()
            .await?;

        let p = response
            .answers
            .get(QUESTION_KEY)
            .ok_or("decision response has no 'triggering' answer")?
            .noul;
        debug!("Model: {}, P(triggering)={:.3}", self.model, p);

        Ok(classify_probability(p))
    }

    /// Build the decision request for one title
    fn build_request(&self, text: &str) -> DecisionRequest {
        let question = Question {
            kind: "noul",
            instructions: "Is this news headline about a traumatic or distressing event? \
                 The headline may be in any language, including Dutch, English, German, French or Spanish.",
            criteria: HashMap::from([
                (
                    "true",
                    "The headline is about death, killing, murder, violence, accidents, \
                     natural disasters, catastrophes, tragedies, trauma, or other graphic content.",
                ),
                (
                    "false",
                    "The headline is about politics, economics, sports, culture, or other non-traumatic news.",
                ),
            ]),
        };

        DecisionRequest {
            model: self.model.clone(),
            state: text.to_string(),
            questions: HashMap::from([(QUESTION_KEY, question)]),
        }
    }

    /// Get the current model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the Ollama URL
    pub fn endpoint(&self) -> String {
        self.base_url.to_string()
    }
}

fn classify_probability(p: f64) -> ContentClassification {
    if p >= TRIGGER_THRESHOLD {
        ContentClassification::Offensive
    } else {
        ContentClassification::Safe
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_classification_display() {
        assert_eq!(format!("{}", ContentClassification::Safe), "SAFE");
        assert_eq!(format!("{}", ContentClassification::Offensive), "OFFENSIVE");
    }

    #[test]
    fn test_filter_creation() {
        let filter = ContentFilter::new("model-id".to_string()).unwrap();
        assert_eq!(filter.endpoint(), "http://localhost:11434/");
        assert_eq!(filter.model(), "model-id".to_string());
    }

    #[test]
    fn test_custom_filter() {
        let filter = ContentFilter::new("custom-model".to_string()).unwrap();
        assert_eq!(filter.endpoint(), "http://localhost:11434/");
        assert_eq!(filter.model(), "custom-model");
    }

    #[test]
    fn test_request_building() {
        let filter = ContentFilter::new("tev1:4b".to_string()).unwrap();
        let json = serde_json::to_value(filter.build_request("Test news headline")).unwrap();

        assert_eq!(json["model"], "tev1:4b");
        assert_eq!(json["state"], "Test news headline");
        assert_eq!(json["questions"]["triggering"]["type"], "noul");
        assert!(json["questions"]["triggering"]["criteria"]["true"].is_string());
        assert!(json["questions"]["triggering"]["criteria"]["false"].is_string());
    }

    #[test]
    fn test_response_parsing() {
        let body = r#"{
            "model": "tev1:4b",
            "answers": { "triggering": { "type": "noul", "noul": 0.93 } },
            "usage": { "input_tokens": 41, "output_tokens": 1 }
        }"#;
        let response: DecisionResponse = serde_json::from_str(body).unwrap();
        assert_eq!(response.answers[QUESTION_KEY].noul, 0.93);
    }

    #[test]
    fn test_threshold() {
        assert_eq!(classify_probability(0.93), ContentClassification::Offensive);
        assert_eq!(classify_probability(0.5), ContentClassification::Offensive);
        assert_eq!(classify_probability(0.1), ContentClassification::Safe);
    }
}
