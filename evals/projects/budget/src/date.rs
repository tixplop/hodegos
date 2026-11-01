use std::fmt;

use crate::erreur::BudgetError;

/// Date d'une opération, telle que la donne le relevé.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub annee: u32,
    pub mois: u32,
    pub jour: u32,
}

impl Date {
    /// Lit « jj/mm/aaaa ».
    pub fn depuis_texte(texte: &str) -> Result<Date, BudgetError> {
        let invalide = || BudgetError::DateInvalide(texte.to_owned());
        let parties: Vec<&str> = texte.trim().split('/').collect();
        let [jour, mois, annee] = parties.as_slice() else {
            return Err(invalide());
        };
        let jour: u32 = jour.parse().map_err(|_| invalide())?;
        let mois: u32 = mois.parse().map_err(|_| invalide())?;
        let annee: u32 = annee.parse().map_err(|_| invalide())?;
        if !(1..=31).contains(&jour) || !(1..=12).contains(&mois) || annee < 1970 {
            return Err(invalide());
        }
        Ok(Date { annee, mois, jour })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:02}/{:02}/{}", self.jour, self.mois, self.annee)
    }
}
