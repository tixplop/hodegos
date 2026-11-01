use std::iter::Sum;
use std::ops::{Add, AddAssign, Neg};

use crate::erreur::BudgetError;

/// Montant en centimes (ADR 0002). Négatif pour une dépense.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Montant(i64);

impl Montant {
    pub const ZERO: Montant = Montant(0);

    pub fn en_centimes(centimes: i64) -> Montant {
        Montant(centimes)
    }

    pub fn centimes(self) -> i64 {
        self.0
    }

    pub fn est_depense(self) -> bool {
        self.0 < 0
    }

    /// Lit « 12,34 », « -6,80 », « 2 450,00 » ou « 2450 ».
    pub fn depuis_texte(texte: &str) -> Result<Montant, BudgetError> {
        let invalide = || BudgetError::MontantInvalide(texte.to_owned());
        let propre: String = texte.chars().filter(|c| !c.is_whitespace()).collect();
        let (signe, chiffres) = match propre.strip_prefix('-') {
            Some(reste) => (-1, reste),
            None => (1, propre.as_str()),
        };
        let (euros, cents) = chiffres.split_once(',').unwrap_or((chiffres, "0"));
        if euros.is_empty() || cents.is_empty() || cents.len() > 2 {
            return Err(invalide());
        }
        let euros: i64 = euros.parse().map_err(|_| invalide())?;
        let cents: i64 = format!("{cents:0<2}").parse().map_err(|_| invalide())?;
        Ok(Montant(signe * (euros * 100 + cents)))
    }
}

impl Add for Montant {
    type Output = Montant;
    fn add(self, autre: Montant) -> Montant {
        Montant(self.0 + autre.0)
    }
}

impl AddAssign for Montant {
    fn add_assign(&mut self, autre: Montant) {
        self.0 += autre.0;
    }
}

impl Neg for Montant {
    type Output = Montant;
    fn neg(self) -> Montant {
        Montant(-self.0)
    }
}

impl Sum for Montant {
    fn sum<I: Iterator<Item = Montant>>(iter: I) -> Montant {
        iter.fold(Montant::ZERO, Add::add)
    }
}

/// Seule conversion d'un montant en texte : « -1234,50 € ».
pub fn formater(montant: Montant) -> String {
    let signe = if montant.0 < 0 { "-" } else { "" };
    let absolu = montant.0.abs();
    format!("{signe}{},{:02} €", absolu / 100, absolu % 100)
}
