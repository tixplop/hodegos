use std::collections::BTreeMap;

use crate::categories::SANS_CATEGORIE;
use crate::erreur::BudgetError;
use crate::montant::Montant;
use crate::periode::Periode;
use crate::transaction::Transaction;

/// Total d'une catégorie sur le mois.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Poste {
    pub categorie: String,
    pub total: Montant,
    pub operations: usize,
}

#[derive(Debug, Clone)]
pub struct RapportMensuel {
    pub periode: Periode,
    pub postes: Vec<Poste>,
    pub depenses: Montant,
    pub revenus: Montant,
}

impl RapportMensuel {
    pub fn solde(&self) -> Montant {
        self.revenus + self.depenses
    }
}

pub fn rapport_du_mois(
    transactions: &[Transaction],
    annee: u32,
    mois: u32,
) -> Result<RapportMensuel, BudgetError> {
    let periode = Periode::new(mois, annee)?;
    let du_mois: Vec<&Transaction> = transactions
        .iter()
        .filter(|t| periode.contient(&t.date))
        .collect();
    let montants = || du_mois.iter().map(|t| t.montant);
    Ok(RapportMensuel {
        periode,
        postes: total_par_categorie(&du_mois),
        depenses: montants().filter(|m| m.est_depense()).sum(),
        revenus: montants().filter(|m| !m.est_depense()).sum(),
    })
}

pub fn total_par_categorie(transactions: &[&Transaction]) -> Vec<Poste> {
    let mut totaux: BTreeMap<&str, (Montant, usize)> = BTreeMap::new();
    for transaction in transactions {
        let categorie = transaction.categorie.as_deref().unwrap_or(SANS_CATEGORIE);
        let entree = totaux.entry(categorie).or_default();
        entree.0 += transaction.montant;
        entree.1 += 1;
    }
    let mut postes: Vec<Poste> = totaux
        .into_iter()
        .map(|(categorie, (total, operations))| Poste {
            categorie: categorie.to_owned(),
            total,
            operations,
        })
        .collect();
    trier_postes(&mut postes);
    postes
}

/// Les plus grosses dépenses en premier, les revenus à la fin.
pub fn trier_postes(postes: &mut [Poste]) {
    postes.sort_by_key(|p| p.total);
}
