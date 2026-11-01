//! Voisinage d'un fichier (phase 9b, ADR 0009), calculé sans modèle à partir de ce que
//! sait la carte : fichiers importés, fichiers qui l'importent, test associé, documents
//! qui le citent, même dossier. Un fichier de code part en signatures, un document par
//! la section concernée ; mêmes exclusions et même masquage que la carte.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::extract::{FileFacts, facts, is_test_file};
use super::imports::{Target, resolve};
use super::{Exclusions, MapCache, estimate_tokens};
use crate::secrets::{exclusion, mask_secrets};
use crate::sections::{sections, slug};
use crate::{Language, Project, RelPath, Session, strip_html_comments};

/// Plafond par défaut du voisinage, en jetons estimés.
pub const DEFAULT_NEIGHBORHOOD_CAP: usize = 3_000;

/// Lien entre un voisin et le fichier relu, par ordre de priorité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    /// Importé par le fichier relu.
    Imported,
    /// Document lié depuis le fichier relu.
    Linked,
    /// Importe le fichier relu.
    Importer,
    /// Test associé (ou tests du même fichier).
    Test,
    /// Document qui cite le chemin ou le dossier du fichier relu.
    CitedBy,
    /// Même dossier.
    SameDir,
}

impl Relation {
    /// Ce que le voisin est au fichier relu, pour le prompt et l'interface.
    pub fn label(self) -> &'static str {
        match self {
            Self::Imported => "importé par ce fichier",
            Self::Linked => "lié depuis ce fichier",
            Self::Importer => "importe ce fichier",
            Self::Test => "test associé",
            Self::CitedBy => "cite ce fichier",
            Self::SameDir => "même dossier",
        }
    }
}

/// Un voisin tel qu'envoyé : `text` est déjà masqué.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Neighbor {
    pub path: String,
    pub relation: Relation,
    /// Forme envoyée : « signatures », « noms des tests », « section Décision », « titres ».
    pub form: String,
    pub text: String,
    pub tokens: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Neighborhood {
    pub neighbors: Vec<Neighbor>,
    /// Jetons estimés de l'ensemble.
    pub tokens: usize,
    /// Voisins écartés faute de place sous le plafond.
    pub skipped: usize,
}

