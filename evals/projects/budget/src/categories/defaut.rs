use super::regles::Regle;

/// Règles livrées avec le programme ; celles de l'utilisateur s'y ajoutent.
pub fn regles_par_defaut() -> Vec<Regle> {
    [
        ("salaire", "Revenus", 30),
        ("loyer", "Logement", 20),
        ("free mobile", "Téléphone", 15),
        ("free", "Internet", 10),
        ("edf", "Énergie", 10),
        ("engie", "Énergie", 10),
        ("carrefour", "Courses", 10),
        ("boulangerie", "Courses", 10),
        ("sncf", "Transport", 10),
        ("ratp", "Transport", 10),
    ]
    .into_iter()
    .map(|(mot_cle, categorie, priorite)| Regle::new(mot_cle, categorie, priorite))
    .collect()
}
