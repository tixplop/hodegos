use std::fmt;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;

use crate::openrouter::OPENROUTER_URL;

/// Clé d'API : jamais affichée par `Debug`, jamais journalisée.
#[derive(Clone, PartialEq, Eq)]
pub struct ApiKey(String);

impl ApiKey {
    pub fn new(key: impl Into<String>) -> Self {
        Self(key.into())
    }

    pub(crate) fn expose(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for ApiKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ApiKey(…)")
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BudgetConfig {
    pub monthly_cap_usd: f64,
    /// Fraction du plafond à partir de laquelle on avertit.
    pub warn_at: f64,
}

/// `~/.config/hodegos/config.toml`. Aucun identifiant de modèle n'est écrit dans le code.
#[derive(Debug, Clone, PartialEq)]
pub struct LlmConfig {
    pub fast: Option<String>,
    pub strong: Option<String>,
    pub timeout: Duration,
    pub base_url: String,
    pub budget: BudgetConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            fast: None,
            strong: None,
            timeout: Duration::from_secs(30),
            base_url: OPENROUTER_URL.to_owned(),
            budget: BudgetConfig {
                monthly_cap_usd: 10.0,
                warn_at: 0.8,
            },
        }
    }
}

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum ConfigError {
    #[error("{path} : {message}")]
    Invalid { path: String, message: String },
    #[error("{0} est lisible par d'autres utilisateurs ; restreignez-le avec chmod 600 {0}")]
    TooOpen(String),
}

#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    llm: RawLlm,
    #[serde(default)]
    budget: RawBudget,
}

#[derive(Deserialize, Default)]
struct RawLlm {
    provider: Option<String>,
    fast: Option<String>,
    strong: Option<String>,
    timeout_secs: Option<u64>,
    base_url: Option<String>,
}

#[derive(Deserialize, Default)]
struct RawBudget {
    monthly_cap_usd: Option<f64>,
    warn_at: Option<f64>,
}

#[derive(Deserialize)]
struct RawCredentials {
    openrouter_api_key: Option<String>,
}

/// Valeur renseignée : ni vide, ni espace réservé (« [identifiant du modèle] »).
fn filled(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_owned())
        .filter(|v| !v.is_empty() && !v.starts_with('['))
}

impl LlmConfig {
    /// Lit `<config_dir>/config.toml` ; un fichier absent donne les valeurs par défaut.
    pub fn load(config_dir: &Path) -> Result<Self, ConfigError> {
        let path = config_dir.join("config.toml");
        let text = match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(invalid(&path, e.to_string())),
        };
        let raw: RawConfig =
            toml::from_str(&text).map_err(|e| invalid(&path, e.message().to_owned()))?;
        if let Some(provider) = raw.llm.provider.as_deref().filter(|p| *p != "openrouter") {
            return Err(invalid(
                &path,
                format!("fournisseur « {provider} » inconnu (seul openrouter est pris en charge)"),
            ));
        }
        let defaults = Self::default();
        let budget = BudgetConfig {
            monthly_cap_usd: raw
                .budget
                .monthly_cap_usd
                .unwrap_or(defaults.budget.monthly_cap_usd),
            warn_at: raw.budget.warn_at.unwrap_or(defaults.budget.warn_at),
        };
        let cap = budget.monthly_cap_usd;
        if cap.is_nan() || cap < 0.0 || !(0.0..=1.0).contains(&budget.warn_at) {
            return Err(invalid(
                &path,
                "budget : monthly_cap_usd ≥ 0 et warn_at entre 0 et 1".to_owned(),
            ));
        }
        Ok(Self {
            fast: filled(raw.llm.fast),
            strong: filled(raw.llm.strong),
            timeout: raw
                .llm
                .timeout_secs
                .map_or(defaults.timeout, Duration::from_secs),
            base_url: filled(raw.llm.base_url).unwrap_or(defaults.base_url),
            budget,
        })
    }
}

fn invalid(path: &Path, message: String) -> ConfigError {
    ConfigError::Invalid {
        path: path.display().to_string(),
        message,
    }
}

