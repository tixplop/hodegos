//! Lecture des relevés bancaires.

mod csv;
pub mod ligne;

pub use self::csv::{importer_fichier, importer_texte};

use crate::erreur::BudgetError;
use crate::transaction::Transaction;

/// Ce qu'un import a pu lire, et les lignes refusées avec leur raison.
#[derive(Debug, Default)]
pub struct ResultatImport {
    pub transactions: Vec<Transaction>,
    pub refusees: Vec<BudgetError>,
}

impl ResultatImport {
    pub fn est_complet(&self) -> bool {
        self.refusees.is_empty()
    }
}
