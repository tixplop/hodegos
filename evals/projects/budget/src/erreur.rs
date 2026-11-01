use thiserror::Error;

/// Seul type d'erreur du programme (ADR 0001).
#[derive(Debug, Error)]
pub enum BudgetError {
    #[error("ligne {ligne} du relevé : {raison}")]
    LigneInvalide { ligne: usize, raison: String },
    #[error("ligne incomplète : {0} colonnes au lieu de 3")]
    LigneIncomplete(usize),
    #[error("montant illisible : « {0} »")]
    MontantInvalide(String),
    #[error("date illisible : « {0} »")]
    DateInvalide(String),
    #[error("mois invalide : {0}")]
    MoisInvalide(u32),
    #[error("règle illisible : « {0} »")]
    RegleInvalide(String),
    #[error("{0}")]
    Usage(String),
    #[error("lecture du relevé : {0}")]
    Csv(#[from] csv::Error),
    #[error("fichier : {0}")]
    Fichier(#[from] std::io::Error),
}