/// Voisinage de `path` (contenu enregistré : `content`), sous `cap` jetons estimés.
/// Un fichier exclu de la carte (ignoré, secret, motif exclu) n'a pas de voisinage ;
/// un fichier admis que la carte ne connaît pas encore (tout juste créé) en a un.
pub fn neighborhood(
    project: &Project,
    session: &Session,
    cache: &MapCache,
    exclude: &Exclusions,
    path: &str,
    content: &str,
    cap: usize,
) -> Neighborhood {
    let mut known: BTreeSet<&str> = cache.paths().collect();
    if !known.contains(path) {
        let admitted =
            RelPath::parse(path).is_ok_and(|rel| project.map_admits(&rel, false, exclude));
        if !admitted || exclusion(path, content).is_some() {
            return Neighborhood::default();
        }
    }
    known.insert(path);
    let own = facts(path, content);
    let mut seen: BTreeSet<String> = BTreeSet::from([path.to_owned()]);
    let mut candidates: Vec<(String, Relation, Option<String>)> = Vec::new();
    let push =
        |candidates: &mut Vec<_>, seen: &mut BTreeSet<String>, target: String, relation, anchor| {
            if seen.insert(target.clone()) {
                candidates.push((target, relation, anchor));
            }
        };

    // 1. Ce qu'il importe (ou lie, pour un document).
    for spec in &own.imports {
        if let Some(Target {
            path: target,
            anchor,
        }) = resolve(path, spec, &known)
        {
            let relation = if Language::of(&target) == Language::Markdown {
                Relation::Linked
            } else {
                Relation::Imported
            };
            push(&mut candidates, &mut seen, target, relation, anchor);
        }
    }
    // 2. Ce qui l'importe (code seulement : un document qui le cite vient en 4).
    for (other, f) in cache.facts() {
        if Language::of(other) == Language::Markdown || other == path {
            continue;
        }
        if f.imports
            .iter()
            .any(|spec| resolve(other, spec, &known).is_some_and(|t| t.path == path))
        {
            push(
                &mut candidates,
                &mut seen,
                other.to_owned(),
                Relation::Importer,
                None,
            );
        }
    }
    // 3. Test associé.
    let mut same_file_tests = None;
    if !is_test_file(path) && !own.tests.is_empty() {
        same_file_tests = Some(own.tests.clone());
    }
    for test in test_candidates(path, &known) {
        push(&mut candidates, &mut seen, test, Relation::Test, None);
    }
    // 4. Documents qui citent son chemin, puis son dossier.
    let dir = path.rsplit_once('/').map(|(d, _)| d).unwrap_or_default();
    let documents: Vec<&str> = known
        .iter()
        .copied()
        .filter(|p| Language::of(p) == Language::Markdown && !seen.contains(*p))
        .collect();
    let mut citing: Vec<(String, String)> = Vec::new();
    let mut by_dir: Vec<(String, String)> = Vec::new();
    for document in documents {
        let Some(text) = read(project, session, document) else {
            continue;
        };
        let text = strip_html_comments(&text);
        if let Some(section) = citing_section(&text, path) {
            citing.push((document.to_owned(), section));
        } else if !dir.is_empty()
            && let Some(section) = citing_section(&text, &format!("{dir}/"))
        {
            by_dir.push((document.to_owned(), section));
        }
    }
    let cited: Vec<(String, String)> = citing.into_iter().chain(by_dir).collect();
    for (document, _) in &cited {
        seen.insert(document.clone());
    }
    // 5. Même dossier.
    let siblings: Vec<String> = known
        .iter()
        .filter(|p| {
            let parent = p.rsplit_once('/').map(|(d, _)| d).unwrap_or_default();
            parent == dir && !seen.contains(**p)
        })
        .map(|p| (*p).to_owned())
        .collect();

    // Mise en forme, dans l'ordre de priorité, jusqu'au plafond.
    let mut out = Neighborhood::default();
    let add = |out: &mut Neighborhood, neighbor: Option<Neighbor>| {
        let Some(neighbor) = neighbor else { return };
        if out.tokens + neighbor.tokens > cap {
            out.skipped += 1;
            return;
        }
        out.tokens += neighbor.tokens;
        out.neighbors.push(neighbor);
    };
    let facts_of = |p: &str| cache.facts().find(|(q, _)| *q == p).map(|(_, f)| f.clone());
    let before_tests: Vec<_> = candidates
        .iter()
        .filter(|(_, r, _)| *r != Relation::Test)
        .cloned()
        .collect();
    for (target, relation, anchor) in &before_tests {
        add(
            &mut out,
            render(
                project,
                session,
                target,
                *relation,
                anchor.as_deref(),
                facts_of(target),
            ),
        );
    }
    if let Some(tests) = same_file_tests {
        add(
            &mut out,
            Some(neighbor(
                path,
                Relation::Test,
                "tests du même fichier",
                &format!("tests : {}", tests.join(" · ")),
            )),
        );
    }
    for (target, relation, _) in candidates.iter().filter(|(_, r, _)| *r == Relation::Test) {
        add(
            &mut out,
            render(project, session, target, *relation, None, facts_of(target)),
        );
    }
    for (document, section) in &cited {
        add(
            &mut out,
            Some(neighbor(
                document,
                Relation::CitedBy,
                &section_form(section),
                section,
            )),
        );
    }
    for sibling in &siblings {
        add(
            &mut out,
            render(
                project,
                session,
                sibling,
                Relation::SameDir,
                None,
                facts_of(sibling),
            ),
        );
    }
    out
}

fn neighbor(path: &str, relation: Relation, form: &str, text: &str) -> Neighbor {
    let text = mask_secrets(text).0;
    Neighbor {
        path: path.to_owned(),
        relation,
        form: form.to_owned(),
        tokens: estimate_tokens(&text) + estimate_tokens(path) + 8,
        text,
    }
}

