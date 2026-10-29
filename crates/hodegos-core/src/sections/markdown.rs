use tree_sitter::{Node, Parser};

use super::Draft;

/// Frontmatter YAML en tête de fichier, entre deux lignes `---` : son texte et le
/// nombre de lignes qu'il occupe, délimiteurs compris.
pub fn frontmatter(content: &str) -> Option<(&str, usize)> {
    let mut lines = content.split_inclusive('\n');
    let first = lines.next()?;
    if first.trim_end() != "---" {
        return None;
    }
    let start = first.len();
    let mut offset = start;
    for (index, line) in lines.enumerate() {
        if matches!(line.trim_end(), "---" | "...") {
            return Some((&content[start..offset], index + 2));
        }
        offset += line.len();
    }
    None
}

struct Heading {
    line: usize,
    level: usize,
    name: String,
}

pub(super) fn sections(content: &str) -> Option<Vec<Draft>> {
    // Le frontmatter est remplacé par des lignes vides : `---` ne doit pas être lu
    // comme un soulignement de titre, et les numéros de ligne restent justes.
    let skipped = frontmatter(content).map_or(0, |(_, lines)| lines);
    let masked: String = content
        .split_inclusive('\n')
        .enumerate()
        .map(|(index, line)| {
            if index < skipped {
                if line.ends_with('\n') { "\n" } else { "" }
            } else {
                line
            }
        })
        .collect();

    let mut parser = Parser::new();
    parser.set_language(&tree_sitter_md::LANGUAGE.into()).ok()?;
    let tree = parser.parse(&masked, None)?;
    let mut headings = Vec::new();
    collect(&tree.root_node(), masked.as_bytes(), &mut headings);

    let total = masked.lines().count();
    let mut drafts = Vec::new();
    let first_heading = headings.first().map_or(total + 1, |h| h.line);
    let intro_has_text = masked
        .lines()
        .take(first_heading - 1)
        .skip(skipped)
        .any(|line| !line.trim().is_empty());
    if intro_has_text {
        drafts.push(Draft {
            kind: "top",
            name: String::new(),
            slug: String::new(),
            start_line: skipped + 1,
            end_line: first_heading - 1,
            parents: Vec::new(),
        });
    }

    let mut stack: Vec<(usize, String)> = Vec::new();
    for (index, heading) in headings.iter().enumerate() {
        while stack
            .last()
            .is_some_and(|(level, _)| *level >= heading.level)
        {
            stack.pop();
        }
        let end_line = headings.get(index + 1).map_or(total, |next| next.line - 1);
        drafts.push(Draft {
            kind: "h",
            slug: slug(&heading.name),
            name: heading.name.clone(),
            start_line: heading.line,
            end_line: end_line.max(heading.line),
            parents: stack.iter().map(|(_, name)| name.clone()).collect(),
        });
        stack.push((heading.level, heading.name.clone()));
    }
    Some(drafts)
}

fn collect(node: &Node<'_>, source: &[u8], out: &mut Vec<Heading>) {
    match node.kind() {
        "atx_heading" => {
            let level = (1..=6)
                .find(|n| {
                    let marker = format!("atx_h{n}_marker");
                    (0..node.child_count())
                        .filter_map(|i| node.child(i))
                        .any(|c| c.kind() == marker)
                })
                .unwrap_or(1);
            let text = heading_text(node, source);
            let name = text.trim_end_matches('#').trim().to_owned();
            out.push(Heading {
                line: node.start_position().row + 1,
                level,
                name,
            });
        }
        "setext_heading" => {
            let level = if (0..node.child_count())
                .filter_map(|i| node.child(i))
                .any(|c| c.kind() == "setext_h1_underline")
            {
                1
            } else {
                2
            };
            out.push(Heading {
                line: node.start_position().row + 1,
                level,
                name: heading_text(node, source).trim().to_owned(),
            });
        }
        _ => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                collect(&child, source, out);
            }
        }
    }
}

fn heading_text(node: &Node<'_>, source: &[u8]) -> String {
    node.child_by_field_name("heading_content")
        .and_then(|n| n.utf8_text(source).ok())
        .unwrap_or_default()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// `Décision d'architecture` → `decision-d-architecture`.
pub(crate) fn slug(name: &str) -> String {
    let ascii = deunicode::deunicode(name).to_lowercase();
    let slug = ascii
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("-");
    if slug.is_empty() {
        "sans-titre".to_owned()
    } else {
        slug
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Language;
    use crate::sections::sections as split;

    const ADR: &str = "\
---
mentors: [architecte]
---
# 0001 — Stockage

Intro.

## Contexte

Hors ligne exigé.

### Contraintes détaillées

```
## pas un titre
```

Sous-titre
----------

## Décision

PostgreSQL hébergé.

## Décision
";

    #[test]
    fn une_section_par_titre_avec_ses_parents() {
        let found = split(Language::Markdown, ADR);
        let summary: Vec<_> = found
            .iter()
            .map(|s| {
                (
                    s.id.as_str(),
                    s.start_line,
                    s.end_line,
                    s.parents.join(" > "),
                )
            })
            .collect();
        assert_eq!(
            summary,
            [
                ("h:0001-stockage", 4, 7, String::new()),
                ("h:contexte", 8, 11, "0001 — Stockage".to_owned()),
                (
                    "h:contraintes-detaillees",
                    12,
                    17,
                    "0001 — Stockage > Contexte".to_owned()
                ),
                // Souligné par `-` : titre de niveau 2, frère de « Contexte ».
                ("h:sous-titre", 18, 20, "0001 — Stockage".to_owned()),
                ("h:decision", 21, 24, "0001 — Stockage".to_owned()),
                ("h:decision-2", 25, 25, "0001 — Stockage".to_owned()),
            ]
        );
        assert_eq!(found[4].name, "Décision");
    }

    #[test]
    fn le_texte_avant_le_premier_titre_forme_top() {
        let found = split(Language::Markdown, "Préambule.\n\n## Titre\n");
        assert_eq!(found[0].id, "top:1");
        assert_eq!((found[0].start_line, found[0].end_line), (1, 2));
        assert_eq!(found[1].id, "h:titre");
    }

    #[test]
    fn le_frontmatter_n_est_ni_un_titre_ni_une_section() {
        assert_eq!(frontmatter(ADR), Some(("mentors: [architecte]\n", 3)));
        assert_eq!(frontmatter("# Pas de frontmatter\n"), None);
        assert_eq!(frontmatter("---\nnon fermé\n"), None);
        let found = split(Language::Markdown, "---\ntitre: x\n---\n");
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn slug_sans_accents_ni_ponctuation() {
        assert_eq!(slug("Décision"), "decision");
        assert_eq!(slug("  Conséquences & risques ! "), "consequences-risques");
        assert_eq!(slug("***"), "sans-titre");
    }
}
