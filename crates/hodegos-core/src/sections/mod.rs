//! Découpage d'un fichier en sections, par tree-sitter.
//!
//! Code : une section par élément de premier niveau (fonction, `impl`, classe…), avec
//! les commentaires, attributs et décorateurs qui le précèdent sans ligne vide ; le
//! reste (imports, instructions) forme des sections `top:N`. Markdown : une section par
//! titre, jusqu'au titre suivant ; le texte avant le premier titre forme `top:1`.
//!
//! Identifiant stable `kind:nom`, suffixé d'un rang (`fn:new-2`) en cas d'homonymes.

mod code;
mod markdown;

use std::collections::HashMap;

use crate::Language;

pub use markdown::frontmatter;

/// Section d'un fichier ; lignes numérotées à partir de 1, bornes incluses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
    /// Titres parents, du plus haut au plus proche (Markdown seulement).
    pub parents: Vec<String>,
}

impl Section {
    /// Texte de la section dans `content`.
    pub fn text<'a>(&self, content: &'a str) -> String {
        content
            .lines()
            .skip(self.start_line - 1)
            .take(self.end_line + 1 - self.start_line)
            .collect::<Vec<&'a str>>()
            .join("\n")
    }

    pub fn contains(&self, line: usize) -> bool {
        (self.start_line..=self.end_line).contains(&line)
    }
}

/// Sections de `content` ; un fichier sans grammaire forme une seule section `top:1`.
pub fn sections(language: Language, content: &str) -> Vec<Section> {
    let drafts = match language {
        Language::Markdown => markdown::sections(content),
        Language::Rust => code::sections(tree_sitter_rust::LANGUAGE.into(), content),
        Language::TypeScript => {
            code::sections(tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), content)
        }
        Language::JavaScript => code::sections(tree_sitter_javascript::LANGUAGE.into(), content),
        Language::Python => code::sections(tree_sitter_python::LANGUAGE.into(), content),
        Language::Text => None,
    };
    let drafts = drafts.unwrap_or_else(|| whole_file(content));
    number(drafts)
}

/// Section qui contient `line`, sinon la précédente, sinon la première.
pub fn section_at(sections: &[Section], line: usize) -> Option<&Section> {
    sections
        .iter()
        .find(|s| s.contains(line))
        .or_else(|| sections.iter().rev().find(|s| s.end_line < line))
        .or_else(|| sections.first())
}

/// Section avant numérotation : `name` vide pour les sections `top`.
pub(crate) struct Draft {
    pub(crate) kind: &'static str,
    pub(crate) name: String,
    pub(crate) slug: String,
    pub(crate) start_line: usize,
    pub(crate) end_line: usize,
    pub(crate) parents: Vec<String>,
}

fn whole_file(content: &str) -> Vec<Draft> {
    let lines = content.lines().count();
    if lines == 0 {
        return Vec::new();
    }
    vec![Draft {
        kind: "top",
        name: String::new(),
        slug: String::new(),
        start_line: 1,
        end_line: lines,
        parents: Vec::new(),
    }]
}

fn number(drafts: Vec<Draft>) -> Vec<Section> {
    let mut seen: HashMap<String, usize> = HashMap::new();
    let mut tops = 0;
    drafts
        .into_iter()
        .map(|draft| {
            let id = if draft.kind == "top" {
                tops += 1;
                format!("top:{tops}")
            } else {
                let base = format!("{}:{}", draft.kind, draft.slug);
                let rank = seen.entry(base.clone()).or_insert(0);
                *rank += 1;
                if *rank == 1 {
                    base
                } else {
                    format!("{base}-{rank}")
                }
            };
            Section {
                id,
                kind: draft.kind.to_owned(),
                name: draft.name,
                start_line: draft.start_line,
                end_line: draft.end_line,
                parents: draft.parents,
            }
        })
        .collect()
}

/// Dernière ligne (à partir de 1) d'un nœud, sans la ligne où il finit en colonne 0.
pub(crate) fn last_line(node: &tree_sitter::Node<'_>) -> usize {
    let end = node.end_position();
    let start = node.start_position();
    if end.column == 0 && end.row > start.row {
        end.row
    } else {
        end.row + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn un_fichier_texte_forme_une_seule_section() {
        let found = sections(Language::Text, "a = 1\nb = 2\n");
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].id.as_str(), found[0].start_line, found[0].end_line),
            ("top:1", 1, 2)
        );
        assert!(sections(Language::Text, "").is_empty());
    }

    #[test]
    fn section_at_retombe_sur_la_precedente_puis_la_premiere() {
        let found = sections(Language::Rust, "\n\nfn a() {}\n\n\nfn b() {}\n");
        assert_eq!(section_at(&found, 3).unwrap().id, "fn:a");
        assert_eq!(section_at(&found, 4).unwrap().id, "fn:a");
        assert_eq!(section_at(&found, 6).unwrap().id, "fn:b");
        assert_eq!(section_at(&found, 1).unwrap().id, "fn:a");
    }
}
