use crate::date::Date;
use crate::montant::Montant;

/// Une opération du relevé.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Transaction {
    pub date: Date,
    pub libelle: String,
    pub montant: Montant,
    /// Attribuée par les règles de classement ; `None` tant qu'aucune ne s'applique.
    pub categorie: Option<String>,
}

impl Transaction {
    pub fn new(date: Date, libelle: &str, montant: Montant) -> Transaction {
        Transaction {
            date,
            libelle: libelle.to_owned(),
            montant,
            categorie: None,
        }
    }
}
