use std::fs;
use std::path::{Path, PathBuf};

use crate::categories::Regle;
use crate::erreur::BudgetError;

/// `~/.config/budget/regles.txt`, s'il existe.
pub fn chemin_des_regles() -> Option<PathBuf> {
    let maison = std::env::var_os("HOME")?;
    let chemin = PathBuf::from(maison).join(".config/budget/regles.txt");
    chemin.exists().then_some(chemin)
}

/// Une règle par ligne : `mot-clé;catégorie;priorité`. `#` commente la ligne.
pub fn charger_regles(chemin: &Path) -> Result<Vec<Regle>, BudgetError> {
    let texte = fs::read_to_string(chemin)?;
    let mut regles = Vec::new();
    for ligne in texte.lines().map(str::trim) {
        if ligne.is_empty() || ligne.starts_with('#') {
            continue;
        }
        let parties: Vec<&str> = ligne.split(';').map(str::trim).collect();
        let [mot_cle, categorie, priorite] = parties.as_slice() else {
            return Err(BudgetError::RegleInvalide(ligne.to_owned()));
        };
        let priorite: u8 = priorite
            .parse()
            .map_err(|_| BudgetError::RegleInvalide(ligne.to_owned()))?;
        regles.push(Regle::new(mot_cle, categorie, priorite));
    }
    Ok(regles)
}