/// Clé OpenRouter : la variable `OPENROUTER_API_KEY` (passée en `env_key`) l'emporte,
/// sinon `<config_dir>/credentials.toml`, qui doit être en droits 600.
pub fn load_api_key(
    config_dir: &Path,
    env_key: Option<String>,
) -> Result<Option<ApiKey>, ConfigError> {
    if let Some(key) = filled(env_key) {
        return Ok(Some(ApiKey(key)));
    }
    let path: PathBuf = config_dir.join("credentials.toml");
    let metadata = match fs::metadata(&path) {
        Ok(metadata) => metadata,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(invalid(&path, e.to_string())),
    };
    if metadata.permissions().mode() & 0o077 != 0 {
        return Err(ConfigError::TooOpen(path.display().to_string()));
    }
    let text = fs::read_to_string(&path).map_err(|e| invalid(&path, e.to_string()))?;
    let raw: RawCredentials =
        toml::from_str(&text).map_err(|e| invalid(&path, e.message().to_owned()))?;
    Ok(filled(raw.openrouter_api_key).map(ApiKey))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(dir: &Path, name: &str, text: &str, mode: u32) {
        let path = dir.join(name);
        fs::write(&path, text).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(mode)).unwrap();
    }

    #[test]
    fn valeurs_par_defaut_sans_fichier() {
        let dir = tempfile::tempdir().unwrap();
        let config = LlmConfig::load(dir.path()).unwrap();
        assert_eq!(config, LlmConfig::default());
        assert_eq!(config.fast, None);
        assert_eq!(config.budget.monthly_cap_usd, 10.0);
    }

    #[test]
    fn lit_le_fichier_et_ignore_les_espaces_reserves() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "config.toml",
            "[llm]\nprovider = \"openrouter\"\nfast = \"a/rapide\"\nstrong = \"[identifiant OpenRouter du modèle fort]\"\ntimeout_secs = 12\n\n[budget]\nmonthly_cap_usd = 5.0\nwarn_at = 0.5\n",
            0o644,
        );
        let config = LlmConfig::load(dir.path()).unwrap();
        assert_eq!(config.fast.as_deref(), Some("a/rapide"));
        assert_eq!(config.strong, None);
        assert_eq!(config.timeout, Duration::from_secs(12));
        assert_eq!(
            config.budget,
            BudgetConfig {
                monthly_cap_usd: 5.0,
                warn_at: 0.5
            }
        );
    }

    #[test]
    fn refuse_une_configuration_invalide() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "config.toml",
            "[llm]\nprovider = \"autre\"\n",
            0o644,
        );
        assert!(matches!(
            LlmConfig::load(dir.path()),
            Err(ConfigError::Invalid { .. })
        ));
        write(dir.path(), "config.toml", "[budget]\nwarn_at = 3\n", 0o644);
        assert!(matches!(
            LlmConfig::load(dir.path()),
            Err(ConfigError::Invalid { .. })
        ));
        write(dir.path(), "config.toml", "[llm\n", 0o644);
        assert!(matches!(
            LlmConfig::load(dir.path()),
            Err(ConfigError::Invalid { .. })
        ));
    }

    #[test]
    fn la_cle_vient_de_l_environnement_ou_d_un_fichier_en_600() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_api_key(dir.path(), None), Ok(None));
        write(
            dir.path(),
            "credentials.toml",
            "openrouter_api_key = \"sk-fichier\"\n",
            0o600,
        );
        assert_eq!(
            load_api_key(dir.path(), None),
            Ok(Some(ApiKey::new("sk-fichier")))
        );
        assert_eq!(
            load_api_key(dir.path(), Some("sk-env".into())),
            Ok(Some(ApiKey::new("sk-env")))
        );
        assert_eq!(
            load_api_key(dir.path(), Some(String::new())),
            Ok(Some(ApiKey::new("sk-fichier")))
        );

        write(
            dir.path(),
            "credentials.toml",
            "openrouter_api_key = \"sk-fichier\"\n",
            0o644,
        );
        assert!(matches!(
            load_api_key(dir.path(), None),
            Err(ConfigError::TooOpen(_))
        ));
        assert!(!format!("{:?}", ApiKey::new("sk-secret")).contains("sk-secret"));
    }
}
