//! Content filtering module using Ollama for offensive content detection

use ollama_rs::Ollama;
use ollama_rs::generation::completion::GenerationResponse;
use ollama_rs::generation::completion::request::GenerationRequest;
use ollama_rs::models::ModelOptions;
use std::error::Error;
use std::sync::OnceLock;
use tracing::{debug, info};
use url::Url;

/// Default Ollama host
const DEFAULT_OLLAMA_HOST: &str = "localhost";

/// Default Ollama port
const DEFAULT_OLLAMA_PORT: u16 = 11434;

/// Default model for content filtering (small, fast, good for classification)
const DEFAULT_FILTER_MODEL: &str = "llama3.2:3b";

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
    pub fn new() -> Result<Self, Box<dyn Error>> {
        info!("Initializing content filter with default settings");
        Self::with_endpoint_and_model(
            DEFAULT_OLLAMA_HOST.to_string(),
            DEFAULT_OLLAMA_PORT,
            DEFAULT_FILTER_MODEL.to_string(),
        )
    }

    /// Create a new content filter with custom endpoint and model
    pub fn with_endpoint_and_model(
        host: String,
        port: u16,
        model: String,
    ) -> Result<Self, Box<dyn Error>> {
        let url = Url::parse(&format!("http://{}:{}", host, port))?;
        debug!("Creating content filter: endpoint={}, model={}", url, model);
        Ok(Self {
            ollama: Ollama::try_new(url)?,
            model,
        })
    }

    /// Create a new content filter from a full URL
    pub fn from_url(url: String, model: String) -> Result<Self, Box<dyn Error>> {
        let parsed_url = Url::parse(&url)?;
        let host = parsed_url
            .host_str()
            .ok_or("Invalid URL: no host")?
            .to_string();
        let port = parsed_url
            .port_or_known_default()
            .unwrap_or(DEFAULT_OLLAMA_PORT);

        Self::with_endpoint_and_model(host, port, model)
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

/// Global content filter instance (lazy initialization)
pub fn get_filter() -> &'static ContentFilter {
    static FILTER: OnceLock<ContentFilter> = OnceLock::new();
    FILTER.get_or_init(|| ContentFilter::new().expect("Failed to create content filter"))
}

/// Initialize a content filter with custom settings
///
/// Accepts optional host, port, and model. If not provided, uses defaults.
/// Returns the initialized ContentFilter or an error.
pub fn init_filter(
    endpoint: Option<String>,
    model: Option<String>,
) -> Result<ContentFilter, Box<dyn Error>> {
    match endpoint {
        Some(url) => {
            // Try to parse as a full URL first
            if let Ok(parsed) = Url::parse(&url) {
                let host = parsed.host_str().ok_or("Invalid URL: no host")?.to_string();
                let port = parsed
                    .port_or_known_default()
                    .unwrap_or(DEFAULT_OLLAMA_PORT);
                let model = model.unwrap_or_else(|| DEFAULT_FILTER_MODEL.to_string());
                debug!("Initializing filter with URL: {} (model: {})", url, model);
                ContentFilter::with_endpoint_and_model(host, port, model)
            } else {
                // Fall back to treating as host with default port
                let host = url;
                let port = DEFAULT_OLLAMA_PORT;
                let model = model.unwrap_or_else(|| DEFAULT_FILTER_MODEL.to_string());
                debug!(
                    "Initializing filter with host: {}:{}/ (model: {})",
                    host, port, model
                );
                ContentFilter::with_endpoint_and_model(host, port, model)
            }
        }
        None => {
            let model = model.unwrap_or_else(|| DEFAULT_FILTER_MODEL.to_string());
            info!(
                "Initializing filter with default settings (model: {})",
                model
            );
            ContentFilter::with_endpoint_and_model(
                DEFAULT_OLLAMA_HOST.to_string(),
                DEFAULT_OLLAMA_PORT,
                model,
            )
        }
    }
}

/// Parse an endpoint string (URL, host:port, or just host) and return (host, port)
pub fn parse_endpoint(endpoint: &str) -> Result<(String, u16), Box<dyn Error>> {
    // Try to parse as a full URL first
    if let Ok(parsed) = Url::parse(endpoint) {
        let host = parsed.host_str().ok_or("Invalid URL: no host")?.to_string();
        let port = parsed
            .port_or_known_default()
            .unwrap_or(DEFAULT_OLLAMA_PORT);
        return Ok((host, port));
    }

    // Try to parse as host:port (without protocol)
    if let Some((host, port_str)) = endpoint.rsplit_once(':')
        && let Ok(port) = port_str.parse::<u16>()
    {
        return Ok((host.to_string(), port));
    }

    // Assume it's just a host, use default port
    Ok((endpoint.to_string(), DEFAULT_OLLAMA_PORT))
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
        let filter = ContentFilter::new().unwrap();
        assert_eq!(filter.endpoint(), "http://localhost:11434/");
        assert_eq!(filter.model(), DEFAULT_FILTER_MODEL);
    }

    #[test]
    fn test_custom_filter() {
        let filter = ContentFilter::with_endpoint_and_model(
            "custom".to_string(),
            11434,
            "custom-model".to_string(),
        )
        .unwrap();
        assert_eq!(filter.endpoint(), "http://custom:11434/");
        assert_eq!(filter.model(), "custom-model");
    }

    #[test]
    fn test_prompt_building() {
        let filter = ContentFilter::new().unwrap();
        let text = "Test news headline";
        let prompt = filter.build_classification_prompt(text);

        assert!(prompt.contains("Test news headline"));
        assert!(prompt.contains("OFFENSIVE or SAFE"));
    }

    #[test]
    fn test_parse_endpoint() {
        // Test full URL
        let (host, port) = parse_endpoint("http://localhost:8080").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, 8080);

        // Test host only
        let (host, port) = parse_endpoint("localhost").unwrap();
        assert_eq!(host, "localhost");
        assert_eq!(port, DEFAULT_OLLAMA_PORT);

        // Test host with port
        let (host, port) = parse_endpoint("192.168.1.1:9000").unwrap();
        assert_eq!(host, "192.168.1.1");
        assert_eq!(port, 9000);
    }
}
