use crate::montant::Montant;
use crate::periode::Periode;
use crate::transaction::Transaction;

/// Un compte bancaire et ses opérations importées.
#[derive(Debug, Default)]
pub struct Compte {
    pub nom: String,
    transactions: Vec<Transaction>,
}

impl Compte {
    pub fn new(nom: &str) -> Compte {
        Compte {
            nom: nom.to_owned(),
            transactions: Vec::new(),
        }
    }

    /// Ajoute les opérations, sans doublon (même date, libellé et montant).
    pub fn ajouter(&mut self, nouvelles: Vec<Transaction>) -> usize {
        let avant = self.transactions.len();
        for transaction in nouvelles {
            if !self.transactions.contains(&transaction) {
                self.transactions.push(transaction);
            }
        }
        self.transactions.len() - avant
    }

    pub fn transactions(&self) -> &[Transaction] {
        &self.transactions
    }

    pub fn transactions_de(&self, periode: &Periode) -> Vec<&Transaction> {
        self.transactions
            .iter()
            .filter(|t| periode.contient(&t.date))
            .collect()
    }

    pub fn solde(&self) -> Montant {
        self.transactions.iter().map(|t| t.montant).sum()
    }
}
