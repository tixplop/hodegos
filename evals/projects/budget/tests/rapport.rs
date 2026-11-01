use budget::date::Date;
use budget::periode::Periode;
use budget::rapport::affichage::afficher_rapport;
use budget::rapport::mensuel::{total_par_categorie, RapportMensuel};
use budget::transaction::Transaction;
use budget::Montant;

fn operation(libelle: &str, centimes: i64, categorie: Option<&str>) -> Transaction {
    let date = Date { annee: 2026, mois: 3, jour: 10 };
    let mut transaction = Transaction::new(date, libelle, Montant::en_centimes(centimes));
    transaction.categorie = categorie.map(str::to_owned);
    transaction
}

#[test]
fn regroupe_par_categorie_et_compte_les_operations() {
    let operations = [
        operation("CARREFOUR", -6215, Some("Courses")),
        operation("Boulangerie", -680, Some("Courses")),
        operation("PRLV SEPA EDF", -7840, Some("Énergie")),
    ];
    let refs: Vec<&Transaction> = operations.iter().collect();
    let postes = total_par_categorie(&refs);
    assert_eq!(postes.len(), 2);
    assert_eq!(postes[0].categorie, "Énergie");
    assert_eq!(postes[1].total.centimes(), -6895);
    assert_eq!(postes[1].operations, 2);
}

#[test]
fn range_les_operations_sans_categorie_dans_divers() {
    let operations = [operation("CHEQUE 1234", -2000, None)];
    let refs: Vec<&Transaction> = operations.iter().collect();
    assert_eq!(total_par_categorie(&refs)[0].categorie, "Divers");
}

#[test]
fn affiche_le_mois_et_le_solde() {
    let rapport = RapportMensuel {
        periode: Periode::new(2026, 3).unwrap(),
        postes: Vec::new(),
        depenses: Montant::en_centimes(-6215),
        revenus: Montant::en_centimes(245_000),
    };
    let texte = afficher_rapport(&rapport);
    assert!(texte.contains("mars 2026"));
    assert!(texte.contains("2387,85 €"));
}
