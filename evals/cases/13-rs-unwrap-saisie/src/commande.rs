use std::io;

pub fn lire_quantite() -> u32 {
    let mut ligne = String::new();
    io::stdin().read_line(&mut ligne).unwrap();
    ligne.trim().parse().unwrap()
}
