//! Imports d'un fichier, relevés par tree-sitter (liens relatifs en Markdown), puis
//! résolus vers les fichiers du projet que connaît la carte. Seuls les imports qui
//! peuvent désigner un fichier du projet sont gardés : `mod`, `use crate::`,
//! `super::`, `self::` en Rust ; chemins relatifs en TypeScript, JavaScript et Python.

use std::collections::BTreeSet;
use std::sync::LazyLock;

use regex::Regex;
use tree_sitter::Node;

use crate::Language;

/// Liens Markdown `[texte](cible)` ; la cible s'arrête au premier blanc ou `)`.
#[allow(clippy::expect_used)]
static LINK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\]\(([^)\s]+)").expect("motif de lien valide"));

/// Imports bruts d'un fichier de code : `mod:x`, `use:crate::a::b`, `rel:./x`,
/// `py:..a.b:c,d`.
pub(crate) fn code_imports(language: Language, root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        let text = |n: &Node<'_>| n.utf8_text(source).unwrap_or_default().to_owned();
        match (language, node.kind()) {
            (Language::Rust, "mod_item") if node.child_by_field_name("body").is_none() => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.push(format!("mod:{}", text(&name)));
                }
                continue;
            }
            (Language::Rust, "use_declaration") => {
                if let Some(argument) = node.child_by_field_name("argument") {
                    let path = text(&argument);
                    if ["crate::", "super::", "self::"]
                        .iter()
                        .any(|p| path.starts_with(p))
                    {
                        out.push(format!("use:{path}"));
                    }
                }
                continue;
            }
            (
                Language::TypeScript | Language::JavaScript,
                "import_statement" | "export_statement",
            ) => {
                if let Some(source_node) = node.child_by_field_name("source") {
                    push_relative(&mut out, &text(&source_node));
                }
            }
            (Language::TypeScript | Language::JavaScript, "call_expression") => {
                let function = node
                    .child_by_field_name("function")
                    .map(|f| text(&f))
                    .unwrap_or_default();
                if function == "require" || function == "import" {
                    if let Some(argument) = node
                        .child_by_field_name("arguments")
                        .and_then(|a| a.named_child(0))
                        .filter(|a| a.kind() == "string")
                    {
                        push_relative(&mut out, &text(&argument));
                    }
                }
            }
            (Language::Python, "import_from_statement") => {
                if let Some(module) = node.child_by_field_name("module_name") {
                    let module = text(&module);
                    if module.starts_with('.') {
                        let mut names = Vec::new();
                        let mut cursor = node.walk();
                        for name in node.children_by_field_name("name", &mut cursor) {
                            let name = match name.kind() {
                                "aliased_import" => name
                                    .child_by_field_name("name")
                                    .map(|n| text(&n))
                                    .unwrap_or_default(),
                                _ => text(&name),
                            };
                            names.push(name);
                        }
                        out.push(format!("py:{module}:{}", names.join(",")));
                    }
                }
                continue;
            }
            _ => {}
        }
        let mut cursor = node.walk();
        let mut children: Vec<Node<'_>> = node.named_children(&mut cursor).collect();
        children.reverse();
        stack.extend(children);
    }
    out
}

fn push_relative(out: &mut Vec<String>, literal: &str) {
    let spec = literal.trim_matches(['\'', '"', '`']);
    if spec.starts_with("./") || spec.starts_with("../") {
        out.push(format!("rel:{spec}"));
    }
}

/// Liens relatifs d'un document Markdown : `link:docs/a.md#ancre`.
pub(crate) fn markdown_links(content: &str) -> Vec<String> {
    LINK.captures_iter(content)
        .filter_map(|c| c.get(1))
        .map(|m| m.as_str().trim_matches(['<', '>']))
        .filter(|target| {
            !target.starts_with('#')
                && !target.starts_with("//")
                && !target.contains("://")
                && !target.starts_with("mailto:")
        })
        .map(|target| format!("link:{target}"))
        .collect()
}

/// Cible d'un import résolu, avec l'ancre d'un lien Markdown (`#decision`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub(crate) path: String,
    pub(crate) anchor: Option<String>,
}

