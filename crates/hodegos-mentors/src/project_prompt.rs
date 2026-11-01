//! Prompt d'une revue du projet (phase 9c, ADR 0010) : même cadre commun que pour une
//! section, sa règle de citation remplacée par celle d'une revue, puis un complément de
//! revue ; ni l'un ni l'autre ne sont modifiables par un fichier de mentor. Les textes
//! reçus sont déjà masqués.

use crate::prompt::{
    COMMON_FRAME, FOLLOW_UP_FRAME, FollowUp, Prompt, SECTION_QUOTE_RULE, escape, posture,
    thread_block,
};
use crate::{Level, Mentor};

/// Règle de citation d'une revue, à la place de celle d'une section.
const PROJECT_QUOTE_RULE: &str = "\
- Chaque remarque cite mot pour mot les passages qu'elle vise (champ quotes),
  pris dans les textes reçus.";

/// Complément de revue, après le cadre commun.
const REVIEW_FRAME: &str = "\
Revue du projet (blocs <carte_du_projet>, <fichiers_cles>,
<diff_depuis_derniere_revue> et <remarques_ouvertes>) :
- Vous relisez l'ensemble : cohérence entre fichiers, décisions et vision, pas une
  ligne isolée.
- about = passage si la remarque ne porte que sur un passage (exactement une
  citation) ; project si elle porte sur le projet (0 à 3 citations, chacune dans
  un fichier différent).
- Une citation : chemin exact d'un fichier reçu, et extrait de 3 à 120 caractères
  copié mot pour mot du texte reçu pour ce fichier ou de la carte.
- Ne répétez pas une remarque déjà ouverte (bloc <remarques_ouvertes>).
- kind next_step : une étape suivante, citations facultatives.";

/// Complément pour la suite d'une remarque de projet.
const PROJECT_FOLLOW_UP_FRAME: &str = "\
Remarque sur le projet (blocs <carte_du_projet> et <fichiers_cites>) :
- Les fichiers cités par la remarque sont reçus pour que vous la suiviez ; vous
  n'écrivez dans aucun d'eux.";

/// Complément d'une réévaluation (phase 9d).
const REEVALUATION_FRAME: &str = "\
Réévaluation d'une remarque sur le projet (blocs <carte_du_projet>,
<fichiers_cles>, <fichiers_cites> et <remarque>) :
- Un passage que citait votre remarque a disparu. Relisez-la à l'échelle du projet.
- verdict keep si elle reste vraie : quotes = 0 à 3 nouvelles citations, chacune
  dans un fichier différent, copiées mot pour mot des textes reçus ; reason vide.
- verdict withdraw si elle ne tient plus : reason = pourquoi, 200 caractères au
  plus, sans code ; quotes vide.
- Votre message et son palier ne changent pas ; vous n'écrivez rien d'autre.";

/// Fichier reçu en entier, ou réduit à ses signatures ou à ses titres.
#[derive(Debug, Clone)]
pub struct FileText<'a> {
    pub path: &'a str,
    /// « entier », « signatures », « titres », « section … ».
    pub form: &'a str,
    pub text: &'a str,
}

/// Diff d'un fichier depuis la dernière revue du mentor.
#[derive(Debug, Clone)]
pub struct FileDiff<'a> {
    pub path: &'a str,
    pub diff: &'a str,
}

/// Remarque de projet encore ouverte : message et citations.
#[derive(Debug, Clone)]
pub struct OpenRemark<'a> {
    pub message: &'a str,
    pub quotes: Vec<(&'a str, &'a str)>,
}

/// Ce que reçoit un mentor de projet pour une revue.
#[derive(Debug, Clone)]
pub struct ReviewInput<'a> {
    pub mentor: &'a Mentor,
    pub level: Level,
    pub map: &'a str,
    pub key_files: Vec<FileText<'a>>,
    /// Vide à la première revue, ou si rien n'a changé depuis la précédente.
    pub diff: Vec<FileDiff<'a>>,
    /// Aucune revue précédente de ce mentor.
    pub first: bool,
    pub open_remarks: Vec<OpenRemark<'a>>,
}

/// Suite d'une remarque de projet : carte, fichiers cités, fil.
#[derive(Debug, Clone)]
pub struct ProjectFollowUpInput<'a> {
    pub mentor: &'a Mentor,
    pub level: Level,
    pub map: &'a str,
    pub cited: Vec<FileText<'a>>,
    pub follow_up: &'a FollowUp<'a>,
}

/// Ce que reçoit un mentor pour réévaluer une remarque de projet.
#[derive(Debug, Clone)]
pub struct ReevaluationInput<'a> {
    pub mentor: &'a Mentor,
    pub level: Level,
    pub map: &'a str,
    pub key_files: Vec<FileText<'a>>,
    /// Sections encore citées.
    pub cited: Vec<FileText<'a>>,
    /// Passages disparus : (chemin, citation).
    pub gone: Vec<(&'a str, &'a str)>,
    pub follow_up: &'a FollowUp<'a>,
}

