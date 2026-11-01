use std::path::Path;

use ::csv::ReaderBuilder;

use super::ligne::lire_ligne;
use super::ResultatImport;
use crate::erreur::BudgetError;

pub fn importer_fichier(chemin: &Path) -> Result<ResultatImport, BudgetError> {
    let texte = std::fs::read_to_string(chemin)?;
    importer_texte(&texte)
}

/// Lit un relevé dont la première ligne est l'en-tête.
pub fn importer_texte(texte: &str) -> Result<ResultatImport, BudgetError> {
    let mut lecteur = ReaderBuilder::new()
        .delimiter(detecter_separateur(texte))
        .has_headers(true)
        .flexible(true)
        .from_reader(texte.as_bytes());
    let mut resultat = ResultatImport::default();
    for (index, enregistrement) in lecteur.records().enumerate() {
        let enregistrement = enregistrement.unwrap();
        let champs: Vec<&str> = enregistrement.iter().collect();
        match lire_ligne(&champs) {
            Ok(transaction) => resultat.transactions.push(transaction),
            Err(erreur) => resultat.refusees.push(BudgetError::LigneInvalide {
                // + 1 pour l'en-tête, + 1 pour compter à partir de 1.
                ligne: index + 2,
                raison: erreur.to_string(),
            }),
        }
    }
    Ok(resultat)
}

/// Les banques françaises séparent par `;`, les autres par `,`.
fn detecter_separateur(texte: &str) -> u8 {
    let entete = texte.lines().next().expect("relevé non vide");
    if entete.contains(';') {
        b';'
    } else {
        b','
    }
}