/// Fichiers du projet (`known`) désignés par l'import `spec` du fichier `from`.
pub(crate) fn resolve(from: &str, spec: &str, known: &BTreeSet<&str>) -> Option<Target> {
    let (kind, rest) = spec.split_once(':')?;
    let dir = parent(from);
    let first = |candidates: Vec<String>| {
        candidates
            .into_iter()
            .find(|c| c != from && known.contains(c.as_str()))
            .map(|path| Target { path, anchor: None })
    };
    match kind {
        "mod" => {
            let base = rust_children_dir(from);
            first(vec![
                join(&base, &format!("{rest}.rs")),
                join(&base, &format!("{rest}/mod.rs")),
            ])
        }
        "use" => rust_use(from, rest, known),
        "rel" => {
            let base = normalize(&join(&dir, rest))?;
            let stem = base.strip_suffix(".js").unwrap_or(&base);
            let mut candidates = vec![base.clone()];
            for extension in ["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
                candidates.push(format!("{stem}.{extension}"));
            }
            for extension in ["ts", "tsx", "js", "jsx"] {
                candidates.push(format!("{base}/index.{extension}"));
            }
            first(candidates)
        }
        "py" => {
            let (module, names) = rest.split_once(':').unwrap_or((rest, ""));
            let dots = module.chars().take_while(|c| *c == '.').count();
            let mut base = dir.clone();
            for _ in 1..dots {
                base = parent(&base);
            }
            let module = module[dots..].replace('.', "/");
            let package = if module.is_empty() {
                base
            } else {
                join(&base, &module)
            };
            let mut candidates = Vec::new();
            if !module.is_empty() {
                candidates.push(format!("{package}.py"));
                candidates.push(join(&package, "__init__.py"));
            }
            for name in names.split(',').filter(|n| !n.is_empty()) {
                candidates.push(join(&package, &format!("{name}.py")));
                candidates.push(join(&package, &format!("{name}/__init__.py")));
            }
            first(candidates)
        }
        "link" => {
            let (target, anchor) = match rest.split_once('#') {
                Some((target, anchor)) => (target, Some(anchor.to_owned())),
                None => (rest, None),
            };
            let target = target.split('?').next().unwrap_or(target);
            let target = target.replace("%20", " ");
            let path = if let Some(absolute) = target.strip_prefix('/') {
                normalize(absolute)?
            } else {
                normalize(&join(&dir, &target))?
            };
            (path != from && known.contains(path.as_str())).then_some(Target { path, anchor })
        }
        _ => None,
    }
}

/// `use crate::a::b::{c, d}` : le fichier du plus long préfixe de module trouvé.
fn rust_use(from: &str, path: &str, known: &BTreeSet<&str>) -> Option<Target> {
    let path = path.split(['{', '*']).next().unwrap_or(path);
    let mut segments: Vec<&str> = path
        .split("::")
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .collect();
    let head = segments.first().copied()?;
    segments.remove(0);
    let base = match head {
        "crate" => rust_crate_root(from, known)?,
        "self" => rust_children_dir(from),
        "super" => {
            let mut base = parent(&rust_children_dir(from));
            while segments.first() == Some(&"super") {
                segments.remove(0);
                base = parent(&base);
            }
            base
        }
        _ => return None,
    };
    for len in (1..=segments.len()).rev() {
        let module = segments[..len].join("/");
        for candidate in [
            join(&base, &format!("{module}.rs")),
            join(&base, &format!("{module}/mod.rs")),
        ] {
            if candidate != from && known.contains(candidate.as_str()) {
                return Some(Target {
                    path: candidate,
                    anchor: None,
                });
            }
        }
    }
    None
}

/// Dossier où vivent les sous-modules de `from` : celui de `lib.rs`, `main.rs`,
/// `mod.rs`, sinon `a/` pour `a.rs`.
fn rust_children_dir(from: &str) -> String {
    let name = from.rsplit('/').next().unwrap_or(from);
    if matches!(name, "lib.rs" | "main.rs" | "mod.rs") {
        parent(from)
    } else {
        from.strip_suffix(".rs").unwrap_or(from).to_owned()
    }
}

/// `src/` du crate qui contient `from` : le plus proche dossier parent avec un `Cargo.toml`.
fn rust_crate_root(from: &str, known: &BTreeSet<&str>) -> Option<String> {
    let mut dir = parent(from);
    loop {
        if known.contains(join(&dir, "Cargo.toml").as_str()) {
            return Some(join(&dir, "src"));
        }
        if dir.is_empty() {
            return None;
        }
        dir = parent(&dir);
    }
}

