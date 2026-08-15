//! Content filtering module using Ollama for offensive content detection

use ollama_rs::Ollama;
use ollama_rs::generation::completion::GenerationResponse;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::models::ModelOptions;
use std::error::Error;
use tracing::{debug, info};

/// Classification result
#[derive(Debug, Clone, PartialEq)]
pub enum ContentClassification {
    Safe,
    Offensive,
}

impl std::fmt::Display for ContentClassification {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContentClassification::Safe => write!(f, "SAFE"),
            ContentClassification::Offensive => write!(f, "OFFENSIVE"),
        }
    }
}

/// Content filter using Ollama for LLM-based classification
#[derive(Debug, Clone)]
pub struct ContentFilter {
    ollama: Ollama,
    model: String,
}

impl ContentFilter {
    /// Create a new content filter with default settings
    pub fn new(model: String) -> Result<Self, Box<dyn Error>> {
        info!("Initializing content filter with default settings");

        Ok(Self {
            ollama: Ollama::try_new("http://localhost:11434")?,
            model,
        })
    }

    /// Check if Ollama is running and accessible
    pub async fn check_health(&self) -> Result<bool, Box<dyn Error>> {
        debug!("Checking Ollama health at {}", self.endpoint());
        // Try to ping the Ollama tags endpoint
        // Note: The underlying Ollama client uses reqwest with a default timeout
        // For slow models or large downloads, you may need to increase the timeout
        // by rebuilding with a custom reqwest client (see reqwest 0.12 docs for timeout config)
        let client = reqwest::Client::new();
        let url = format!(
            "{}://{}:{}/api/tags",
            self.ollama.url().scheme(),
            self.ollama.url().host_str().unwrap_or("localhost"),
            self.ollama.url().port_or_known_default().unwrap_or(11434)
        );

        let response = client
            .get(&url)
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

    /// Classify text as safe or offensive using the LLM
    pub async fn classify(&self, text: &str) -> Result<ContentClassification, Box<dyn Error>> {
        debug!("Classifying text: {}", text);

        let prompt = self.build_classification_prompt(text);
        let request = GenerationRequest::new(self.model.clone(), prompt).options(
            ModelOptions::default()
                .temperature(0.0) // Deterministic for classification
                .num_predict(32), // Enough for "SAFE" or "OFFENSIVE"
        );

        let response: GenerationResponse = self.ollama.generate(request).await?;
        debug!(
            "LLM model: {}, response: '{}', done: {}",
            response.model, response.response, response.done
        );
        let output = response.response.to_uppercase();

        if output.contains("OFFENSIVE") {
            Ok(ContentClassification::Offensive)
        } else {
            Ok(ContentClassification::Safe)
        }
    }

    /// Check if text is offensive (convenience method)
    pub async fn is_offensive(&self, text: &str) -> Result<bool, Box<dyn Error>> {
        let result = self.classify(text).await?;
        debug!("Classification result for '{}': {}", text, result);
        Ok(result == ContentClassification::Offensive)
    }

    /// Build the classification prompt
    fn build_classification_prompt(&self, text: &str) -> String {
        format!(
            "You are a content filter for a news ticker application. \
             The text may be in any language including Dutch, English, German, French, Spanish, or other languages. \
             Classify the following text as OFFENSIVE or SAFE. \
             Mark as OFFENSIVE if the text contains: death, killing, murder, violence, accident, disaster, catastrophe, tragedy, trauma, or any graphic/horrific content. \
             This includes news about fatalities, violent crimes, accidents, natural disasters, and other traumatic events. \
             Text about politics, economics, sports, culture, or non-traumatic news should be marked SAFE. \
             Respond with ONLY the word OFFENSIVE or SAFE, with no other text or explanations.\n\nText: {}",
            text
        )
    }

    /// Get the current model name
    pub fn model(&self) -> &str {
        &self.model
    }

    /// Get the Ollama URL
    pub fn endpoint(&self) -> String {
        self.ollama.url().to_string()
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
        assert_eq!(filter.endpoint(), "http://custom:11434/");
        assert_eq!(filter.model(), "custom-model");
    }

    #[test]
    fn test_prompt_building() {
        let filter = ContentFilter::new("something".to_string()).unwrap();
        let text = "Test news headline";
        let prompt = filter.build_classification_prompt(text);

        assert!(prompt.contains("Test news headline"));
        assert!(prompt.contains("OFFENSIVE or SAFE"));
    }
}
