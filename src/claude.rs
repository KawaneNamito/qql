use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::json;

use crate::config::ResolvedProviderConfig;
use crate::provider::Provider;

pub struct ClaudeProvider {
    config: ResolvedProviderConfig,
}

impl ClaudeProvider {
    pub fn new(config: ResolvedProviderConfig) -> Self {
        Self { config }
    }
}

impl Provider for ClaudeProvider {
    fn ask(&self, question: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Response {
            content: Vec<ContentBlock>,
        }

        #[derive(Deserialize)]
        struct ContentBlock {
            text: Option<String>,
        }

        let response: Response = ureq::post("https://api.anthropic.com/v1/messages")
            .set("x-api-key", &self.config.api_key)
            .set("anthropic-version", "2023-06-01")
            .send_json(json!({
                "model": self.config.model,
                "max_tokens": 1024,
                "messages": [
                    {
                        "role": "user",
                        "content": question,
                    }
                ]
            }))
            .map_err(http_error)?
            .into_json()
            .context("failed to decode Claude response")?;

        response
            .content
            .into_iter()
            .find_map(|block| block.text)
            .ok_or_else(|| anyhow!("Claude response did not include text content"))
    }
}

fn http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => anyhow!(
            "Claude request failed with status {}: {}",
            code,
            response
                .into_string()
                .unwrap_or_else(|_| "failed to read response body".to_owned())
        ),
        ureq::Error::Transport(error) => anyhow!("Claude transport error: {error}"),
    }
}
