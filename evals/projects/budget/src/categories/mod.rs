//! Classement des opérations par mot-clé du libellé.

pub mod defaut;
pub mod regles;

pub use regles::{Classeur, Regle};

use crate::transaction::Transaction;

/// Catégorie des opérations qu'aucune règle ne reconnaît.
pub const SANS_CATEGORIE: &str = "Divers";

/// Attribue une catégorie aux opérations qui n'en ont pas encore.
/// Renvoie le nombre d'opérations classées.
pub fn categoriser(transactions: &mut [Transaction], classeur: &Classeur) -> usize {
    let mut classees = 0;
    for transaction in transactions.iter_mut().filter(|t| t.categorie.is_none()) {
        if let Some(categorie) = classeur.classer(&transaction.libelle) {
            transaction.categorie = Some(categorie.to_owned());
            classees += 1;
        }
    }
    classees
}

/// Les opérations restées sans catégorie, pour suggérer de nouvelles règles.
pub fn non_classees(transactions: &[Transaction]) -> Vec<&Transaction> {
    transactions
        .iter()
        .filter(|t| t.categorie.is_none())
        .collect()
}