/// « section Décision » d'après la première ligne (le titre) d'une section citée.
fn section_form(section: &str) -> String {
    let title = section
        .lines()
        .next()
        .unwrap_or_default()
        .trim_start_matches('#')
        .trim();
    if title.is_empty() {
        "section".to_owned()
    } else {
        format!("section « {title} »")
    }
}

/// Code : signatures, sinon noms des tests ; document : section visée par l'ancre,
/// sinon titres. Rien à montrer : pas de voisin.
fn render(
    project: &Project,
    session: &Session,
    path: &str,
    relation: Relation,
    anchor: Option<&str>,
    facts: Option<FileFacts>,
) -> Option<Neighbor> {
    let facts = facts?;
    if Language::of(path) == Language::Markdown {
        let text = strip_html_comments(&read(project, session, path)?);
        let all = sections(Language::Markdown, &text);
        if let Some(anchor) = anchor
            && let Some(section) = all
                .iter()
                .find(|s| s.kind == "h" && slug(&s.name) == slug(anchor))
        {
            let body = compact(&section.text(&text));
            return Some(neighbor(
                path,
                relation,
                &format!("section « {} »", section.name),
                &body,
            ));
        }
        let titles: Vec<String> = all
            .iter()
            .filter(|s| s.kind == "h")
            .map(|s| format!("{}- {}", "  ".repeat(s.parents.len()), s.name))
            .collect();
        return (!titles.is_empty())
            .then(|| neighbor(path, relation, "titres", &titles.join("\n")));
    }
    if !facts.symbols.is_empty() {
        return Some(neighbor(
            path,
            relation,
            "signatures",
            &facts.symbols.join("\n"),
        ));
    }
    if !facts.tests.is_empty() {
        return Some(neighbor(
            path,
            relation,
            "noms des tests",
            &format!("tests : {}", facts.tests.join(" · ")),
        ));
    }
    None
}

/// Première section d'un document qui contient `needle`, lignes vides retirées.
fn citing_section(text: &str, needle: &str) -> Option<String> {
    if !text.contains(needle) {
        return None;
    }
    sections(Language::Markdown, text)
        .iter()
        .map(|s| s.text(text))
        .find(|body| body.contains(needle))
        .map(|body| compact(&body))
}

