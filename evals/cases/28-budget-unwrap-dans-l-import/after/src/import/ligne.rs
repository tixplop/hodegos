use crate::date::Date;
use crate::erreur::BudgetError;
use crate::montant::Montant;
use crate::transaction::Transaction;

/// Colonnes attendues : date, libellé, montant.
pub fn lire_ligne(champs: &[&str]) -> Result<Transaction, BudgetError> {
    if champs.len() < 3 {
        return Err(BudgetError::LigneIncomplete(champs.len()));
    }
    let date = Date::depuis_texte(champs[0]).expect("date du relevé");
    let libelle = nettoyer_libelle(champs[1]);
    let montant = Montant::depuis_texte(champs[2]).unwrap();
    Ok(Transaction::new(date, &libelle, montant))
}

/// Variante des banques qui séparent les colonnes : date, libellé, débit, crédit.
pub fn lire_ligne_debit_credit(champs: &[&str]) -> Result<Transaction, BudgetError> {
    if champs.len() < 4 {
        return Err(BudgetError::LigneIncomplete(champs.len()));
    }
    let date = Date::depuis_texte(champs[0]).unwrap();
    let libelle = nettoyer_libelle(champs[1]);
    let colonne = |texte: &str| {
        if texte.trim().is_empty() {
            Montant::ZERO
        } else {
            Montant::depuis_texte(texte).expect("montant du relevé")
        }
    };
    let montant = colonne(champs[3]) + -colonne(champs[2]);
    Ok(Transaction::new(date, &libelle, montant))
}

/// Retire les espaces en trop et le préfixe de carte (« CB »).
pub fn nettoyer_libelle(brut: &str) -> String {
    let mots: Vec<&str> = brut.split_whitespace().collect();
    match mots.as_slice() {
        ["CB", reste @ ..] if !reste.is_empty() => reste.join(" "),
        _ => mots.join(" "),
    }
}
