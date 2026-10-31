use std::fmt;
use std::fs;
use std::io::Write as _;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use toml_edit::{DocumentMut, Item, Table, Value};

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

    /// Quatre derniers caractères, pour reconnaître la clé sans la montrer.
    pub fn hint(&self) -> String {
        let chars: Vec<char> = self.0.chars().collect();
        chars[chars.len().saturating_sub(4)..].iter().collect()
    }
}

/// Effort de raisonnement d'un niveau ; « Par défaut » n'envoie rien au modèle.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Effort {
    #[default]
    Default,
    Low,
    Medium,
    High,
}

impl Effort {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Default => "default",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }

    pub fn from_name(name: &str) -> Option<Self> {
        [Self::Default, Self::Low, Self::Medium, Self::High]
            .into_iter()
            .find(|e| e.as_str() == name)
    }

    /// Valeur de `reasoning.effort` dans la requête, absente pour « Par défaut ».
    pub fn reasoning(self) -> Option<&'static str> {
        (self != Self::Default).then(|| self.as_str())
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
    pub fast_effort: Effort,
    pub strong_effort: Effort,
    pub timeout: Duration,
    pub base_url: String,
    pub budget: BudgetConfig,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            fast: None,
            strong: None,
            fast_effort: Effort::Default,
            strong_effort: Effort::Default,
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
    #[error("{path} : écriture impossible : {message}")]
    Write { path: String, message: String },
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
    fast_effort: Option<String>,
    strong_effort: Option<String>,
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
        let effort = |name: &str, value: Option<String>| match value.as_deref() {
            None => Ok(Effort::Default),
            Some(text) => Effort::from_name(text).ok_or_else(|| {
                invalid(
                    &path,
                    format!("{name} : « {text} » inconnu (default, low, medium ou high)"),
                )
            }),
        };
        Ok(Self {
            fast: filled(raw.llm.fast),
            strong: filled(raw.llm.strong),
            fast_effort: effort("fast_effort", raw.llm.fast_effort)?,
            strong_effort: effort("strong_effort", raw.llm.strong_effort)?,
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

/// Réglages modifiables depuis l'IDE (phase 8d).
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub fast: String,
    pub strong: String,
    pub fast_effort: Effort,
    pub strong_effort: Effort,
    pub budget: BudgetConfig,
}

/// Écrit les réglages dans `<config_dir>/config.toml`, en gardant commentaires, ordre
/// et autres clés ; écriture atomique.
pub fn write_settings(config_dir: &Path, settings: &Settings) -> Result<(), ConfigError> {
    let path = config_dir.join("config.toml");
    let (mut doc, mode) = read_document(&path)?;
    let llm = table(&mut doc, "llm", &path)?;
    set(llm, "fast", Value::from(settings.fast.as_str()));
    set(llm, "strong", Value::from(settings.strong.as_str()));
    set(
        llm,
        "fast_effort",
        Value::from(settings.fast_effort.as_str()),
    );
    set(
        llm,
        "strong_effort",
        Value::from(settings.strong_effort.as_str()),
    );
    let budget = table(&mut doc, "budget", &path)?;
    set(
        budget,
        "monthly_cap_usd",
        Value::from(settings.budget.monthly_cap_usd),
    );
    set(budget, "warn_at", Value::from(settings.budget.warn_at));
    write_atomic(&path, &doc.to_string(), mode.unwrap_or(0o644))
}

/// Écrit la clé dans `<config_dir>/credentials.toml`, lisible par vous seul (600).
pub fn write_api_key(config_dir: &Path, key: &ApiKey) -> Result<(), ConfigError> {
    let path = config_dir.join("credentials.toml");
    let (mut doc, _) = read_document(&path)?;
    set(
        doc.as_table_mut(),
        "openrouter_api_key",
        Value::from(key.expose()),
    );
    write_atomic(&path, &doc.to_string(), 0o600)
}

/// Document existant (vide s'il manque) et ses droits.
fn read_document(path: &Path) -> Result<(DocumentMut, Option<u32>), ConfigError> {
    let text = match fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            return Ok((DocumentMut::new(), None));
        }
        Err(e) => return Err(invalid(path, e.to_string())),
    };
    let mode = fs::metadata(path)
        .ok()
        .map(|m| m.permissions().mode() & 0o777);
    let doc = text
        .parse::<DocumentMut>()
        .map_err(|e| invalid(path, e.message().to_owned()))?;
    Ok((doc, mode))
}

/// Table `[name]`, créée si besoin ; une clé `name` qui n'est pas une table est refusée.
fn table<'a>(
    doc: &'a mut DocumentMut,
    name: &str,
    path: &Path,
) -> Result<&'a mut Table, ConfigError> {
    doc.entry(name)
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .ok_or_else(|| invalid(path, format!("{name} doit être une table [{name}]")))
}

