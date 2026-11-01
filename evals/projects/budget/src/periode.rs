use crate::date::Date;
use crate::erreur::BudgetError;

const NOMS_DES_MOIS: [&str; 12] = [
    "janvier", "février", "mars", "avril", "mai", "juin", "juillet", "août",
    "septembre", "octobre", "novembre", "décembre",
];

/// Un mois civil.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Periode {
    annee: u32,
    mois: u32,
}

impl Periode {
    pub fn new(annee: u32, mois: u32) -> Result<Periode, BudgetError> {
        if !(1..=12).contains(&mois) {
            return Err(BudgetError::MoisInvalide(mois));
        }
        Ok(Periode { annee, mois })
    }

    pub fn annee(&self) -> u32 {
        self.annee
    }

    pub fn mois(&self) -> u32 {
        self.mois
    }

    pub fn contient(&self, date: &Date) -> bool {
        date.annee == self.annee && date.mois == self.mois
    }

    /// « mars 2026 ».
    pub fn libelle(&self) -> String {
        let nom = NOMS_DES_MOIS[(self.mois - 1) as usize];
        format!("{nom} {}", self.annee)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuse_un_treizieme_mois() {
        assert!(Periode::new(2026, 13).is_err());
    }

    #[test]
    fn libelle_en_toutes_lettres() {
        assert_eq!(Periode::new(2026, 8).unwrap().libelle(), "août 2026");
    }
}
