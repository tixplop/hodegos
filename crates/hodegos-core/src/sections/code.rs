use tree_sitter::{Language, Node, Parser};

use super::{Draft, last_line};

/// Rôle d'un nœud de premier niveau.
enum Role {
    /// Élément nommé : sa propre section.
    Item(&'static str, String),
    /// Commentaire, attribut, décorateur : rattaché à l'élément qui suit.
    Attached,
    /// Tout le reste : sections `top`.
    Top,
}

pub(super) fn sections(language: Language, content: &str) -> Option<Vec<Draft>> {
    let mut parser = Parser::new();
    parser.set_language(&language).ok()?;
    let tree = parser.parse(content, None)?;
    let root = tree.root_node();
    let source = content.as_bytes();

    let mut drafts = Vec::new();
    // Nœuds rattachables en attente : (première ligne, dernière ligne).
    let mut pending: Option<(usize, usize)> = None;
    // Section `top` en cours : (première ligne, dernière ligne).
    let mut top: Option<(usize, usize)> = None;

    let mut cursor = root.walk();
    for node in root.named_children(&mut cursor) {
        let start = node.start_position().row + 1;
        let end = last_line(&node);
        match role(&node, source) {
            Role::Attached => {
                pending = match pending {
                    Some((first, last)) if start <= last + 1 => Some((first, end)),
                    Some(previous) => {
                        extend(&mut top, previous);
                        Some((start, end))
                    }
                    None => Some((start, end)),
                };
            }
            Role::Top => {
                if let Some(previous) = pending.take() {
                    extend(&mut top, previous);
                }
                extend(&mut top, (start, end));
            }
            Role::Item(kind, name) => {
                let mut first = start;
                if let Some((attached_first, attached_last)) = pending.take() {
                    if start <= attached_last + 1 {
                        first = attached_first;
                    } else {
                        extend(&mut top, (attached_first, attached_last));
                    }
                }
                flush(&mut drafts, &mut top);
                drafts.push(Draft {
                    kind,
                    slug: name.clone(),
                    name,
                    start_line: first,
                    end_line: end,
                    parents: Vec::new(),
                });
            }
        }
    }
    if let Some(previous) = pending.take() {
        extend(&mut top, previous);
    }
    flush(&mut drafts, &mut top);
    Some(drafts)
}

fn extend(top: &mut Option<(usize, usize)>, (start, end): (usize, usize)) {
    *top = Some(match *top {
        Some((first, last)) => (first, last.max(end)),
        None => (start, end),
    });
}

fn flush(drafts: &mut Vec<Draft>, top: &mut Option<(usize, usize)>) {
    if let Some((start, end)) = top.take() {
        drafts.push(Draft {
            kind: "top",
            name: String::new(),
            slug: String::new(),
            start_line: start,
            end_line: end,
            parents: Vec::new(),
        });
    }
}

fn role(node: &Node<'_>, source: &[u8]) -> Role {
    let named = |kind| match field_text(node, "name", source) {
        Some(name) => Role::Item(kind, name),
        None => Role::Top,
    };
    match node.kind() {
        "line_comment" | "block_comment" | "comment" | "attribute_item" | "decorator" => {
            Role::Attached
        }
        // Rust
        "function_item" | "function_signature_item" => named("fn"),
        "struct_item" | "union_item" => named("struct"),
        "enum_item" | "enum_declaration" => named("enum"),
        "trait_item" => named("trait"),
        "mod_item" | "internal_module" | "module" => named("mod"),
        "const_item" | "static_item" => named("const"),
        "type_item" | "type_alias_declaration" => named("type"),
        "macro_definition" => named("macro"),
        "impl_item" => impl_role(node, source),
        // TypeScript, JavaScript
        "function_declaration" | "generator_function_declaration" => named("fn"),
        "class_declaration" | "abstract_class_declaration" => named("class"),
        "interface_declaration" => named("interface"),
        "lexical_declaration" | "variable_declaration" => declaration_role(node, source),
        "export_statement" => export_role(node, source),
        "expression_statement" => match node.named_child(0) {
            Some(inner) if inner.kind() == "internal_module" => role(&inner, source),
            _ => Role::Top,
        },
        // Python
        "function_definition" => named("fn"),
        "class_definition" => named("class"),
        "decorated_definition" => match node.child_by_field_name("definition") {
            Some(definition) => role(&definition, source),
            None => Role::Top,
        },
        _ => Role::Top,
    }
}

/// `impl Point` → `impl:Point` ; `impl Display for Point` → `impl:Display-for-Point`.
fn impl_role(node: &Node<'_>, source: &[u8]) -> Role {
    let Some(target) = node
        .child_by_field_name("type")
        .and_then(|n| type_name(&n, source))
    else {
        return Role::Top;
    };
    let name = match node
        .child_by_field_name("trait")
        .and_then(|n| type_name(&n, source))
    {
        Some(tr) => format!("{tr}-for-{target}"),
        None => target,
    };
    Role::Item("impl", name)
}

/// Nom d'un type sans ses paramètres génériques ni son chemin : `fmt::Display<T>` → `Display`.
fn type_name(node: &Node<'_>, source: &[u8]) -> Option<String> {
    let text = node.utf8_text(source).ok()?;
    let base = text.split('<').next()?.trim();
    let name = base.rsplit("::").next()?.trim();
    (!name.is_empty()).then(|| name.to_owned())
}

/// `const f = () => …` est une fonction ; `const N = 1`, une constante.
fn declaration_role(node: &Node<'_>, source: &[u8]) -> Role {
    let Some(declarator) = node
        .named_child(0)
        .filter(|n| n.kind() == "variable_declarator")
    else {
        return Role::Top;
    };
    let Some(name) = declarator
        .child_by_field_name("name")
        .filter(|n| n.kind() == "identifier")
        .and_then(|n| n.utf8_text(source).ok())
    else {
        return Role::Top;
    };
    let is_function = declarator.child_by_field_name("value").is_some_and(|v| {
        matches!(
            v.kind(),
            "arrow_function" | "function_expression" | "function" | "generator_function"
        )
    });
    Role::Item(if is_function { "fn" } else { "const" }, name.to_owned())
}

fn export_role(node: &Node<'_>, source: &[u8]) -> Role {
    if let Some(declaration) = node.child_by_field_name("declaration") {
        return role(&declaration, source);
    }
    match node.child_by_field_name("value") {
        Some(value)
            if matches!(
                value.kind(),
                "function_expression" | "arrow_function" | "function"
            ) =>
        {
            Role::Item("fn", "default".to_owned())
        }
        Some(_) => Role::Item("const", "default".to_owned()),
        None => Role::Top,
    }
}

fn field_text(node: &Node<'_>, field: &str, source: &[u8]) -> Option<String> {
    node.child_by_field_name(field)?
        .utf8_text(source)
        .ok()
        .map(str::to_owned)
}

#[cfg(test)]
mod tests {
    use crate::Language;
    use crate::sections::{Section, sections};