/// Remplace une valeur en gardant la clé, ses commentaires et celui de fin de ligne.
fn set(table: &mut Table, key: &str, mut value: Value) {
    match table.get_mut(key) {
        Some(item) => {
            if let Some(old) = item.as_value() {
                *value.decor_mut() = old.decor().clone();
            }
            *item = Item::Value(value);
        }
        None => {
            table.insert(key, Item::Value(value));
        }
    }
}

fn write_atomic(path: &Path, text: &str, mode: u32) -> Result<(), ConfigError> {
    let failed = |e: std::io::Error| ConfigError::Write {
        path: path.display().to_string(),
        message: e.to_string(),
    };
    let dir = path.parent().unwrap_or(Path::new("."));
    fs::create_dir_all(dir).map_err(failed)?;
    let mut file = tempfile::NamedTempFile::new_in(dir).map_err(failed)?;
    file.as_file()
        .set_permissions(fs::Permissions::from_mode(mode))
        .map_err(failed)?;
    file.write_all(text.as_bytes()).map_err(failed)?;
    file.as_file().sync_all().map_err(failed)?;
    file.persist(path).map_err(|e| failed(e.error))?;
    Ok(())
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

    #[test]
    fn lit_l_effort_et_refuse_une_valeur_inconnue() {
        let dir = tempfile::tempdir().unwrap();
        write(
            dir.path(),
            "config.toml",
            "[llm]\nfast = \"a/b\"\nstrong_effort = \"medium\"\n",
            0o644,
        );
        let config = LlmConfig::load(dir.path()).unwrap();
        assert_eq!(config.fast_effort, Effort::Default);
        assert_eq!(config.strong_effort, Effort::Medium);
        write(
            dir.path(),
            "config.toml",
            "[llm]\nfast_effort = \"max\"\n",
            0o644,
        );
        let err = LlmConfig::load(dir.path()).unwrap_err().to_string();
        assert!(err.contains("fast_effort"), "{err}");
    }

    #[test]
    fn ecrire_les_reglages_garde_commentaires_et_ordre() {
        let dir = tempfile::tempdir().unwrap();
        let original = "\
# Configuration de Hodegos

[llm]
provider = \"openrouter\"
# Modèle du Mentor du langage
fast = \"ancien/rapide\" # choisi en phase 6
strong = \"ancien/fort\"
timeout_secs = 20

[budget]
monthly_cap_usd = 10.0 # dollars
";
        write(dir.path(), "config.toml", original, 0o640);
        let settings = Settings {
            fast: "nouveau/rapide".to_owned(),
            strong: "ancien/fort".to_owned(),
            fast_effort: Effort::Default,
            strong_effort: Effort::High,
            budget: BudgetConfig {
                monthly_cap_usd: 5.0,
                warn_at: 0.5,
            },
        };
        write_settings(dir.path(), &settings).unwrap();
        let text = fs::read_to_string(dir.path().join("config.toml")).unwrap();
        assert!(text.starts_with("# Configuration de Hodegos\n\n[llm]\nprovider = \"openrouter\"\n# Modèle du Mentor du langage\nfast = \"nouveau/rapide\" # choisi en phase 6\n"), "{text}");
        assert!(text.contains("timeout_secs = 20"), "{text}");
        assert!(text.contains("monthly_cap_usd = 5.0 # dollars"), "{text}");
        let mode = fs::metadata(dir.path().join("config.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o640, "droits conservés");
        let config = LlmConfig::load(dir.path()).unwrap();
        assert_eq!(config.fast.as_deref(), Some("nouveau/rapide"));
        assert_eq!(config.strong_effort, Effort::High);
        assert_eq!(config.timeout, Duration::from_secs(20));
        assert_eq!(config.budget, settings.budget);

        // Sans fichier : créé en 644.
        let empty = tempfile::tempdir().unwrap();
        write_settings(empty.path(), &settings).unwrap();
        assert_eq!(
            LlmConfig::load(empty.path()).unwrap().fast_effort,
            Effort::Default
        );
        let mode = fs::metadata(empty.path().join("config.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o644);
    }

    #[test]
    fn la_cle_ecrite_est_en_600_et_relue() {
        let dir = tempfile::tempdir().unwrap();
        write_api_key(dir.path(), &ApiKey::new("sk-or-nouvelle-a3f9")).unwrap();
        let mode = fs::metadata(dir.path().join("credentials.toml"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o600);
        assert_eq!(
            load_api_key(dir.path(), None),
            Ok(Some(ApiKey::new("sk-or-nouvelle-a3f9")))
        );
        assert_eq!(ApiKey::new("sk-or-nouvelle-a3f9").hint(), "a3f9");
        assert_eq!(ApiKey::new("ab").hint(), "ab");
    }
}