fn compact(text: &str) -> String {
    text.lines()
        .map(str::trim_end)
        .filter(|l| !l.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

fn read(project: &Project, session: &Session, path: &str) -> Option<String> {
    let rel = RelPath::parse(path).ok()?;
    project.read_file(session, &rel).ok().map(|f| f.content)
}

/// Fichiers de test par convention du langage, s'ils existent.
fn test_candidates(path: &str, known: &BTreeSet<&str>) -> Vec<String> {
    if is_test_file(path) {
        return Vec::new();
    }
    let (dir, name) = path.rsplit_once('/').unwrap_or(("", path));
    let join = |d: &str, n: &str| {
        if d.is_empty() {
            n.to_owned()
        } else {
            format!("{d}/{n}")
        }
    };
    let stem = name.rsplit_once('.').map_or(name, |(stem, _)| stem);
    let mut candidates = Vec::new();
    match Language::of(path) {
        Language::TypeScript | Language::JavaScript => {
            for kind in ["test", "spec"] {
                for ext in ["ts", "tsx", "js", "jsx", "mjs", "cjs"] {
                    candidates.push(join(dir, &format!("{stem}.{kind}.{ext}")));
                    candidates.push(join(
                        &join(dir, "__tests__"),
                        &format!("{stem}.{kind}.{ext}"),
                    ));
                }
            }
        }
        Language::Python => {
            for d in [dir.to_owned(), join(dir, "tests"), "tests".to_owned()] {
                candidates.push(join(&d, &format!("test_{stem}.py")));
                candidates.push(join(&d, &format!("{stem}_test.py")));
            }
        }
        Language::Rust => {
            // `tests/<nom>.rs` du crate qui contient le fichier.
            let mut crate_dir = dir.to_owned();
            loop {
                if known.contains(join(&crate_dir, "Cargo.toml").as_str()) {
                    candidates.push(join(&join(&crate_dir, "tests"), &format!("{stem}.rs")));
                    break;
                }
                if crate_dir.is_empty() {
                    break;
                }
                crate_dir = crate_dir
                    .rsplit_once('/')
                    .map(|(d, _)| d.to_owned())
                    .unwrap_or_default();
            }
        }
        _ => {}
    }
    candidates
        .into_iter()
        .filter(|c| known.contains(c.as_str()))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::Path;

    use super::*;
    use crate::UserId;

    fn write(root: &Path, path: &str, content: &str) {
        let path = root.join(path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, content).unwrap();
    }

    const ANNULER: &str = "import { lister, enregistrer } from \"../stockage/depot\";\nimport { readFileSync } from \"fs\";\n\nexport function annuler(id: string): void {\n  const brut = readFileSync(\"data/emprunts.json\");\n}\n";

    fn example() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        write(root, ".gitignore", "secret/\n");
        write(root, "src/emprunts/annuler.ts", ANNULER);
        write(
            root,
            "src/emprunts/reserver.ts",
            "import { lister } from \"../stockage/depot\";\nexport function reserver(objet: string): boolean {\n  return lister().length > 0;\n}\n",
        );
        write(
            root,
            "src/emprunts/reserver.test.ts",
            "test(\"refuse un chevauchement\", () => {});\n",
        );
        write(
            root,
            "src/emprunts/annuler.test.ts",
            "import { annuler } from \"./annuler\";\ntest(\"annule un emprunt\", () => {});\n",
        );
        write(
            root,
            "src/stockage/depot.ts",
            "export function lister(): Emprunt[] {\n  return JSON.parse(lire());\n}\n",
        );
        write(
            root,
            "src/notifications/email.ts",
            "import { annuler } from \"../emprunts/annuler\";\nexport function prevenir(): void {}\n",
        );
        write(
            root,
            "secret/espion.ts",
            "import { annuler } from \"../src/emprunts/annuler\";\nexport function espion() {}\n",
        );
        write(
            root,
            "docs/notes.md",
            "# Notes\n\n## Intro\n\nRien.\n\n## Annulation\n\nVoir src/emprunts/annuler.ts.\npassword = \"tres-secret-99\"\n\n## Fin\n\nRien.\n",
        );
        write(
            root,
            "docs/architecture/0001-stockage.md",
            "# Stockage\n\n## Décision\n\nUn seul module, src/stockage/depot.ts.\n",
        );
        dir
    }

    fn around(root: &Path, path: &str, cap: usize) -> Neighborhood {
        let project = Project::open(root).unwrap();
        let session = project.session(UserId::local());
        let mut cache = MapCache::default();
        cache
            .refresh(&project, &session, &Exclusions::default())
            .unwrap();
        let content = fs::read_to_string(root.join(path)).unwrap();
        neighborhood(
            &project,
            &session,
            &cache,
            &Exclusions::default(),
            path,
            &content,
            cap,
        )
    }

    #[test]
    fn voisins_par_ordre_de_priorite_en_signatures() {
        let dir = example();
        let around = around(
            dir.path(),
            "src/emprunts/annuler.ts",
            DEFAULT_NEIGHBORHOOD_CAP,
        );
        let found: Vec<(&str, Relation, &str)> = around
            .neighbors
            .iter()
            .map(|n| (n.path.as_str(), n.relation, n.form.as_str()))
            .collect();
        assert_eq!(
            found,
            vec![
                ("src/stockage/depot.ts", Relation::Imported, "signatures"),
                (
                    "src/emprunts/annuler.test.ts",
                    Relation::Importer,
                    "noms des tests"
                ),
                (
                    "src/notifications/email.ts",
                    Relation::Importer,
                    "signatures"
                ),
                ("docs/notes.md", Relation::CitedBy, "section « Annulation »"),
                (
                    "src/emprunts/reserver.test.ts",
                    Relation::SameDir,
                    "noms des tests"
                ),
                ("src/emprunts/reserver.ts", Relation::SameDir, "signatures"),
            ]
        );
        let depot = &around.neighbors[0];
        assert_eq!(depot.text, "export function lister(): Emprunt[]");
        let notes = &around.neighbors[3];
        assert!(notes.text.starts_with("## Annulation"), "{}", notes.text);
        assert!(notes.text.contains("[secret masqué]"));
        assert!(!notes.text.contains("tres-secret-99"));
        assert!(!notes.text.contains("Rien."));
        let all: String = around.neighbors.iter().map(|n| n.text.clone()).collect();
        assert!(!all.contains("JSON.parse"), "jamais les corps");
        assert!(
            !around
                .neighbors
                .iter()
                .any(|n| n.path.starts_with("secret/"))
        );
        assert_eq!(
            around.tokens,
            around.neighbors.iter().map(|n| n.tokens).sum::<usize>()
        );
    }

    #[test]
    fn le_plafond_garde_les_premiers_voisins() {
        let dir = example();
        let around = around(dir.path(), "src/emprunts/annuler.ts", 40);
        assert!(around.tokens <= 40);
        assert!(around.skipped > 0);
        assert_eq!(around.neighbors[0].path, "src/stockage/depot.ts");
    }

    #[test]
    fn test_associe_document_lie_et_fichier_exclu() {
        let dir = example();
        let root = dir.path();
        // Le test par convention passe avant le même dossier.
        let around_reserver = around(root, "src/emprunts/reserver.ts", DEFAULT_NEIGHBORHOOD_CAP);
        assert!(around_reserver.neighbors.iter().any(|n| {
            n.path == "src/emprunts/reserver.test.ts" && n.relation == Relation::Test
        }));

        // Rust : tests du même fichier et `tests/<nom>.rs` du crate.
        write(root, "Cargo.toml", "[package]\nname = \"x\"\n");
        write(
            root,
            "src/lib.rs",
            "pub fn a() {}\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn a_marche() {}\n}\n",
        );
        write(root, "tests/lib.rs", "#[test]\nfn parcours() {}\n");
        let rust = around(root, "src/lib.rs", DEFAULT_NEIGHBORHOOD_CAP);
        let tests: Vec<(&str, &str)> = rust
            .neighbors
            .iter()
            .filter(|n| n.relation == Relation::Test)
            .map(|n| (n.path.as_str(), n.text.as_str()))
            .collect();
        assert_eq!(
            tests,
            vec![
                ("src/lib.rs", "tests : a_marche"),
                ("tests/lib.rs", "tests : parcours")
            ]
        );

        // Document lié avec une ancre : la section visée.
        write(
            root,
            "docs/vision.md",
            "# Troc\n\nVoir [la décision](architecture/0001-stockage.md#decision).\n",
        );
        let vision = around(root, "docs/vision.md", DEFAULT_NEIGHBORHOOD_CAP);
        let linked = &vision.neighbors[0];
        assert_eq!(linked.relation, Relation::Linked);
        assert_eq!(linked.form, "section « Décision »");
        assert!(linked.text.contains("Un seul module"));

        // Fichier hors de la carte (ignoré) : aucun voisinage.
        assert_eq!(
            around(root, "secret/espion.ts", DEFAULT_NEIGHBORHOOD_CAP),
            Neighborhood::default()
        );
        // Fichier créé après le calcul de la carte : il a quand même son voisinage.
        let project = Project::open(root).unwrap();
        let session = project.session(UserId::local());
        let mut cache = MapCache::default();
        cache
            .refresh(&project, &session, &Exclusions::default())
            .unwrap();
        let fresh = "import { lister } from \"../stockage/depot\";\n";
        write(root, "src/emprunts/nouveau.ts", fresh);
        let late = neighborhood(
            &project,
            &session,
            &cache,
            &Exclusions::default(),
            "src/emprunts/nouveau.ts",
            fresh,
            DEFAULT_NEIGHBORHOOD_CAP,
        );
        assert_eq!(late.neighbors[0].path, "src/stockage/depot.ts");
    }
}
