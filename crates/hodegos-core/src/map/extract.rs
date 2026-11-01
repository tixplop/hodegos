//! Ce que la carte retient d'un fichier : signatures publiques, noms des tests,
//! décision d'un ADR, README, vision, manifeste. Jamais un corps de fonction.

use std::collections::BTreeMap;
use std::path::Path;

use serde::{Deserialize, Serialize};
use tree_sitter::{Node, Parser};

use crate::sections::{frontmatter, sections};
use crate::{Language, strip_html_comments};

/// Longueur maximale d'une signature, en caractères.
const MAX_SIGNATURE: usize = 160;

/// Ce que la carte retient d'un fichier ; vide si le fichier n'apporte rien.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub(crate) struct FileFacts {
    /// Signatures des éléments publics ou exportés ; les membres sont indentés.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) symbols: Vec<String>,
    /// Noms des tests du fichier.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) tests: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) decision: Option<Decision>,
    /// Lignes retenues d'un README ou de la vision.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) document: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) manifest: Option<Manifest>,
}

/// ADR : numéro, titre, statut et texte de la section Décision.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Decision {
    pub(crate) number: Option<String>,
    pub(crate) title: String,
    pub(crate) status: Option<String>,
    pub(crate) text: Vec<String>,
}

/// Manifeste : nom du paquet, dépendances (`nom version`) et outil de test révélé.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct Manifest {
    pub(crate) package: Option<String>,
    pub(crate) dependencies: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub(crate) dev_dependencies: Vec<String>,
    pub(crate) test_tool: Option<String>,
}

/// Rôle d'un fichier dans la carte, d'après son chemin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Role {
    Vision,
    Readme,
    Decision,
    Manifest,
    Code,
    Other,
}

pub(crate) fn role(path: &str) -> Role {
    let name = file_name(path);
    let lower = name.to_ascii_lowercase();
    if path == "docs/vision.md" {
        return Role::Vision;
    }
    if !path.contains('/') && (lower == "readme.md" || lower == "readme") {
        return Role::Readme;
    }
    if matches!(
        name,
        "Cargo.toml" | "package.json" | "pyproject.toml" | "requirements.txt"
    ) {
        return Role::Manifest;
    }
    let language = Language::of(path);
    if language == Language::Markdown {
        let in_adr = path.rsplit('/').nth(1) == Some("adr");
        if in_adr || path.starts_with("docs/architecture/") {
            return Role::Decision;
        }
        return Role::Other;
    }
    if matches!(
        language,
        Language::Rust | Language::TypeScript | Language::JavaScript | Language::Python
    ) {
        return Role::Code;
    }
    Role::Other
}

/// Fichier de test, d'après les conventions de son langage.
pub(crate) fn is_test_file(path: &str) -> bool {
    let name = file_name(path);
    let in_dir = |dirs: &[&str]| path.split('/').rev().skip(1).any(|d| dirs.contains(&d));
    match Language::of(path) {
        Language::Rust => in_dir(&["tests"]),
        Language::TypeScript | Language::JavaScript => {
            name.contains(".test.") || name.contains(".spec.") || in_dir(&["__tests__"])
        }
        Language::Python => {
            name.starts_with("test_") || name.ends_with("_test.py") || in_dir(&["tests"])
        }
        _ => false,
    }
}

