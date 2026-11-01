use budget::import::importer_texte;
use budget::import::ligne::nettoyer_libelle;

const RELEVE: &str = "\
date;libelle;montant
01/03/2026;VIR SALAIRE ACME;2450,00
03/03/2026;CB CARREFOUR 03/03;-62,15
";

#[test]
fn importe_toutes_les_lignes_d_un_releve_propre() {
    let resultat = importer_texte(RELEVE).unwrap();
    assert!(resultat.est_complet());
    assert_eq!(resultat.transactions.len(), 2);
    assert_eq!(resultat.transactions[1].montant.centimes(), -6215);
}

#[test]
fn accepte_la_virgule_comme_separateur() {
    let releve = "date,libelle,montant\n02/03/2026,PRLV SEPA EDF,\"-78,40\"\n";
    let resultat = importer_texte(releve).unwrap();
    assert_eq!(resultat.transactions[0].libelle, "PRLV SEPA EDF");
}

#[test]
fn refuse_une_ligne_incomplete_et_garde_les_autres() {
    let releve = format!("{RELEVE}04/03/2026;SANS MONTANT\n");
    let resultat = importer_texte(&releve).unwrap();
    assert_eq!(resultat.transactions.len(), 2);
    assert_eq!(resultat.refusees.len(), 1);
}

#[test]
fn retire_le_prefixe_de_carte() {
    assert_eq!(nettoyer_libelle("CB   CARREFOUR  03/03"), "CARREFOUR 03/03");
}
