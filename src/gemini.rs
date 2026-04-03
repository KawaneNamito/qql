use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use serde_json::json;

use crate::config::ResolvedProviderConfig;
use crate::provider::Provider;

pub struct GeminiProvider {
    config: ResolvedProviderConfig,
}

impl GeminiProvider {
    pub fn new(config: ResolvedProviderConfig) -> Self {
        Self { config }
    }
}

impl Provider for GeminiProvider {
    fn ask(&self, question: &str) -> Result<String> {
        #[derive(Deserialize)]
        struct Response {
            candidates: Vec<Candidate>,
        }

        #[derive(Deserialize)]
        struct Candidate {
            content: Content,
        }

        #[derive(Deserialize)]
        struct Content {
            parts: Vec<Part>,
        }

        #[derive(Deserialize)]
        struct Part {
            text: Option<String>,
        }

        let url = format!(
            "https://generativelanguage.googleapis.com/v1beta/models/{}:generateContent?key={}",
            self.config.model, self.config.api_key
        );
        let response: Response = ureq::post(&url)
            .send_json(json!({
                "contents": [
                    {
                        "parts": [
                            {
                                "text": question,
                            }
                        ]
                    }
                ]
            }))
            .map_err(http_error)?
            .into_json()
            .context("failed to decode Gemini response")?;

        let parts = response
            .candidates
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("Gemini response did not include candidates"))?
            .content
            .parts;
        let joined = parts
            .into_iter()
            .filter_map(|part| part.text)
            .collect::<Vec<_>>()
            .join("\n");

        if joined.is_empty() {
            return Err(anyhow!("Gemini response did not include text content"));
        }

        Ok(joined)
    }
}

fn http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => anyhow!(
            "Gemini request failed with status {}: {}",
            code,
            response
                .into_string()
                .unwrap_or_else(|_| "failed to read response body".to_owned())
        ),
        ureq::Error::Transport(error) => anyhow!("Gemini transport error: {error}"),
    }
}