/// Prompt d'une réévaluation.
pub fn reevaluation_prompt(input: &ReevaluationInput<'_>) -> Prompt {
    let system = format!(
        "{}\n\n{REEVALUATION_FRAME}\n\n{}",
        frame(input.mentor, input.level),
        posture(input.mentor, "", "projet")
    );
    let mut gone = String::from("<passages_disparus>\n");
    for (path, quote) in &input.gone {
        gone.push_str(&format!(
            "<citation chemin=\"{path}\">{}</citation>\n",
            escape(quote)
        ));
    }
    gone.push_str("</passages_disparus>\n");
    let user = format!(
        "<carte_du_projet>\n{}\n</carte_du_projet>\n{}{}<remarque>\n{gone}{}</remarque>\nRéévaluez cette remarque.",
        input.map.trim_end(),
        files_block("fichiers_cles", &input.key_files),
        files_block("fichiers_cites", &input.cited),
        thread_block(input.follow_up),
    );
    Prompt { system, user }
}

fn frame(mentor: &Mentor, level: Level) -> String {
    let max = mentor.project.as_ref().map_or(3, |p| p.max_remarks);
    COMMON_FRAME
        .replace("{name}", &mentor.name)
        .replace("{level}", level.as_str())
        .replace("{max_remarks}", &max.to_string())
        .replace(SECTION_QUOTE_RULE, PROJECT_QUOTE_RULE)
}

