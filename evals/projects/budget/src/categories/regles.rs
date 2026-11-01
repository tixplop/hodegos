/// « Si le libellé contient `mot_cle`, l'opération va dans `categorie`. »
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Regle {
    mot_cle: String,
    categorie: String,
    priorite: u8,
}

impl Regle {
    pub fn new(mot_cle: &str, categorie: &str, priorite: u8) -> Regle {
        Regle {
            mot_cle: mot_cle.trim().to_lowercase(),
            categorie: categorie.trim().to_owned(),
            priorite,
        }
    }

    /// Sans tenir compte de la casse : « Carrefour » et « CARREFOUR ».
    pub fn correspond(&self, libelle: &str) -> bool {
        !self.mot_cle.is_empty() && libelle.to_lowercase().contains(&self.mot_cle)
    }

    pub fn categorie(&self) -> &str {
        &self.categorie
    }

    pub fn priorite(&self) -> u8 {
        self.priorite
    }
}

/// Règles rangées pour que la première qui correspond soit la bonne.
#[derive(Debug, Default)]
pub struct Classeur {
    regles: Vec<Regle>,
}

impl Classeur {
    pub fn new(regles: Vec<Regle>) -> Classeur {
        let mut classeur = Classeur::default();
        for regle in regles {
            classeur.ajouter(regle);
        }
        classeur
    }

    /// Priorité la plus haute d'abord ; à priorité égale, le mot-clé le plus
    /// long, puis l'ordre d'ajout.
    pub fn ajouter(&mut self, regle: Regle) {
        let rang = |r: &Regle| (u8::MAX - r.priorite, usize::MAX - r.mot_cle.len());
        let position = self.regles.partition_point(|r| rang(r) <= rang(&regle));
        self.regles.insert(position, regle);
    }

    pub fn classer(&self, libelle: &str) -> Option<&str> {
        self.regles
            .iter()
            .find(|r| r.correspond(libelle))
            .map(Regle::categorie)
    }

    pub fn regles(&self) -> &[Regle] {
        &self.regles
    }
}
