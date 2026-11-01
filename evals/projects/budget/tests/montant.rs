use budget::{formater, Montant};

#[test]
fn lit_un_montant_avec_virgule() {
    assert_eq!(Montant::depuis_texte("12,34").unwrap().centimes(), 1234);
}

#[test]
fn lit_une_depense() {
    assert_eq!(Montant::depuis_texte("-6,8").unwrap().centimes(), -680);
}

#[test]
fn lit_un_montant_entier_avec_espaces() {
    assert_eq!(Montant::depuis_texte("2 450").unwrap().centimes(), 245_000);
}

#[test]
fn refuse_trois_decimales() {
    assert!(Montant::depuis_texte("1,234").is_err());
}

#[test]
fn formate_avec_virgule_et_symbole() {
    assert_eq!(formater(Montant::en_centimes(-123_450)), "-1234,50 €");
    assert_eq!(formater(Montant::en_centimes(5)), "0,05 €");
}

#[test]
fn additionne_sans_arrondi() {
    let total: Montant = [10, 20, 70].into_iter().map(Montant::en_centimes).sum();
    assert_eq!(total.centimes(), 100);
}
