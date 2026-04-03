use std::collections::BTreeMap;
use std::path::Path;

use anyhow::{Context, Result, anyhow};
use dialoguer::{Input, MultiSelect, Select, theme::ColorfulTheme};
use serde::Deserialize;

use crate::config::{Config, ProviderConfig, ProviderKind};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModelSelection {
    Preset(String),
    Custom,
}

pub trait InitUi {
    fn select_providers(&mut self, available: &[ProviderKind]) -> Result<Vec<ProviderKind>>;
    fn input_api_key(&mut self, provider: ProviderKind) -> Result<String>;
    fn select_model(
        &mut self,
        provider: ProviderKind,
        available: &[String],
    ) -> Result<ModelSelection>;
    fn input_custom_model(&mut self, provider: ProviderKind) -> Result<String>;
}

pub trait ModelCatalog {
    fn list_models(&self, provider: ProviderKind, api_key: &str) -> Result<Vec<String>>;
}

pub struct DialoguerInitUi;

impl DialoguerInitUi {
    fn theme() -> ColorfulTheme {
        ColorfulTheme::default()
    }
}

impl Default for DialoguerInitUi {
    fn default() -> Self {
        Self
    }
}

impl InitUi for DialoguerInitUi {
    fn select_providers(&mut self, available: &[ProviderKind]) -> Result<Vec<ProviderKind>> {
        let items = available
            .iter()
            .map(|provider| format!("{} ({})", provider.display_name(), provider.as_str()))
            .collect::<Vec<_>>();
        let mut defaults = vec![false; items.len()];
        if let Some(first) = defaults.first_mut() {
            *first = true;
        }

        let selected = MultiSelect::with_theme(&Self::theme())
            .with_prompt("Select providers to configure")
            .items(&items)
            .defaults(&defaults)
            .interact()?;

        Ok(selected.into_iter().map(|index| available[index]).collect())
    }

    fn input_api_key(&mut self, provider: ProviderKind) -> Result<String> {
        Ok(Input::<String>::with_theme(&Self::theme())
            .with_prompt(format!("Paste API key for {}", provider.display_name()))
            .interact_text()?)
    }

    fn select_model(
        &mut self,
        provider: ProviderKind,
        available: &[String],
    ) -> Result<ModelSelection> {
        let mut items = available.to_vec();
        items.push("Custom model".to_owned());

        let selected = Select::with_theme(&Self::theme())
            .with_prompt(format!("Select model for {}", provider.display_name()))
            .items(&items)
            .default(0)
            .interact()?;

        if selected == available.len() {
            Ok(ModelSelection::Custom)
        } else {
            Ok(ModelSelection::Preset(available[selected].clone()))
        }
    }

    fn input_custom_model(&mut self, provider: ProviderKind) -> Result<String> {
        Ok(Input::<String>::with_theme(&Self::theme())
            .with_prompt(format!(
                "Enter custom model id for {}",
                provider.display_name()
            ))
            .interact_text()?)
    }
}

pub struct RealModelCatalog;

impl ModelCatalog for RealModelCatalog {
    fn list_models(&self, provider: ProviderKind, api_key: &str) -> Result<Vec<String>> {
        match provider {
            ProviderKind::Openai => list_openai_models(api_key),
            ProviderKind::Claude => list_claude_models(api_key),
            ProviderKind::Gemini => list_gemini_models(api_key),
        }
    }
}

pub fn run_init(path: &Path, ui: &mut dyn InitUi, catalog: &dyn ModelCatalog) -> Result<String> {
    if path.exists() {
        return Err(anyhow!("config file already exists: {}", path.display()));
    }
    let config = build_config(ui, catalog)?;
    Config::write_to_path(path, &config)?;
    Ok(format!("Created config file: {}", path.display()))
}

fn build_config(ui: &mut dyn InitUi, catalog: &dyn ModelCatalog) -> Result<Config> {
    let providers = ui.select_providers(ProviderKind::all())?;
    if providers.is_empty() {
        return Err(anyhow!("at least one provider must be selected"));
    }

    let mut provider_configs = BTreeMap::new();
    for provider in &providers {
        let api_key = ui.input_api_key(*provider)?.trim().to_owned();
        if api_key.is_empty() {
            return Err(anyhow!("provider `{}` api_key is empty", provider.as_str()));
        }

        let available_models = resolve_available_models(catalog, *provider, &api_key);
        let model = match ui.select_model(*provider, &available_models)? {
            ModelSelection::Preset(model) => model,
            ModelSelection::Custom => {
                let model = ui.input_custom_model(*provider)?.trim().to_owned();
                if model.is_empty() {
                    return Err(anyhow!("provider `{}` model is empty", provider.as_str()));
                }
                model
            }
        };

        provider_configs.insert(
            *provider,
            ProviderConfig {
                api_key,
                model: Some(model),
            },
        );
    }

    Ok(Config {
        default_providers: providers,
        providers: provider_configs,
    })
}

