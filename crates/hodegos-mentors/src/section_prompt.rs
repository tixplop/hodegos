//! Prompt d'une section, tel qu'il part vers le modèle : secrets masqués, section,
//! diff, 20 lignes de contexte et fil éventuel. Partagé par la passe proactive, les
//! reprises d'une remarque et l'évaluation.

use hodegos_core::secrets::mask_secrets;
use hodegos_core::{Section, strip_html_comments, strip_html_comments_in_diff};

use crate::prompt::{Author, FollowUp, FollowUpKind, Prompt, PromptInput, build_prompt};
use crate::{Level, Mentor};

/// Lignes de contexte de part et d'autre de la section.
pub const CONTEXT_LINES: usize = 20;

/// Fil d'une remarque, textes en clair : ils sont masqués avant l'envoi.
#[derive(Debug, Clone)]
pub struct Thread<'a> {
    pub kind: FollowUpKind,
    pub quote: &'a str,
    pub levels_given: Vec<Level>,
    pub entries: Vec<(Author, &'a str)>,
}

/// Ce qu'il faut pour interroger un mentor sur une section.
#[derive(Debug, Clone)]
pub struct SectionRequest<'a> {
    pub mentor: &'a Mentor,
    pub level: Level,
    /// Chemin relatif du fichier.
    pub path: &'a str,
    pub language: &'a str,
    /// Contenu réel du fichier enregistré.
    pub content: &'a str,
    pub section: &'a Section,
    /// Diff de la section ; vide pour une reprise.
    pub diff: &'a str,
    pub thread: Option<&'a Thread<'a>>,
}

/// Construit le prompt ; rien de ce qui part n'échappe au masquage des secrets
/// (section, contexte, diff, citation et messages du fil). En Markdown, les
/// commentaires HTML sont retirés de la section, du contexte et du diff.
pub fn section_prompt(request: &SectionRequest<'_>) -> Prompt {
    let (masked, _) = mask_secrets(request.content);
    let all: Vec<&str> = masked.split('\n').collect();
    let section = request.section;
    let (diff, _) = mask_secrets(request.diff);
    let markdown = request.language == "markdown";
    let visible = |text: String| {
        if markdown {
            strip_html_comments(&text)
        } else {
            text
        }
    };
    let diff = if markdown {
        strip_html_comments_in_diff(&diff)
    } else {
        diff
    };

    let masked_thread = request.thread.map(|thread| {
        let quote = mask_secrets(thread.quote).0;
        let entries: Vec<(Author, String)> = thread
            .entries
            .iter()
            .map(|(author, text)| (*author, mask_secrets(text).0))
            .collect();
        (thread, quote, entries)
    });
    let follow_up = masked_thread
        .as_ref()
        .map(|(thread, quote, entries)| FollowUp {
            kind: thread.kind,
            quote,
            levels_given: thread.levels_given.clone(),
            entries: entries
                .iter()
                .map(|(author, text)| (*author, text.as_str()))
                .collect(),
        });

    build_prompt(&PromptInput {
        mentor: request.mentor,
        level: request.level,
        path: request.path,
        language: request.language,
        section_id: &section.id,
        section_text: &visible(lines(&all, section.start_line, section.end_line)),
        section_diff: &diff,
        context: &visible(context(&all, section)),
        follow_up: follow_up.as_ref(),
    })
}

/// Lignes `start..=end` (à partir de 1).
fn lines(all: &[&str], start: usize, end: usize) -> String {
    all.iter()
        .skip(start.saturating_sub(1))
        .take(end + 1 - start)
        .copied()
        .collect::<Vec<_>>()
        .join("\n")
}

