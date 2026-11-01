use std::path::Path;
use std::process::ExitCode;

use budget::categories::{categoriser, defaut::regles_par_defaut, Classeur};
use budget::config::{charger_regles, chemin_des_regles};
use budget::import::{importer_fichier, ResultatImport};
use budget::rapport::{affichage::afficher_rapport, rapport_du_mois};
use budget::BudgetError;

const USAGE: &str = "usage : budget import <relevé.csv> | budget rapport <relevé.csv> <année> <mois>";

fn main() -> ExitCode {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    match executer(&arguments) {
        Ok(()) => ExitCode::SUCCESS,
        Err(erreur) => {
            eprintln!("budget : {erreur}");
            ExitCode::FAILURE
        }
    }
}

fn executer(arguments: &[String]) -> Result<(), BudgetError> {
    match arguments {
        [commande, releve] if commande == "import" => {
            let resultat = importer(Path::new(releve))?;
            println!("{} opérations lues", resultat.transactions.len());
            for refus in &resultat.refusees {
                println!("  refusée — {refus}");
            }
            Ok(())
        }
        [commande, releve, an, m] if commande == "rapport" => {
            let resultat = importer(Path::new(releve))?;
            let rapport = rapport_du_mois(&resultat.transactions, nombre(an)?, nombre(m)?)?;
            print!("{}", afficher_rapport(&rapport));
            Ok(())
        }
        _ => Err(BudgetError::Usage(USAGE.to_owned())),
    }
}

fn importer(releve: &Path) -> Result<ResultatImport, BudgetError> {
    let mut regles = regles_par_defaut();
    if let Some(chemin) = chemin_des_regles() {
        regles.extend(charger_regles(&chemin)?);
    }
    let mut resultat = importer_fichier(releve)?;
    categoriser(&mut resultat.transactions, &Classeur::new(regles));
    Ok(resultat)
}

fn nombre(texte: &str) -> Result<u32, BudgetError> {
    texte
        .parse()
        .map_err(|_| BudgetError::Usage(USAGE.to_owned()))
}
