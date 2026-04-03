use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, anyhow};
use serde::{Deserialize, Serialize};

#[derive(
    Clone,
    Copy,
    Debug,
    Deserialize,
    Eq,
    Hash,
    Ord,
    PartialEq,
    PartialOrd,
    Serialize,
    clap::ValueEnum,
)]
#[serde(rename_all = "lowercase")]
pub enum ProviderKind {
    Openai,
    Claude,
    Gemini,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ProviderConfig {
    pub api_key: String,
    pub model: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Config {
    pub default_providers: Vec<ProviderKind>,
    pub providers: std::collections::BTreeMap<ProviderKind, ProviderConfig>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedProviderConfig {
    pub api_key: String,
    pub model: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppPaths {
    pub config_path: PathBuf,
    pub history_path: PathBuf,
}

impl AppPaths {
    pub fn from_base_dir(base_dir: &Path) -> Self {
        Self {
            config_path: base_dir.join("config.json"),
            history_path: base_dir.join("history.json"),
        }
    }

    pub fn discover() -> Result<Self> {
        let qql_dir = if let Some(xdg_config_home) = env::var_os("XDG_CONFIG_HOME") {
            PathBuf::from(xdg_config_home).join("qql")
        } else {
            dirs::home_dir()
                .map(|home| home.join(".config").join("qql"))
                .ok_or_else(|| anyhow!("failed to resolve home directory"))?
        };

        Ok(Self::from_base_dir(&qql_dir))
    }
}

impl Config {
    pub fn write_to_path(path: &Path, config: &Self) -> Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create config directory: {}", parent.display())
            })?;
        }

        let body = serde_json::to_string_pretty(config)?;
        fs::write(path, format!("{body}\n"))
            .with_context(|| format!("failed to write config file: {}", path.display()))?;
        Ok(())
    }

    pub fn load_from_path(path: &Path) -> Result<Self> {
        let raw = match fs::read_to_string(path) {
            Ok(raw) => raw,
            Err(error) if error.kind() == ErrorKind::NotFound => {
                return Err(anyhow!(
                    "failed to read config file: {}. Run `qql init` to create it.",
                    path.display()
                ));
            }
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("failed to read config file: {}", path.display()));
            }
        };
        let config: Self = serde_json::from_str(&raw)
            .with_context(|| format!("failed to parse config file: {}", path.display()))?;
        config.validate()?;
        Ok(config)
    }

    pub fn providers_to_use(
        &self,
        override_providers: &[ProviderKind],
    ) -> Result<Vec<ProviderKind>> {
        let source = if override_providers.is_empty() {
            &self.default_providers
        } else {
            override_providers
        };
        let mut seen = BTreeSet::new();
        let mut providers = Vec::new();

        for &provider in source {
            if !seen.insert(provider) {
                continue;
            }
            if !self.providers.contains_key(&provider) {
                return Err(anyhow!(
                    "provider `{}` is not configured",
                    provider.as_str()
                ));
            }
            providers.push(provider);
        }

        if providers.is_empty() {
            return Err(anyhow!("no providers selected"));
        }

        Ok(providers)
    }

    pub fn resolved_provider_config(&self, kind: ProviderKind) -> Result<ResolvedProviderConfig> {
        let provider = self
            .providers
            .get(&kind)
            .ok_or_else(|| anyhow!("provider `{}` is not configured", kind.as_str()))?;
        let api_key = provider.api_key.trim();
        if api_key.is_empty() {
            return Err(anyhow!("provider `{}` api_key is empty", kind.as_str()));
        }

        Ok(ResolvedProviderConfig {
            api_key: api_key.to_owned(),
            model: provider
                .model
                .clone()
                .unwrap_or_else(|| kind.default_model().to_owned()),
        })
    }

    fn validate(&self) -> Result<()> {
        if self.default_providers.is_empty() {
            return Err(anyhow!("default_providers must not be empty"));
        }

        for kind in self.providers.keys() {
            let _ = self.resolved_provider_config(*kind)?;
        }

        Ok(())
    }
}

impl ProviderKind {
    pub fn all() -> &'static [ProviderKind] {
        &[Self::Openai, Self::Claude, Self::Gemini]
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Openai => "openai",
            Self::Claude => "claude",
            Self::Gemini => "gemini",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Openai => "OpenAI",
            Self::Claude => "Claude",
            Self::Gemini => "Gemini",
        }
    }

    pub fn default_model(self) -> &'static str {
        match self {
            Self::Openai => "gpt-4o-mini",
            Self::Claude => "claude-haiku-4-5",
            Self::Gemini => "gemini-2.0-flash",
        }
    }

    pub fn init_models(self) -> &'static [&'static str] {
        match self {
            Self::Openai => &[
                "gpt-5.2",
                "gpt-5",
                "gpt-5-mini",
                "gpt-5.2-codex",
                "gpt-4o-mini",
            ],
            Self::Claude => &[
                "claude-sonnet-4-20250514",
                "claude-opus-4-1-20250805",
                "claude-3-5-haiku-latest",
            ],
            Self::Gemini => &[
                "gemini-2.5-flash",
                "gemini-2.5-flash-lite",
                "gemini-2.5-pro",
                "gemini-2.0-flash",
            ],
        }
    }
}