fn parent(path: &str) -> String {
    path.rsplit_once('/')
        .map(|(dir, _)| dir.to_owned())
        .unwrap_or_default()
}

fn join(dir: &str, name: &str) -> String {
    if dir.is_empty() {
        name.to_owned()
    } else {
        format!("{dir}/{name}")
    }
}

/// Résout `.` et `..` ; `None` si le chemin sort du projet.
fn normalize(path: &str) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            part => parts.push(part),
        }
    }
    Some(parts.join("/"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::map::extract::facts;

    fn known(paths: &[&'static str]) -> BTreeSet<&'static str> {
        paths.iter().copied().collect()
    }

    fn targets(from: &str, content: &str, paths: &[&'static str]) -> Vec<String> {
        let known = known(paths);
        facts(from, content)
            .imports
            .iter()
            .filter_map(|spec| resolve(from, spec, &known))
            .map(|t| t.path)
            .collect()
    }

    #[test]
    fn imports_rust() {
        let paths = [
            "Cargo.toml",
            "src/lib.rs",
            "src/stockage.rs",
            "src/emprunts/mod.rs",
            "src/emprunts/annuler.rs",
            "src/emprunts/reserver.rs",
        ];
        assert_eq!(
            targets(
                "src/lib.rs",
                "mod stockage;\npub mod emprunts;\nmod absent;\nmod tests { }\nuse std::fmt;\n",
                &paths
            ),
            vec!["src/stockage.rs", "src/emprunts/mod.rs"]
        );
        assert_eq!(
            targets(
                "src/emprunts/annuler.rs",
                "use crate::stockage::{lire, ecrire};\nuse super::reserver::reserver;\nuse self::interne::x;\n",
                &paths
            ),
            vec!["src/stockage.rs", "src/emprunts/reserver.rs"]
        );
        assert_eq!(
            targets(
                "src/emprunts/mod.rs",
                "pub mod annuler;\nuse super::stockage;\n",
                &paths
            ),
            vec!["src/emprunts/annuler.rs", "src/stockage.rs"]
        );
    }

    #[test]
    fn imports_typescript_et_javascript() {
        let paths = [
            "src/emprunts/reserver.ts",
            "src/stockage/depot.ts",
            "src/stockage/index.ts",
            "src/util.js",
        ];
        assert_eq!(
            targets(
                "src/emprunts/reserver.ts",
                "import { lister } from \"../stockage/depot\";\nimport * as s from '../stockage';\nimport express from 'express';\nexport { x } from '../util.js';\n",
                &paths
            ),
            vec![
                "src/stockage/depot.ts",
                "src/stockage/index.ts",
                "src/util.js"
            ]
        );
        assert_eq!(
            targets(
                "src/util.js",
                "const depot = require('./stockage/depot');\nconst fs = require('fs');\n",
                &paths
            ),
            vec!["src/stockage/depot.ts"]
        );
    }

    #[test]
    fn imports_python_relatifs() {
        let paths = [
            "app/taches.py",
            "app/stockage.py",
            "app/outils/__init__.py",
            "app/outils/dates.py",
            "modeles.py",
        ];
        assert_eq!(
            targets(
                "app/taches.py",
                "import os\nfrom . import stockage\nfrom .outils import dates\nfrom ..modeles import Tache\nfrom requests import get\n",
                &paths
            ),
            vec!["app/stockage.py", "app/outils/__init__.py", "modeles.py"]
        );
    }

    #[test]
    fn liens_markdown() {
        let paths = ["docs/vision.md", "docs/adr/0001-stockage.md", "src/a.rs"];
        let known = known(&paths);
        let links: Vec<Target> = facts(
            "docs/vision.md",
            "Voir [la décision](adr/0001-stockage.md#decision), [le code](../src/a.rs), [site](https://x.fr), [haut](#idee), [absent](rien.md).",
        )
        .imports
        .iter()
        .filter_map(|spec| resolve("docs/vision.md", spec, &known))
        .collect();
        assert_eq!(
            links,
            vec![
                Target {
                    path: "docs/adr/0001-stockage.md".to_owned(),
                    anchor: Some("decision".to_owned())
                },
                Target {
                    path: "src/a.rs".to_owned(),
                    anchor: None
                },
            ]
        );
        assert_eq!(normalize("../x"), None);
    }
}