fn resolve_available_models(
    catalog: &dyn ModelCatalog,
    provider: ProviderKind,
    api_key: &str,
) -> Vec<String> {
    match catalog.list_models(provider, api_key) {
        Ok(models) if !models.is_empty() => models,
        _ => provider
            .init_models()
            .iter()
            .map(|model| (*model).to_owned())
            .collect(),
    }
}

fn list_openai_models(api_key: &str) -> Result<Vec<String>> {
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Model>,
    }

    #[derive(Deserialize)]
    struct Model {
        id: String,
    }

    let response: Response = ureq::get("https://api.openai.com/v1/models")
        .set("Authorization", &format!("Bearer {api_key}"))
        .call()
        .map_err(openai_http_error)?
        .into_json()
        .context("failed to decode OpenAI models response")?;

    let mut models = response
        .data
        .into_iter()
        .map(|model| model.id)
        .filter(|id| is_supported_openai_model(id))
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

fn list_claude_models(api_key: &str) -> Result<Vec<String>> {
    #[derive(Deserialize)]
    struct Response {
        data: Vec<Model>,
    }

    #[derive(Deserialize)]
    struct Model {
        id: String,
    }

    let response: Response = ureq::get("https://api.anthropic.com/v1/models")
        .set("x-api-key", api_key)
        .set("anthropic-version", "2023-06-01")
        .call()
        .map_err(claude_http_error)?
        .into_json()
        .context("failed to decode Claude models response")?;

    let mut models = response
        .data
        .into_iter()
        .map(|model| model.id)
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

fn list_gemini_models(api_key: &str) -> Result<Vec<String>> {
    #[derive(Deserialize)]
    struct Response {
        models: Vec<Model>,
    }

    #[derive(Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct Model {
        name: String,
        supported_generation_methods: Option<Vec<String>>,
    }

    let response: Response = ureq::get("https://generativelanguage.googleapis.com/v1beta/models")
        .query("key", api_key)
        .call()
        .map_err(gemini_http_error)?
        .into_json()
        .context("failed to decode Gemini models response")?;

    let mut models = response
        .models
        .into_iter()
        .filter(|model| {
            model
                .supported_generation_methods
                .as_ref()
                .is_some_and(|methods| methods.iter().any(|method| method == "generateContent"))
        })
        .filter_map(|model| {
            model
                .name
                .strip_prefix("models/")
                .map(|name| name.to_owned())
        })
        .collect::<Vec<_>>();
    models.sort();
    models.dedup();
    Ok(models)
}

fn is_supported_openai_model(id: &str) -> bool {
    let lower = id.to_ascii_lowercase();
    (lower.starts_with("gpt-") || lower.starts_with("chatgpt-"))
        && !lower.contains("image")
        && !lower.contains("audio")
        && !lower.contains("transcribe")
        && !lower.contains("tts")
        && !lower.contains("realtime")
        && !lower.contains("search")
}

fn openai_http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => anyhow!(
            "OpenAI model list request failed with status {}: {}",
            code,
            response
                .into_string()
                .unwrap_or_else(|_| "failed to read response body".to_owned())
        ),
        ureq::Error::Transport(error) => anyhow!("OpenAI model list transport error: {error}"),
    }
}

fn claude_http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => anyhow!(
            "Claude model list request failed with status {}: {}",
            code,
            response
                .into_string()
                .unwrap_or_else(|_| "failed to read response body".to_owned())
        ),
        ureq::Error::Transport(error) => anyhow!("Claude model list transport error: {error}"),
    }
}

fn gemini_http_error(error: ureq::Error) -> anyhow::Error {
    match error {
        ureq::Error::Status(code, response) => anyhow!(
            "Gemini model list request failed with status {}: {}",
            code,
            response
                .into_string()
                .unwrap_or_else(|_| "failed to read response body".to_owned())
        ),
        ureq::Error::Transport(error) => anyhow!("Gemini model list transport error: {error}"),
    }
}
