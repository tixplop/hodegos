use crate::rapport::affichage::formater_montant;
use crate::rapport::mensuel::RapportMensuel;

/// Rapport en texte tabulé, à coller dans un tableur.
pub fn exporter_tsv(rapport: &RapportMensuel) -> String {
    let mut sortie = String::from("categorie\ttotal\toperations\n");
    for poste in &rapport.postes {
        sortie.push_str(&format!(
            "{}\t{}\t{}\n",
            poste.categorie,
            formater_montant(poste.total.centimes()),
            poste.operations
        ));
    }
    sortie.push_str(&format!(
        "Solde\t{}\t\n",
        formater_montant(rapport.solde().centimes())
    ));
    sortie
}
