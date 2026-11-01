//! Rapports calculés à partir des opérations classées.

pub mod affichage;
pub mod export;
pub mod mensuel;

pub use mensuel::{rapport_du_mois, Poste, RapportMensuel};
