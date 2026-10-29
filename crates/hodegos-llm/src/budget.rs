use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

use jiff::Timestamp;
use jiff::tz::TimeZone;
use serde::{Deserialize, Serialize};

use crate::BudgetConfig;

/// Mois du budget, en UTC : `2026-10`.
pub fn month_of(at: Timestamp) -> String {
    at.to_zoned(TimeZone::UTC).strftime("%Y-%m").to_string()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetLevel {
    Ok,
    /// Seuil d'avertissement atteint (80 % par défaut).
    Warning,
    /// Plafond atteint : plus de passe proactive.
    Exceeded,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BudgetState {
    pub used_usd: f64,
    pub cap_usd: f64,
    pub level: BudgetLevel,
}

#[derive(Serialize, Deserialize, Default)]
struct UsageFile {
    /// Coût cumulé par mois (UTC), en dollars.
    months: BTreeMap<String, f64>,
}

/// Budget mensuel dur, cumulé dans `usage.json` (données de l'utilisateur).
pub struct Budget {
    path: PathBuf,
    config: BudgetConfig,
    months: Mutex<BTreeMap<String, f64>>,
}

impl Budget {
    /// Lit `path` s'il existe ; un fichier illisible repart de zéro, avec un avertissement.
    pub fn open(path: PathBuf, config: BudgetConfig) -> Self {
        let months = match fs::read_to_string(&path) {
            Ok(text) => serde_json::from_str::<UsageFile>(&text)
                .map(|f| f.months)
                .unwrap_or_else(|err| {
                    tracing::warn!(path = %path.display(), error = %err, "usage.json illisible, cumul remis à zéro");
                    BTreeMap::new()
                }),
            Err(_) => BTreeMap::new(),
        };
        Self {
            path,
            config,
            months: Mutex::new(months),
        }
    }

    pub fn state(&self, at: Timestamp) -> BudgetState {
        let used = self
            .months
            .lock()
            .map(|m| m.get(&month_of(at)).copied().unwrap_or(0.0))
            .unwrap_or(0.0);
        self.state_for(used)
    }

    fn state_for(&self, used: f64) -> BudgetState {
        let cap = self.config.monthly_cap_usd;
        let level = if used >= cap {
            BudgetLevel::Exceeded
        } else if used >= cap * self.config.warn_at {
            BudgetLevel::Warning
        } else {
            BudgetLevel::Ok
        };
        BudgetState {
            used_usd: used,
            cap_usd: cap,
            level,
        }
    }

    /// Ajoute le coût d'un appel au mois de `at` et l'écrit sur disque.
    pub fn record(&self, cost_usd: f64, at: Timestamp) -> std::io::Result<BudgetState> {
        let (used, snapshot) = {
            let mut months = self
                .months
                .lock()
                .map_err(|_| std::io::Error::other("verrou du budget empoisonné"))?;
            let used = months.entry(month_of(at)).or_insert(0.0);
            *used += cost_usd.max(0.0);
            (*used, months.clone())
        };
        self.write(&snapshot)?;
        Ok(self.state_for(used))
    }

    fn write(&self, months: &BTreeMap<String, f64>) -> std::io::Result<()> {
        let dir = self
            .path
            .parent()
            .ok_or_else(|| std::io::Error::other("chemin de usage.json sans dossier"))?;
        fs::create_dir_all(dir)?;
        let text = serde_json::to_string_pretty(&UsageFile {
            months: months.clone(),
        })
        .map_err(std::io::Error::other)?;
        let mut temp = tempfile::NamedTempFile::new_in(dir)?;
        temp.write_all(text.as_bytes())?;
        temp.persist(&self.path).map_err(|e| e.error)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(date: &str) -> Timestamp {
        date.parse().unwrap()
    }

    fn budget(dir: &std::path::Path) -> Budget {
        Budget::open(
            dir.join("usage.json"),
            BudgetConfig {
                monthly_cap_usd: 10.0,
                warn_at: 0.8,
            },
        )
    }

    #[test]
    fn avertit_a_80_pour_cent_et_bloque_a_100() {
        let dir = tempfile::tempdir().unwrap();
        let budget = budget(dir.path());
        let now = at("2026-10-29T12:00:00Z");
        assert_eq!(budget.state(now).level, BudgetLevel::Ok);
        assert_eq!(budget.record(7.9, now).unwrap().level, BudgetLevel::Ok);
        assert_eq!(budget.record(0.2, now).unwrap().level, BudgetLevel::Warning);
        let state = budget.record(2.0, now).unwrap();
        assert_eq!(state.level, BudgetLevel::Exceeded);
        assert!((state.used_usd - 10.1).abs() < 1e-9);
    }

    #[test]
    fn le_cumul_est_mensuel_en_utc_et_survit_au_redemarrage() {
        let dir = tempfile::tempdir().unwrap();
        let budget = budget(dir.path());
        budget.record(9.5, at("2026-10-31T23:30:00Z")).unwrap();
        assert_eq!(budget.state(at("2026-11-01T00:30:00Z")).used_usd, 0.0);
        assert_eq!(month_of(at("2026-10-31T23:30:00Z")), "2026-10");

        let reopened = super::Budget::open(dir.path().join("usage.json"), budget.config.clone());
        assert_eq!(
            reopened.state(at("2026-10-15T00:00:00Z")).level,
            BudgetLevel::Warning
        );
    }

    #[test]
    fn un_fichier_illisible_repart_de_zero() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("usage.json"), "pas du json").unwrap();
        assert_eq!(
            budget(dir.path())
                .state(at("2026-10-01T00:00:00Z"))
                .used_usd,
            0.0
        );
    }
}
