pub fn archiver(lignes: &[String], archive: &mut Vec<String>) {
    lignes.iter().map(|ligne| archive.push(ligne.to_uppercase()));
}