    fn summary(found: &[Section]) -> Vec<(String, usize, usize)> {
        found
            .iter()
            .map(|s| (s.id.clone(), s.start_line, s.end_line))
            .collect()
    }

    fn ids(found: &[Section]) -> Vec<&str> {
        found.iter().map(|s| s.id.as_str()).collect()
    }

    const RUST: &str = "\
use std::fmt;

/// Un point.
#[derive(Debug)]
struct Point {
    x: i32,
}

impl Point {
    fn new() -> Self {
        Point { x: 0 }
    }
}

impl<T> fmt::Display for Wrap<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        Ok(())
    }
}

// Commentaire isolé.

fn main() {}
macro_rules! m { () => {} }
const N: u8 = 1;
mod outils {}
trait Forme {}
enum Couleur { Rouge }
fn main() {}
";

    #[test]
    fn rust_un_element_par_section_avec_ses_attributs() {
        let found = sections(Language::Rust, RUST);
        assert_eq!(
            summary(&found),
            [
                ("top:1".to_owned(), 1, 1),
                ("struct:Point".to_owned(), 3, 7),
                ("impl:Point".to_owned(), 9, 13),
                ("impl:Display-for-Wrap".to_owned(), 15, 19),
                ("top:2".to_owned(), 21, 21),
                ("fn:main".to_owned(), 23, 23),
                ("macro:m".to_owned(), 24, 24),
                ("const:N".to_owned(), 25, 25),
                ("mod:outils".to_owned(), 26, 26),
                ("trait:Forme".to_owned(), 27, 27),
                ("enum:Couleur".to_owned(), 28, 28),
                ("fn:main-2".to_owned(), 29, 29),
            ]
        );
        assert_eq!(found[1].name, "Point");
    }

    #[test]
    fn rust_identifiants_stables_apres_modification() {
        let before = sections(Language::Rust, RUST);
        let edited = RUST
            .replace("Point { x: 0 }", "Point {\n            x: 0,\n        }")
            .replace("use std::fmt;", "use std::fmt;\n\nfn ajoutee() {}");
        let after = sections(Language::Rust, &edited);
        for section in &before {
            assert!(
                ids(&after).contains(&section.id.as_str()),
                "{} disparu",
                section.id
            );
        }
        assert!(ids(&after).contains(&"fn:ajoutee"));
        let reserver = after.iter().find(|s| s.id == "impl:Point").unwrap();
        assert_eq!(reserver.end_line - reserver.start_line, 6);
    }

    const TS: &str = "\
import { a } from 'a';

