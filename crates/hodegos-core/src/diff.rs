//! Sections modifiées entre deux versions d'un fichier, avec leur diff.

use std::collections::HashMap;

use similar::{ChangeTag, TextDiff};

use crate::{Language, Section, sections};

/// Section nouvelle ou modifiée, avec son diff unifié.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SectionChange {
    pub section: Section,
    /// Diff unifié de la section, ancienne version vide pour une section nouvelle.
    pub diff: String,
    /// Lignes ajoutées ou retirées.
    pub changed_lines: usize,
}

/// Sections de `new` absentes de `old` ou dont le texte a changé, dans l'ordre du fichier.
pub fn changed_sections(language: Language, old: &str, new: &str) -> Vec<SectionChange> {
    let before: HashMap<String, String> = sections(language, old)
        .into_iter()
        .map(|s| {
            let text = s.text(old);
            (s.id, text)
        })
        .collect();
    sections(language, new)
        .into_iter()
        .filter_map(|section| {
            let text = section.text(new);
            let previous = before.get(&section.id).map_or("", String::as_str);
            if previous == text {
                return None;
            }
            let diff = TextDiff::from_lines(previous, text.as_str());
            let changed_lines = diff
                .iter_all_changes()
                .filter(|c| c.tag() != ChangeTag::Equal)
                .count();
            let unified = diff
                .unified_diff()
                .context_radius(3)
                .header("avant", "après")
                .to_string();
            Some(SectionChange {
                section,
                diff: unified,
                changed_lines,
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const BEFORE: &str = "fn a() {\n    1\n}\n\nfn b() {\n    2\n}\n";

    #[test]
    fn retient_seulement_les_sections_modifiees_ou_nouvelles() {
        let after = "fn a() {\n    1\n}\n\nfn b() {\n    3\n}\n\nfn c() {}\n";
        let changes = changed_sections(Language::Rust, BEFORE, after);
        let ids: Vec<_> = changes.iter().map(|c| c.section.id.as_str()).collect();
        assert_eq!(ids, ["fn:b", "fn:c"]);
        assert_eq!(changes[0].changed_lines, 2);
        assert!(changes[0].diff.contains("-    2"), "{}", changes[0].diff);
        assert!(changes[0].diff.contains("+    3"), "{}", changes[0].diff);
        assert!(
            changes[1].diff.contains("+fn c() {}"),
            "{}",
            changes[1].diff
        );
    }

    #[test]
    fn un_deplacement_de_lignes_sans_changement_de_texte_ne_compte_pas() {
        let after = format!("\n\n{BEFORE}");
        assert!(changed_sections(Language::Rust, BEFORE, &after).is_empty());
        assert!(changed_sections(Language::Rust, BEFORE, BEFORE).is_empty());
    }
}