/// Jusqu'à 20 lignes avant et 20 lignes après la section.
fn context(all: &[&str], section: &Section) -> String {
    let before_start = section.start_line.saturating_sub(CONTEXT_LINES).max(1);
    let before = if section.start_line > 1 {
        lines(all, before_start, section.start_line - 1)
    } else {
        String::new()
    };
    let last = all.len();
    let after = if section.end_line < last {
        lines(
            all,
            section.end_line + 1,
            (section.end_line + CONTEXT_LINES).min(last),
        )
    } else {
        String::new()
    };
    match (before.trim().is_empty(), after.trim().is_empty()) {
        (true, true) => String::new(),
        (false, true) => before,
        (true, false) => after,
        (false, false) => format!("{before}\n[…section…]\n{after}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Origin, Parsed, parse};

    fn section(start_line: usize, end_line: usize) -> Section {
        Section {
            id: "fn:a".to_owned(),
            kind: "fn".to_owned(),
            name: "a".to_owned(),
            start_line,
            end_line,
            parents: vec![],
        }
    }

    #[test]
    fn contexte_de_vingt_lignes_de_part_et_d_autre() {
        let text: Vec<String> = (1..=60).map(|n| format!("l{n}")).collect();
        let all: Vec<&str> = text.iter().map(String::as_str).collect();
        let context = context(&all, &section(30, 32));
        let lines: Vec<&str> = context.lines().collect();
        assert_eq!(lines.first(), Some(&"l10"));
        assert_eq!(lines[19], "l29");
        assert_eq!(lines[20], "[…section…]");
        assert_eq!(lines[21], "l33");
        assert_eq!(lines.last(), Some(&"l52"));
        assert_eq!(lines.len(), 41);
    }

    #[test]
    fn tout_ce_qui_part_est_masque() {
        let mentor = match parse(
            "---\nid: m\nname: M\ninitials: MM\nrole: R\n---\nPosture.\n",
            Origin::User,
            "m.md",
        )
        .unwrap()
        {
            Parsed::Active(m) => *m,
            Parsed::Disabled { .. } => unreachable!(),
        };
        let content = "const k = 1;\nfunction a() {\n  const token = \"ghp_abcdefghijklmnopqrstuvwxyz0123456789\";\n}";
        let thread = Thread {
            kind: FollowUpKind::Reply,
            quote: "const token",
            levels_given: vec![Level::Indice],
            entries: vec![(Author::Person, "ma clé : sk-abcdefghijklmnopqrstuvwxyz0123")],
        };
        let prompt = section_prompt(&SectionRequest {
            mentor: &mentor,
            level: Level::Indice,
            path: "src/a.ts",
            language: "typescript",
            content,
            section: &section(2, 4),
            diff: "+  const token = \"ghp_abcdefghijklmnopqrstuvwxyz0123456789\";\n",
            thread: Some(&thread),
        });
        assert!(!prompt.user.contains("ghp_"), "{}", prompt.user);
        assert!(!prompt.user.contains("sk-abc"), "{}", prompt.user);
        assert!(prompt.user.contains("[secret masqué]"));
        assert!(
            prompt
                .user
                .contains("<contexte>\nconst k = 1;\n</contexte>")
        );
    }

    #[test]
    fn les_commentaires_html_ne_partent_pas() {
        let mentor = match parse(
            "---\nid: m\nname: M\ninitials: MM\nrole: R\n---\nPosture.\n",
            Origin::User,
            "m.md",
        )
        .unwrap()
        {
            Parsed::Active(m) => *m,
            Parsed::Disabled { .. } => unreachable!(),
        };
        let content = "# essai\n<!-- avant -->\n\n## Idée\n\n<!-- consigne\nsur deux lignes -->\nMon idée\n\n## Pour qui\n\n<!-- après -->\n";
        let mut idee = section(4, 9);
        idee.id = "h:idee".to_owned();
        let prompt = section_prompt(&SectionRequest {
            mentor: &mentor,
            level: Level::Question,
            path: "docs/vision.md",
            language: "markdown",
            content,
            section: &idee,
            diff: "--- avant\n+++ après\n@@ -0,0 +1,4 @@\n+## Idée\n+<!-- consigne\n+sur deux lignes -->\n+Mon idée\n",
            thread: None,
        });
        assert!(!prompt.user.contains("<!--"), "{}", prompt.user);
        for hidden in ["avant -->", "consigne", "sur deux lignes", "après -->"] {
            assert!(!prompt.user.contains(hidden), "{hidden} : {}", prompt.user);
        }
        assert!(
            prompt.user.contains("## Idée\n\nMon idée"),
            "{}",
            prompt.user
        );
        assert!(prompt.user.contains("+Mon idée"), "{}", prompt.user);
    }
}