fn files_block(tag: &str, files: &[FileText<'_>]) -> String {
    let mut out = format!("<{tag}>\n");
    for file in files {
        out.push_str(&format!(
            "<fichier chemin=\"{}\" forme=\"{}\">\n{}\n</fichier>\n",
            file.path,
            file.form,
            file.text.trim_end()
        ));
    }
    out.push_str(&format!("</{tag}>\n"));
    out
}

/// Prompt d'une revue.
pub fn review_prompt(input: &ReviewInput<'_>) -> Prompt {
    let system = format!(
        "{}\n\n{REVIEW_FRAME}\n\n{}",
        frame(input.mentor, input.level),
        posture(input.mentor, "", "projet")
    );
    let mut user = format!(
        "<carte_du_projet>\n{}\n</carte_du_projet>\n",
        input.map.trim_end()
    );
    user.push_str(&files_block("fichiers_cles", &input.key_files));
    if input.diff.is_empty() {
        user.push_str(if input.first {
            "<diff_depuis_derniere_revue>première revue : aucun diff</diff_depuis_derniere_revue>\n"
        } else {
            "<diff_depuis_derniere_revue>aucun changement depuis votre dernière revue</diff_depuis_derniere_revue>\n"
        });
    } else {
        user.push_str("<diff_depuis_derniere_revue>\n");
        for file in &input.diff {
            user.push_str(&format!(
                "<fichier chemin=\"{}\">\n{}\n</fichier>\n",
                file.path,
                file.diff.trim_end()
            ));
        }
        user.push_str("</diff_depuis_derniere_revue>\n");
    }
    user.push_str("<remarques_ouvertes>\n");
    for remark in &input.open_remarks {
        user.push_str("<remarque>\n");
        for (path, quote) in &remark.quotes {
            user.push_str(&format!(
                "<citation chemin=\"{path}\">{}</citation>\n",
                escape(quote)
            ));
        }
        user.push_str(&format!(
            "<message>{}</message>\n</remarque>\n",
            escape(remark.message)
        ));
    }
    user.push_str("</remarques_ouvertes>\n");
    user.push_str(&format!("Palier demandé : {}.", input.level.as_str()));
    Prompt { system, user }
}

/// Prompt du palier suivant ou de la relance d'une remarque de projet.
pub fn project_follow_up_prompt(input: &ProjectFollowUpInput<'_>) -> Prompt {
    let system = format!(
        "{}\n\n{PROJECT_FOLLOW_UP_FRAME}\n\n{FOLLOW_UP_FRAME}\n\n{}",
        frame(input.mentor, input.level),
        posture(input.mentor, "", "projet")
    );
    let user = format!(
        "<carte_du_projet>\n{}\n</carte_du_projet>\n{}{}Palier demandé : {}.",
        input.map.trim_end(),
        files_block("fichiers_cites", &input.cited),
        thread_block(input.follow_up),
        input.level.as_str()
    );
    Prompt { system, user }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prompt::{Author, FollowUpKind};
    use crate::{Origin, Parsed, parse};

    fn mentor() -> Mentor {
        let text = "---\nid: archi\nname: Archi\ninitials: AR\nrole: R\nscopes: [section, project]\nproject:\n  key_files: [\"docs/**\"]\n  max_remarks: 2\n---\nRelisez à l'échelle {scope}.\n";
        match parse(text, Origin::User, "archi.md").unwrap() {
            Parsed::Active(m) => *m,
            Parsed::Disabled { .. } => unreachable!(),
        }
    }

    #[test]
    fn revue_avec_carte_fichiers_cles_diff_et_remarques_ouvertes() {
        let mentor = mentor();
        let input = ReviewInput {
            mentor: &mentor,
            level: Level::Indice,
            map: "# troc — carte\n",
            key_files: vec![FileText {
                path: "docs/adr/0001.md",
                form: "entier",
                text: "## Décision\nUn seul module.",
            }],
            diff: vec![],
            first: true,
            open_remarks: vec![OpenRemark {
                message: "Comptez <les> écritures.",
                quotes: vec![("src/a.ts", "writeFileSync(")],
            }],
        };
        let prompt = review_prompt(&input);
        assert!(
            prompt
                .system
                .starts_with("Vous êtes Archi, un mentor dans Hodegos")
        );
        assert!(prompt.system.contains("Vous ne donnez jamais la solution"));
        assert!(prompt.system.contains("Au plus 2 remarques."));
        assert!(!prompt.system.contains(SECTION_QUOTE_RULE));
        assert!(prompt.system.contains("champ quotes"));
        assert!(prompt.system.contains("about = passage"));
        assert!(prompt.system.ends_with("Relisez à l'échelle projet."));
        let review = prompt.system.find("Revue du projet").unwrap();
        assert!(review < prompt.system.find("Relisez").unwrap());
        assert!(prompt.user.starts_with("<carte_du_projet>\n# troc — carte\n</carte_du_projet>\n<fichiers_cles>\n<fichier chemin=\"docs/adr/0001.md\" forme=\"entier\">\n## Décision\nUn seul module.\n</fichier>\n</fichiers_cles>\n"), "{}", prompt.user);
        assert!(prompt.user.contains("première revue : aucun diff"));
        assert!(
            prompt
                .user
                .contains("<citation chemin=\"src/a.ts\">writeFileSync(</citation>")
        );
        assert!(
            prompt
                .user
                .contains("<message>Comptez &lt;les&gt; écritures.</message>")
        );
        assert!(prompt.user.ends_with("Palier demandé : indice."));

        let unchanged = ReviewInput {
            first: false,
            ..input.clone()
        };
        assert!(
            review_prompt(&unchanged)
                .user
                .contains("aucun changement depuis votre dernière revue")
        );
        let with_diff = ReviewInput {
            diff: vec![FileDiff {
                path: "src/a.ts",
                diff: "+x\n",
            }],
            first: false,
            ..input
        };
        let prompt = review_prompt(&with_diff);
        assert!(prompt.user.contains("<diff_depuis_derniere_revue>\n<fichier chemin=\"src/a.ts\">\n+x\n</fichier>\n</diff_depuis_derniere_revue>"));
    }

    #[test]
    fn suite_d_une_remarque_de_projet() {
        let mentor = mentor();
        let follow_up = FollowUp {
            kind: FollowUpKind::NextLevel,
            quote: "writeFileSync(",
            levels_given: vec![Level::Indice],
            entries: vec![(Author::Mentor, "Comptez.")],
        };
        let prompt = project_follow_up_prompt(&ProjectFollowUpInput {
            mentor: &mentor,
            level: Level::Question,
            map: "# carte",
            cited: vec![FileText {
                path: "src/a.ts",
                form: "section fn:a",
                text: "function a() {}",
            }],
            follow_up: &follow_up,
        });
        assert!(prompt.system.contains("Suite d'une remarque"));
        assert!(prompt.system.contains("Remarque sur le projet"));
        assert!(
            prompt
                .user
                .contains("<fichiers_cites>\n<fichier chemin=\"src/a.ts\" forme=\"section fn:a\">")
        );
        assert!(prompt.user.contains("<fil>"));
        assert!(prompt.user.ends_with("Palier demandé : question."));
    }

    #[test]
    fn reevaluation_avec_passages_disparus() {
        let mentor = mentor();
        let follow_up = FollowUp {
            kind: FollowUpKind::Reply,
            quote: "writeFileSync(",
            levels_given: vec![Level::Indice],
            entries: vec![(Author::Mentor, "Comptez."), (Author::Person, "Deux.")],
        };
        let prompt = reevaluation_prompt(&ReevaluationInput {
            mentor: &mentor,
            level: Level::Indice,
            map: "# carte",
            key_files: vec![],
            cited: vec![],
            gone: vec![("src/email.ts", "writeFileSync(")],
            follow_up: &follow_up,
        });
        assert!(prompt.system.contains("Vous ne donnez jamais la solution"));
        assert!(prompt.system.contains("verdict keep"));
        assert!(prompt.user.contains(
            "<passages_disparus>\n<citation chemin=\"src/email.ts\">writeFileSync(</citation>"
        ));
        assert!(
            prompt
                .user
                .contains("<message auteur=\"personne\">Deux.</message>")
        );
    }
}
