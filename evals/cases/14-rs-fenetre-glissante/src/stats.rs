pub fn moyenne_glissante(valeurs: &[f64], fenetre: usize) -> Vec<f64> {
    let mut moyennes = Vec::new();
    for debut in 0..valeurs.len() - fenetre {
        let somme: f64 = valeurs[debut..debut + fenetre].iter().sum();
        moyennes.push(somme / fenetre as f64);
    }
    moyennes
}
