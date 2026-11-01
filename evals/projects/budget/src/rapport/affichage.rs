use crate::rapport::mensuel::{Poste, RapportMensuel};

/// Rapport lisible dans un terminal.
pub fn afficher_rapport(rapport: &RapportMensuel) -> String {
    let mut sortie = format!("Budget — {}\n\n", rapport.periode.libelle());
    for poste in &rapport.postes {
        sortie.push_str(&ligne_poste(poste));
    }
    sortie.push('\n');
    sortie.push_str(&format!(
        "Dépenses : {}\n",
        formater_montant(rapport.depenses.centimes())
    ));
    sortie.push_str(&format!(
        "Revenus  : {}\n",
        formater_montant(rapport.revenus.centimes())
    ));
    sortie.push_str(&format!(
        "Solde    : {}\n",
        formater_montant(rapport.solde().centimes())
    ));
    sortie
}

fn ligne_poste(poste: &Poste) -> String {
    format!(
        "{:<20} {:>14}  ({} opérations)\n",
        poste.categorie,
        formater_montant(poste.total.centimes()),
        poste.operations
    )
}

/// « -62,15 € ».
pub fn formater_montant(centimes: i64) -> String {
    let euros = centimes as f64 / 100.0;
    format!("{euros:.2} €").replace('.', ",")
}