// Réserve un objet.
export function reserver(e: number): boolean {
  return true;
}
export const f = (x: number) => x;
const N = 1;
export interface Emprunt { debut: Date }
export default function () {}
export class Agenda {}
type Id = string;
enum Etat { Libre }
namespace Outils {}
export { N };
@dec
class Deco {}
";

    #[test]
    fn typescript_fonctions_classes_types_et_reste() {
        assert_eq!(
            ids(&sections(Language::TypeScript, TS)),
            [
                "top:1",
                "fn:reserver",
                "fn:f",
                "const:N",
                "interface:Emprunt",
                "fn:default",
                "class:Agenda",
                "type:Id",
                "enum:Etat",
                "mod:Outils",
                "top:2",
                "class:Deco",
            ]
        );
        let found = sections(Language::TypeScript, TS);
        assert_eq!((found[1].start_line, found[1].end_line), (3, 6));
    }

    #[test]
    fn javascript_fonctions_et_classes() {
        let js = "const a = require('a');\nfunction f() {}\nexport const h = function () {};\nclass K {}\nmodule.exports = { f };\n";
        assert_eq!(
            ids(&sections(Language::JavaScript, js)),
            ["const:a", "fn:f", "fn:h", "class:K", "top:1"]
        );
    }

    #[test]
    fn python_decorateurs_et_commentaires_rattaches() {
        let py = "import os\n\n# Calcule.\n@cache\ndef f(x):\n    return x\n\nclass C:\n    def m(self):\n        pass\nprint(1)\n";
        let found = sections(Language::Python, py);
        assert_eq!(
            summary(&found),
            [
                ("top:1".to_owned(), 1, 1),
                ("fn:f".to_owned(), 3, 6),
                ("class:C".to_owned(), 8, 10),
                ("top:2".to_owned(), 11, 11),
            ]
        );
    }

    #[test]
    fn un_commentaire_separe_par_une_ligne_vide_reste_hors_element() {
        let found = sections(Language::Rust, "// Fichier.\n\nfn a() {}\n");
        assert_eq!(
            summary(&found),
            [("top:1".to_owned(), 1, 1), ("fn:a".to_owned(), 3, 3)]
        );
    }
}