fn file_name(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Faits d'un fichier lisible, déjà retenu par l'exploration (ni ignoré ni secret).
pub(crate) fn facts(path: &str, content: &str) -> FileFacts {
    match role(path) {
        Role::Vision => FileFacts {
            document: Some(vision(content)),
            ..FileFacts::default()
        },
        Role::Readme => FileFacts {
            document: Some(readme(content)),
            ..FileFacts::default()
        },
        Role::Decision => FileFacts {
            decision: Some(decision(path, content)),
            ..FileFacts::default()
        },
        Role::Manifest => FileFacts {
            manifest: manifest(path, content),
            ..FileFacts::default()
        },
        Role::Code => code(path, content),
        Role::Other => FileFacts::default(),
    }
}

/* ---------- Markdown ---------- */

/// Texte sans frontmatter ni commentaires HTML, lignes vides retirées.
fn document_lines(content: &str) -> Vec<String> {
    let body = match frontmatter(content) {
        Some((_, lines)) => content.lines().skip(lines).collect::<Vec<_>>().join("\n"),
        None => content.to_owned(),
    };
    strip_html_comments(&body)
        .lines()
        .map(str::trim_end)
        .filter(|line| !line.trim().is_empty())
        .map(str::to_owned)
        .collect()
}

fn vision(content: &str) -> Vec<String> {
    document_lines(content)
}

/// Titres (indentés selon leur niveau), puis premier paragraphe.
fn readme(content: &str) -> Vec<String> {
    let text = strip_html_comments(content);
    let mut lines: Vec<String> = Vec::new();
    if let Some(paragraph) = first_paragraph(&text) {
        lines.push(paragraph);
    }
    for section in sections(Language::Markdown, &text) {
        if section.kind == "h" {
            let indent = "  ".repeat(section.parents.len());
            lines.push(format!("{indent}- {}", section.name));
        }
    }
    lines
}

/// Premier bloc de texte qui n'est ni un titre, ni une image, ni un frontmatter.
fn first_paragraph(text: &str) -> Option<String> {
    let skipped = frontmatter(text).map_or(0, |(_, lines)| lines);
    let mut paragraph: Vec<&str> = Vec::new();
    for line in text.lines().skip(skipped) {
        let trimmed = line.trim();
        let skip = trimmed.starts_with('#')
            || trimmed.starts_with("![")
            || trimmed.starts_with("[![")
            || trimmed.starts_with("===")
            || trimmed.starts_with("---");
        if trimmed.is_empty() || skip {
            if !paragraph.is_empty() {
                break;
            }
            continue;
        }
        paragraph.push(trimmed);
    }
    (!paragraph.is_empty()).then(|| paragraph.join(" "))
}

fn decision(path: &str, content: &str) -> Decision {
    let text = strip_html_comments(content);
    let all = sections(Language::Markdown, &text);
    let title = all
        .iter()
        .find(|s| s.kind == "h" && s.parents.is_empty())
        .map(|s| s.name.clone())
        .unwrap_or_else(|| file_name(path).trim_end_matches(".md").to_owned());
    let stem = file_name(path);
    let digits: String = stem.chars().take_while(char::is_ascii_digit).collect();
    let number = (!digits.is_empty()).then_some(digits);

    let normalized = |name: &str| crate::sections::slug(name);
    let body = |name: &[&str]| {
        all.iter()
            .find(|s| s.kind == "h" && name.contains(&normalized(&s.name).as_str()))
            .map(|s| {
                s.text(&text)
                    .lines()
                    .skip(1)
                    .filter(|line| !line.trim().is_empty())
                    .map(|line| line.trim_end().to_owned())
                    .collect::<Vec<_>>()
            })
    };
    let status = text
        .lines()
        .find_map(status_line)
        .or_else(|| body(&["statut", "status"]).and_then(|lines| lines.into_iter().next()));
    Decision {
        number,
        title,
        status,
        text: body(&["decision", "decisions"]).unwrap_or_default(),
    }
}

/// `Statut : accepté · 1er novembre 2026` → `accepté`.
fn status_line(line: &str) -> Option<String> {
    let trimmed = line.trim().trim_start_matches("**");
    let lower = trimmed.to_lowercase();
    let rest = ["statut", "status"]
        .iter()
        .find_map(|word| lower.starts_with(word).then(|| &trimmed[word.len()..]))?;
    let value = rest
        .trim_start_matches("**")
        .trim_start()
        .strip_prefix(':')?
        .trim();
    let value = value.split(" · ").next().unwrap_or(value).trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/* ---------- Manifestes ---------- */

fn manifest(path: &str, content: &str) -> Option<Manifest> {
    match file_name(path) {
        "Cargo.toml" => cargo(content),
        "package.json" => package_json(content),
        "pyproject.toml" => pyproject(content),
        "requirements.txt" => Some(requirements(content)),
        _ => None,
    }
}

fn cargo(content: &str) -> Option<Manifest> {
    let value: toml::Table = toml::from_str(content).ok()?;
    let package = value.get("package").and_then(|p| {
        let name = p.get("name")?.as_str()?;
        Some(match p.get("version").and_then(|v| v.as_str()) {
            Some(version) => format!("{name} {version}"),
            None => name.to_owned(),
        })
    });
    let table = |value: Option<&toml::Value>| -> Vec<String> {
        value
            .and_then(|t| t.as_table())
            .map(|t| {
                t.iter()
                    .map(|(name, spec)| match cargo_version(spec) {
                        Some(version) => format!("{name} {version}"),
                        None => name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let mut dependencies = table(value.get("dependencies"));
    dependencies.extend(table(
        value.get("workspace").and_then(|w| w.get("dependencies")),
    ));
    Some(Manifest {
        package,
        dependencies,
        dev_dependencies: table(value.get("dev-dependencies")),
        test_tool: Some("cargo test".to_owned()),
    })
}

fn cargo_version(spec: &toml::Value) -> Option<String> {
    if let Some(version) = spec.as_str() {
        return Some(version.to_owned());
    }
    let table = spec.as_table()?;
    if let Some(version) = table.get("version").and_then(|v| v.as_str()) {
        return Some(version.to_owned());
    }
    if table.get("workspace").and_then(|v| v.as_bool()) == Some(true) {
        return Some("(workspace)".to_owned());
    }
    table.get("path").map(|_| "(chemin)".to_owned())
}

fn package_json(content: &str) -> Option<Manifest> {
    let value: serde_json::Value = serde_json::from_str(content).ok()?;
    let entries = |key: &str| -> Vec<String> {
        value
            .get(key)
            .and_then(|d| d.as_object())
            .map(|d| {
                d.iter()
                    .map(|(name, version)| match version.as_str() {
                        Some(version) => format!("{name} {version}"),
                        None => name.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let package = value.get("name").and_then(|n| n.as_str()).map(|name| {
        match value.get("version").and_then(|v| v.as_str()) {
            Some(version) => format!("{name} {version}"),
            None => name.to_owned(),
        }
    });
    let script = value
        .get("scripts")
        .and_then(|s| s.get("test"))
        .and_then(|t| t.as_str())
        .unwrap_or_default();
    let dev = entries("devDependencies");
    let dependencies = entries("dependencies");
    let known = ["vitest", "jest", "mocha", "ava"];
    let test_tool = known
        .iter()
        .find(|tool| script.contains(*tool))
        .or_else(|| {
            known.iter().find(|tool| {
                dev.iter()
                    .chain(&dependencies)
                    .any(|d| d.split(' ').next() == Some(tool))
            })
        })
        .map(|tool| (*tool).to_owned())
        .or_else(|| {
            script
                .contains("node --test")
                .then(|| "node --test".to_owned())
        });
    Some(Manifest {
        package,
        dependencies,
        dev_dependencies: dev,
        test_tool,
    })
}

fn pyproject(content: &str) -> Option<Manifest> {
    let value: toml::Table = toml::from_str(content).ok()?;
    let project = value.get("project");
    let package = project
        .and_then(|p| p.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_owned);
    let mut dependencies: Vec<String> = project
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_array())
        .map(|d| {
            d.iter()
                .filter_map(|v| v.as_str().map(str::to_owned))
                .collect()
        })
        .unwrap_or_default();
    let poetry = value
        .get("tool")
        .and_then(|t| t.get("poetry"))
        .and_then(|p| p.get("dependencies"))
        .and_then(|d| d.as_table());
    if let Some(poetry) = poetry {
        dependencies.extend(poetry.iter().map(|(name, spec)| match spec.as_str() {
            Some(version) => format!("{name} {version}"),
            None => name.clone(),
        }));
    }
    let pytest = dependencies.iter().any(|d| d.starts_with("pytest"))
        || value.get("tool").and_then(|t| t.get("pytest")).is_some();
    Some(Manifest {
        package,
        dependencies,
        dev_dependencies: Vec::new(),
        test_tool: pytest.then(|| "pytest".to_owned()),
    })
}

fn requirements(content: &str) -> Manifest {
    let dependencies: Vec<String> = content
        .lines()
        .map(|line| line.split('#').next().unwrap_or_default().trim())
        .filter(|line| !line.is_empty() && !line.starts_with('-'))
        .map(str::to_owned)
        .collect();
    let pytest = dependencies.iter().any(|d| d.starts_with("pytest"));
    Manifest {
        package: None,
        dependencies,
        dev_dependencies: Vec::new(),
        test_tool: pytest.then(|| "pytest".to_owned()),
    }
}

/* ---------- Code ---------- */

fn code(path: &str, content: &str) -> FileFacts {
    let language = Language::of(path);
    let grammar: tree_sitter::Language = match language {
        Language::Rust => tree_sitter_rust::LANGUAGE.into(),
        Language::TypeScript => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        Language::JavaScript => tree_sitter_javascript::LANGUAGE.into(),
        Language::Python => tree_sitter_python::LANGUAGE.into(),
        _ => return FileFacts::default(),
    };
    let mut parser = Parser::new();
    if parser.set_language(&grammar).is_err() {
        return FileFacts::default();
    }
    let Some(tree) = parser.parse(content, None) else {
        return FileFacts::default();
    };
    let root = tree.root_node();
    let source = content.as_bytes();
    let symbols = if is_test_file(path) {
        Vec::new()
    } else {
        match language {
            Language::Rust => rust_symbols(&root, source),
            Language::Python => python_symbols(&root, source),
            _ => script_symbols(&root, source),
        }
    };
    let tests = match language {
        Language::Rust => rust_tests(&root, source),
        Language::Python => python_tests(&root, source),
        _ => script_tests(&root, source),
    };
    FileFacts {
        symbols,
        tests,
        ..FileFacts::default()
    }
}

fn text<'a>(node: &Node<'_>, source: &'a [u8]) -> &'a str {
    node.utf8_text(source).unwrap_or_default()
}

fn named_children<'t>(node: &Node<'t>) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// Texte de `node` jusqu'au début de `until` (son corps), sur une ligne, sans les
/// signes qui ouvrent le corps.
fn signature(node: &Node<'_>, until: Option<Node<'_>>, source: &[u8]) -> String {
    let end = until.map_or(node.end_byte(), |n| n.start_byte());
    let raw = std::str::from_utf8(&source[node.start_byte()..end.max(node.start_byte())])
        .unwrap_or_default();
    let line = raw.split_whitespace().collect::<Vec<_>>().join(" ");
    let line = line.trim_end_matches(['{', '=', ' ']).trim_end();
    let line = line.strip_suffix("=>").map_or(line, str::trim_end);
    shorten(line)
}

fn shorten(line: &str) -> String {
    if line.chars().count() <= MAX_SIGNATURE {
        return line.to_owned();
    }
    let cut: String = line.chars().take(MAX_SIGNATURE - 1).collect();
    format!("{cut}…")
}

/// Signature d'un élément : jusqu'à son corps ; une constante s'arrête à sa valeur.
fn item_signature(node: &Node<'_>, source: &[u8]) -> String {
    let until = node
        .child_by_field_name("body")
        .or_else(|| node.child_by_field_name("value"));
    signature(node, until, source)
}

/* Rust */

fn is_pub(node: &Node<'_>, source: &[u8]) -> bool {
    named_children(node)
        .iter()
        .any(|c| c.kind() == "visibility_modifier" && text(c, source) == "pub")
}

fn rust_symbols(root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for node in named_children(root) {
        match node.kind() {
            "function_item"
            | "function_signature_item"
            | "struct_item"
            | "enum_item"
            | "union_item"
            | "trait_item"
            | "type_item"
            | "const_item"
            | "static_item"
            | "mod_item"
                if is_pub(&node, source) =>
            {
                out.push(item_signature(&node, source));
            }
            "impl_item" => {
                let trait_impl = node.child_by_field_name("trait").is_some();
                let methods: Vec<String> = node
                    .child_by_field_name("body")
                    .map(|body| named_children(&body))
                    .unwrap_or_default()
                    .iter()
                    .filter(|m| m.kind() == "function_item")
                    .filter(|m| trait_impl || is_pub(m, source))
                    .map(|m| format!("  {}", item_signature(m, source)))
                    .collect();
                if trait_impl || !methods.is_empty() {
                    out.push(item_signature(&node, source));
                    out.extend(methods);
                }
            }
            _ => {}
        }
    }
    out
}

fn rust_tests(root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        if node.kind() == "function_item" && rust_test_attribute(&node, source) {
            if let Some(name) = node.child_by_field_name("name") {
                out.push(text(&name, source).to_owned());
            }
            continue;
        }
        let mut children = named_children(&node);
        children.reverse();
        stack.extend(children);
    }
    out
}

/// `#[test]`, `#[tokio::test]`… juste au-dessus de la fonction (commentaires sautés).
fn rust_test_attribute(node: &Node<'_>, source: &[u8]) -> bool {
    let mut previous = node.prev_named_sibling();
    while let Some(sibling) = previous {
        match sibling.kind() {
            "attribute_item" => {
                let attribute = text(&sibling, source);
                if attribute.contains("test]") || attribute.contains("test(") {
                    return true;
                }
            }
            "line_comment" | "block_comment" => {}
            _ => return false,
        }
        previous = sibling.prev_named_sibling();
    }
    false
}

/* TypeScript, JavaScript */

fn script_symbols(root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for node in named_children(root) {
        match node.kind() {
            "export_statement" => match node.child_by_field_name("declaration") {
                Some(declaration) => script_declaration(&node, &declaration, source, &mut out),
                None => {
                    let until = node
                        .child_by_field_name("value")
                        .and_then(|v| v.child_by_field_name("body"));
                    out.push(signature(&node, until, source));
                }
            },
            "expression_statement" => {
                let Some(assignment) = node
                    .named_child(0)
                    .filter(|n| n.kind() == "assignment_expression")
                else {
                    continue;
                };
                let left = assignment
                    .child_by_field_name("left")
                    .map(|l| text(&l, source))
                    .unwrap_or_default();
                if left == "module.exports" || left.starts_with("exports.") {
                    let until = assignment
                        .child_by_field_name("right")
                        .and_then(|r| r.child_by_field_name("body"));
                    out.push(signature(&node, until, source));
                }
            }
            _ => {}
        }
    }
    out
}

/// Déclaration exportée : la signature part du mot `export`.
fn script_declaration(
    export: &Node<'_>,
    declaration: &Node<'_>,
    source: &[u8],
    out: &mut Vec<String>,
) {
    match declaration.kind() {
        "lexical_declaration" | "variable_declaration" => {
            let declarator = declaration
                .named_child(0)
                .filter(|n| n.kind() == "variable_declarator");
            let value = declarator.and_then(|d| d.child_by_field_name("value"));
            let until = match value {
                Some(v) if v.kind().contains("function") => v.child_by_field_name("body"),
                Some(v) => Some(v),
                None => None,
            };
            out.push(signature(export, until, source));
        }
        "class_declaration" | "abstract_class_declaration" => {
            let body = declaration.child_by_field_name("body");
            out.push(signature(export, body, source));
            for member in body.map(|b| named_children(&b)).unwrap_or_default() {
                if member.kind() != "method_definition" {
                    continue;
                }
                let name = member
                    .child_by_field_name("name")
                    .map(|n| text(&n, source))
                    .unwrap_or_default();
                let hidden = name.starts_with('#')
                    || name.starts_with('_')
                    || named_children(&member).iter().any(|c| {
                        c.kind() == "accessibility_modifier" && text(c, source) != "public"
                    });
                if !hidden {
                    out.push(format!("  {}", item_signature(&member, source)));
                }
            }
        }
        "type_alias_declaration" => out.push(signature(export, None, source)),
        _ => {
            let until = declaration.child_by_field_name("body");
            out.push(signature(export, until, source));
        }
    }
}

fn script_tests(root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    let mut stack = vec![*root];
    while let Some(node) = stack.pop() {
        if node.kind() == "call_expression" {
            let function = node
                .child_by_field_name("function")
                .map(|f| text(&f, source))
                .unwrap_or_default();
            let base = function.split('.').next().unwrap_or_default();
            if base == "test" || base == "it" {
                let name = node
                    .child_by_field_name("arguments")
                    .and_then(|a| a.named_child(0))
                    .filter(|a| matches!(a.kind(), "string" | "template_string"))
                    .map(|a| text(&a, source).trim_matches(['\'', '"', '`']).to_owned());
                if let Some(name) = name {
                    out.push(name);
                }
            }
        }
        let mut children = named_children(&node);
        children.reverse();
        stack.extend(children);
    }
    out
}

/* Python */

fn python_definition<'t>(node: &Node<'t>) -> Option<Node<'t>> {
    match node.kind() {
        "function_definition" | "class_definition" => Some(*node),
        "decorated_definition" => node.child_by_field_name("definition"),
        _ => None,
    }
}

fn python_name<'a>(node: &Node<'_>, source: &'a [u8]) -> &'a str {
    node.child_by_field_name("name")
        .map(|n| text(&n, source))
        .unwrap_or_default()
}

fn python_symbols(root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for node in named_children(root) {
        let Some(definition) = python_definition(&node) else {
            continue;
        };
        if python_name(&definition, source).starts_with('_') {
            continue;
        }
        out.push(item_signature(&definition, source));
        if definition.kind() != "class_definition" {
            continue;
        }
        for member in definition
            .child_by_field_name("body")
            .map(|b| named_children(&b))
            .unwrap_or_default()
        {
            let Some(method) = python_definition(&member) else {
                continue;
            };
            let name = python_name(&method, source);
            if method.kind() == "function_definition"
                && (!name.starts_with('_') || name == "__init__")
            {
                out.push(format!("  {}", item_signature(&method, source)));
            }
        }
    }
    out
}

fn python_tests(root: &Node<'_>, source: &[u8]) -> Vec<String> {
    let mut out = Vec::new();
    for node in named_children(root) {
        let Some(definition) = python_definition(&node) else {
            continue;
        };
        let name = python_name(&definition, source);
        if definition.kind() == "function_definition" && name.starts_with("test") {
            out.push(name.to_owned());
        } else if definition.kind() == "class_definition" && name.starts_with("Test") {
            for member in definition
                .child_by_field_name("body")
                .map(|b| named_children(&b))
                .unwrap_or_default()
            {
                if let Some(method) = python_definition(&member) {
                    let method_name = python_name(&method, source);
                    if method_name.starts_with("test") {
                        out.push(format!("{name}.{method_name}"));
                    }
                }
            }
        }
    }
    out
}

/// Langages et nombre de fichiers, pour l'en-tête.
pub(crate) fn language_counts<'a>(paths: impl Iterator<Item = &'a str>) -> Vec<(String, usize)> {
    let mut counts: BTreeMap<&'static str, usize> = BTreeMap::new();
    for path in paths {
        let language = Language::of(path);
        if language != Language::Text {
            *counts.entry(language.as_str()).or_default() += 1;
        }
    }
    let mut sorted: Vec<(String, usize)> = counts
        .into_iter()
        .map(|(name, count)| (name.to_owned(), count))
        .collect();
    sorted.sort_by(|a, b| b.1.cmp(&a.1).then_with(|| a.0.cmp(&b.0)));
    sorted
}

/// Nom du dossier racine, pour le titre de la carte.
pub(crate) fn project_name(root: &Path) -> String {
    root.file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("projet")
        .to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signatures_rust_publiques_sans_corps() {
        let source = r#"
/// Doc.
pub fn reserver(objet: &str, debut: u32) -> Result<(), Erreur> {
    let secret_interne = 42;
    Ok(())
}
fn privee() {}
pub(crate) fn interne() {}
pub struct Point { pub x: i32 }
pub const MAX: usize = 1000;
impl Point {
    pub fn new(x: i32) -> Self { Self { x } }
    fn cache(&self) {}
}
impl std::fmt::Display for Point {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { Ok(()) }
}
#[cfg(test)]
mod tests {
    #[test]
    fn reserve_un_objet() {}
    #[tokio::test]
    async fn en_parallele() {}
    fn aide() {}
}
"#;
        let facts = facts("src/lib.rs", source);
        assert_eq!(
            facts.symbols,
            vec![
                "pub fn reserver(objet: &str, debut: u32) -> Result<(), Erreur>",
                "pub struct Point",
                "pub const MAX: usize",
                "impl Point",
                "  pub fn new(x: i32) -> Self",
                "impl std::fmt::Display for Point",
                "  fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result",
            ]
        );
        assert!(!facts.symbols.iter().any(|s| s.contains("secret_interne")));
        assert_eq!(facts.tests, vec!["reserve_un_objet", "en_parallele"]);
    }

    #[test]
    fn signatures_typescript_exportees() {
        let source = r#"
import { lister } from "../stockage/depot";
export function reserver(objet: Objet, debut: Date): Emprunt {
  return lister().find((e) => e.debut === debut);
}
function interne() {}
export const annuler = (id: string): void => { supprimer(id); };
export const LIMITE = 3;
export interface Emprunt { id: string }
export type Id = string;
export class Depot {
  lire(id: string): Emprunt { return x; }
  private ecrire() {}
  #cache() {}
}
export { interne as aussi };
"#;
        let facts = facts("src/emprunts/reserver.ts", source);
        assert_eq!(
            facts.symbols,
            vec![
                "export function reserver(objet: Objet, debut: Date): Emprunt",
                "export const annuler = (id: string): void",
                "export const LIMITE",
                "export interface Emprunt",
                "export type Id = string;",
                "export class Depot",
                "  lire(id: string): Emprunt",
                "export { interne as aussi };",
            ]
        );
        assert!(facts.tests.is_empty());
    }

    #[test]
    fn tests_javascript_et_fichier_de_test_sans_symboles() {
        let source = r#"
import { reserver } from "./reserver";
export function aide() {}
describe("reserver", () => {
  test("refuse un chevauchement", () => {});
  it.each([1])('accepte %s', () => {});
});
"#;
        let facts = facts("src/emprunts/reserver.test.js", source);
        assert!(facts.symbols.is_empty());
        assert_eq!(facts.tests, vec!["refuse un chevauchement", "accepte %s"]);
        assert!(!facts.symbols.iter().any(|s| s.contains("module")));
        let exports = facts_of_js("module.exports = function lire(x) { return x; };\n");
        assert_eq!(exports, vec!["module.exports = function lire(x)"]);
    }

    fn facts_of_js(source: &str) -> Vec<String> {
        facts("lib/a.js", source).symbols
    }

    #[test]
    fn signatures_python_publiques() {
        let source = r#"
def lister(taches: list) -> list:
    return [t for t in taches if not t["terminee"]]

def _aide():
    pass

@dataclass
class Tache:
    def __init__(self, titre):
        self.titre = titre
    def terminer(self) -> None:
        self.terminee = True
    def _interne(self):
        pass

class TestTache:
    def test_terminer(self):
        pass

def test_lister():
    pass
"#;
        let facts = facts("app/taches.py", source);
        assert_eq!(
            facts.symbols,
            vec![
                "def lister(taches: list) -> list:",
                "class Tache:",
                "  def __init__(self, titre):",
                "  def terminer(self) -> None:",
                "class TestTache:",
                "  def test_terminer(self):",
                "def test_lister():",
            ]
        );
        assert_eq!(facts.tests, vec!["TestTache.test_terminer", "test_lister"]);
    }

    #[test]
    fn adr_numero_titre_statut_et_decision() {
        let source = "# ADR 0001 — Stockage\n\nStatut : accepté · 1er novembre 2026\n\n## Contexte\n\nOn hésite.\n\n## Décision\n\nUn seul module, `src/stockage/depot.ts`,\nlit et écrit `data/emprunts.json`.\n<!-- note pour moi -->\n\n## Conséquences\n\nRien.\n";
        let decision = facts("docs/adr/0001-stockage.md", source).decision.unwrap();
        assert_eq!(decision.number.as_deref(), Some("0001"));
        assert_eq!(decision.title, "ADR 0001 — Stockage");
        assert_eq!(decision.status.as_deref(), Some("accepté"));
        assert_eq!(
            decision.text,
            vec![
                "Un seul module, `src/stockage/depot.ts`,",
                "lit et écrit `data/emprunts.json`."
            ]
        );
        assert_eq!(role("docs/architecture/vue.md"), Role::Decision);
        assert_eq!(role("docs/notes.md"), Role::Other);
    }

    #[test]
    fn vision_sans_commentaires_et_readme() {
        let vision = facts(
            "docs/vision.md",
            "---\ntitre: x\n---\n# Troc\n\n## Idée\n<!-- Décrivez l'idée -->\nPrêter entre voisins.\n",
        );
        assert_eq!(
            vision.document.unwrap(),
            vec!["# Troc", "## Idée", "Prêter entre voisins."]
        );
        let readme = facts(
            "README.md",
            "# Troc\n\n[![ci](x)](y)\n\nPrêter des objets\nentre voisins.\n\nSuite.\n\n## Installer\n\n### Linux\n",
        );
        assert_eq!(
            readme.document.unwrap(),
            vec![
                "Prêter des objets entre voisins.",
                "- Troc",
                "  - Installer",
                "    - Linux"
            ]
        );
        assert_eq!(role("docs/README.md"), Role::Other);
    }

    #[test]
    fn manifestes_et_outil_de_test() {
        let cargo = manifest(
            "Cargo.toml",
            "[package]\nname = \"troc\"\nversion = \"0.1.0\"\n[dependencies]\nserde = { version = \"1\", features = [\"derive\"] }\nanyhow = \"1.0\"\nlocal = { path = \"../local\" }\n[dev-dependencies]\ntempfile = \"3\"\n",
        )
        .unwrap();
        assert_eq!(cargo.package.as_deref(), Some("troc 0.1.0"));
        assert_eq!(
            cargo.dependencies,
            vec!["anyhow 1.0", "local (chemin)", "serde 1"]
        );
        assert_eq!(cargo.dev_dependencies, vec!["tempfile 3"]);
        assert_eq!(cargo.test_tool.as_deref(), Some("cargo test"));

        let npm = manifest(
            "package.json",
            r#"{"name":"troc","dependencies":{"express":"^5.0.0"},"devDependencies":{"vitest":"^3.0.0"},"scripts":{"test":"vitest run"}}"#,
        )
        .unwrap();
        assert_eq!(npm.dependencies, vec!["express ^5.0.0"]);
        assert_eq!(npm.test_tool.as_deref(), Some("vitest"));

        let python = manifest(
            "pyproject.toml",
            "[project]\nname = \"troc\"\ndependencies = [\"httpx>=0.27\"]\n[tool.pytest.ini_options]\n",
        )
        .unwrap();
        assert_eq!(python.dependencies, vec!["httpx>=0.27"]);
        assert_eq!(python.test_tool.as_deref(), Some("pytest"));

        let requirements = manifest(
            "requirements.txt",
            "# commentaire\nrequests==2.32\n-r dev.txt\npytest\n",
        )
        .unwrap();
        assert_eq!(requirements.dependencies, vec!["requests==2.32", "pytest"]);
        assert_eq!(requirements.test_tool.as_deref(), Some("pytest"));
        assert!(manifest("Cargo.toml", "pas du toml = = =").is_none());
    }

    #[test]
    fn fichiers_de_test_par_convention() {
        assert!(is_test_file("tests/passes.rs"));
        assert!(is_test_file("crates/x/tests/a.rs"));
        assert!(!is_test_file("src/lib.rs"));
        assert!(is_test_file("src/a.test.ts"));
        assert!(is_test_file("src/__tests__/a.js"));
        assert!(is_test_file("test_taches.py"));
        assert!(is_test_file("app/taches_test.py"));
        assert!(!is_test_file("app/taches.py"));
    }
}
