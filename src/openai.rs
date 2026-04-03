use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::json;

use crate::config::ResolvedProviderConfig;
use crate::provider::Provider;

pub struct OpenAiProvider {
    config: ResolvedProviderConfig,
}

impl OpenAiProvider {
    pub fn new(config: ResolvedProviderConfig) -> Self {
        Self { config }
    }
}

impl Provider for OpenAiProvider {
    fn ask(&self, question: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Response {
            choices: Vec<Choice>,
        }

        #[derive(Deserialize)]
        struct Choice {
            message: Message,
        }

        #[derive(Deserialize)]
        struct Message {
            content: String,
        }

        let response: Response = ureq::post("https://api.openai.com/v1/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.config.api_key))
            .send_json(json!({
                "model": self.config.model,
                "messages": [
                    {
                        "role": "user",
                        "content": question,
                    }
                ]
            }))
            .map_err(http_error)?
            .into_json()
            .context("failed to decode OpenAI response")?;

        response
            .choices
            .into_iter()
            .next()
            .map(|choice| choice.message.content)
            .ok_or_else(|| anyhow!("OpenAI response did not include a message"))
    }
}

fn http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => anyhow!(
            "OpenAI request failed with status {}: {}",
            code,
            response
                .into_string()
                .unwrap_or_else(|_| "failed to read response body".to_owned())
        ),
        ureq::Error::Transport(error) => anyhow!("OpenAI transport error: {error}"),
    }
}
