//! Texte de la carte, bloc par bloc, sous son plafond. Au-delà, les blocs sont
//! raccourcis du dernier au premier, élément entier par élément entier, et un bloc
//! raccourci dit ce qu'il a laissé (« … 12 fichiers non listés »).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::extract::{Decision, FileFacts, Role, language_counts, project_name, role};
use super::{BlockId, MAX_MAP_FILES, MapCache, MapSettings, tokens_of};
use crate::secrets::mask_secrets;

/// Carte telle qu'envoyée, avec le détail de chaque bloc.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectMap {
    pub blocks: Vec<MapBlock>,
    /// Jetons estimés du texte entier.
    pub tokens: usize,
    pub cap: usize,
    /// Texte envoyé aux mentors, secrets masqués.
    pub text: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MapBlock {
    pub id: BlockId,
    pub enabled: bool,
    /// Jetons estimés du bloc tel qu'envoyé (0 s'il est désactivé ou vide).
    pub tokens: usize,
    pub truncated: bool,
    /// Ce que contient le bloc, pour l'interface.
    pub detail: String,
}

/// Élément qu'on retire d'un bloc d'un seul tenant (un fichier, une décision, une ligne).
struct Item {
    lines: Vec<String>,
    /// Nombre d'unités qu'il représente (fichiers d'un dossier).
    count: usize,
}

impl Item {
    fn chars(&self) -> usize {
        self.lines.iter().map(|l| l.chars().count() + 1).sum()
    }
}

struct Draft {
    id: BlockId,
    title: String,
    items: Vec<Item>,
    /// Unités retirées, et comment le dire : (singulier, pluriel).
    unit: (&'static str, &'static str),
    omitted: usize,
    detail: String,
}

impl Draft {
    fn new(id: BlockId, title: impl Into<String>, unit: (&'static str, &'static str)) -> Self {
        Self {
            id,
            title: title.into(),
            items: Vec::new(),
            unit,
            omitted: 0,
            detail: String::new(),
        }
    }

    fn push(&mut self, lines: Vec<String>, count: usize) {
        let lines = lines.iter().map(|line| mask_secrets(line).0).collect();
        self.items.push(Item { lines, count });
    }

    fn omission(&self) -> Option<String> {
        match self.omitted {
            0 => None,
            1 => Some(format!("… 1 {}", self.unit.0)),
            n => Some(format!("… {n} {}", self.unit.1)),
        }
    }

    /// Texte du bloc ; l'en-tête n'a pas de titre de section.
    fn render(&self) -> String {
        let mut out = String::new();
        if self.id != BlockId::Header {
            out.push_str(&format!("\n## {}\n", self.title));
        }
        for item in &self.items {
            for line in &item.lines {
                out.push_str(line);
                out.push('\n');
            }
        }
        if let Some(omission) = self.omission() {
            out.push_str(&omission);
            out.push('\n');
        }
        out
    }

    fn chars(&self) -> usize {
        let heading = if self.id == BlockId::Header {
            0
        } else {
            self.title.chars().count() + 5
        };
        let omission = self.omission().map_or(0, |o| o.chars().count() + 1);
        heading + self.items.iter().map(Item::chars).sum::<usize>() + omission
    }

    fn is_empty(&self) -> bool {
        self.items.is_empty() && self.omitted == 0
    }
}

/// Construit la carte du projet de racine `root` d'après ce que sait `cache`.
pub fn build_map(root: &Path, cache: &MapCache, settings: &MapSettings) -> ProjectMap {
    let drafts = drafts(root, cache);
    let mut active: Vec<Draft> = Vec::new();
    let mut blocks: Vec<MapBlock> = Vec::new();
    for draft in drafts {
        let enabled = draft.id == BlockId::Header || !settings.disabled.contains(&draft.id);
        blocks.push(MapBlock {
            id: draft.id,
            enabled,
            tokens: 0,
            truncated: false,
            detail: draft.detail.clone(),
        });
        if enabled && !draft.is_empty() {
            active.push(draft);
        }
    }

    // Raccourcis du dernier bloc vers le premier ; l'en-tête reste entier.
    let mut total: usize = active.iter().map(Draft::chars).sum();
    for index in (0..active.len()).rev() {
        if tokens_of(total) <= settings.cap || active[index].id == BlockId::Header {
            continue;
        }
        let draft = &mut active[index];
        while tokens_of(total) > settings.cap {
            let before = draft.chars();
            let Some(item) = draft.items.pop() else { break };
            draft.omitted += item.count;
            total = total - before + draft.chars();
        }
    }

    let mut text = String::new();
    for draft in &active {
        let rendered = draft.render();
        if let Some(block) = blocks.iter_mut().find(|b| b.id == draft.id) {
            block.tokens = tokens_of(rendered.chars().count());
            block.truncated = draft.omitted > 0;
        }
        text.push_str(&rendered);
    }
    ProjectMap {
        blocks,
        tokens: tokens_of(text.chars().count()),
        cap: settings.cap,
        text,
    }
}

fn plural(count: usize, one: &str, many: &str) -> String {
    if count == 1 {
        format!("1 {one}")
    } else {
        format!("{count} {many}")
    }
}

fn drafts(root: &Path, cache: &MapCache) -> Vec<Draft> {
    let facts: Vec<(&str, &FileFacts)> = cache.facts().collect();
    let with_role = |wanted: Role| {
        facts
            .iter()
            .filter(move |(path, _)| role(path) == wanted)
            .copied()
    };

    // En-tête.
    let mut header = Draft::new(BlockId::Header, "", ("ligne", "lignes"));
    let languages = language_counts(cache.paths())
        .iter()
        .map(|(name, count)| format!("{name} {count}"))
        .collect::<Vec<_>>()
        .join(", ");
    let tools: BTreeSet<&str> = facts
        .iter()
        .filter_map(|(_, f)| f.manifest.as_ref()?.test_tool.as_deref())
        .collect();
    let mut summary = format!(
        "Fichiers : {}",
        if cache.truncated() {
            format!("{} (exploration limitée à {MAX_MAP_FILES})", cache.len())
        } else {
            cache.len().to_string()
        }
    );
    if !languages.is_empty() {
        summary = format!("Langages : {languages} · {summary}");
    }
    if !tools.is_empty() {
        let tools: Vec<&str> = tools.into_iter().collect();
        summary.push_str(&format!(" · Tests : {}", tools.join(", ")));
    }
    header.push(
        vec![
            format!(
                "# {} — carte du projet (calculée par Hodegos)",
                project_name(root)
            ),
            summary,
        ],
        1,
    );
    header.detail = "nom, langages, nombre de fichiers, outil de test".to_owned();

    // Vision.
    let mut vision = Draft::new(
        BlockId::Vision,
        "Vision (docs/vision.md)",
        ("ligne non listée", "lignes non listées"),
    );
    match with_role(Role::Vision).find_map(|(_, f)| f.document.as_ref()) {
        Some(lines) => {
            for line in lines {
                vision.push(vec![line.clone()], 1);
            }
            vision.detail = "docs/vision.md, sans les commentaires".to_owned();
        }
        None => vision.detail = "aucune vision (docs/vision.md)".to_owned(),
    }

    // Décisions.
    let mut decisions = Draft::new(
        BlockId::Decisions,
        "Décisions",
        ("décision non listée", "décisions non listées"),
    );
    let mut adrs: Vec<(&str, &Decision)> = with_role(Role::Decision)
        .filter_map(|(path, f)| Some((path, f.decision.as_ref()?)))
        .collect();
    adrs.sort_by(|a, b| a.1.number.cmp(&b.1.number).then_with(|| a.0.cmp(b.0)));
    for (path, adr) in &adrs {
        let status = adr
            .status
            .as_ref()
            .map(|s| format!(" ({s})"))
            .unwrap_or_default();
        let mut lines = vec![format!("{} — {}{status}", path, adr.title)];
        if adr.text.is_empty() {
            lines.push("  (pas de section Décision)".to_owned());
        }
        lines.extend(adr.text.iter().map(|line| format!("  {}", line.trim())));
        decisions.push(lines, 1);
    }
    decisions.detail = if adrs.is_empty() {
        "aucune décision (**/adr/*.md, docs/architecture/**)".to_owned()
    } else {
        format!(
            "{} : titre, statut, section Décision",
            plural(adrs.len(), "décision", "décisions")
        )
    };

    // Arborescence.
    let mut tree = Draft::new(
        BlockId::Tree,
        "Arborescence",
        ("fichier non listé", "fichiers non listés"),
    );
    let mut by_dir: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for path in cache.paths() {
        let (dir, name) = path.rsplit_once('/').unwrap_or((".", path));
        by_dir.entry(dir).or_default().push(name);
    }
    for (dir, names) in &by_dir {
        tree.push(vec![format!("{dir}/ : {}", names.join(", "))], names.len());
    }
    tree.detail = format!(
        "{} non ignorés, regroupés par dossier",
        plural(cache.len(), "fichier", "fichiers")
    );

    // README.
    let readme_file = with_role(Role::Readme).find(|(_, f)| f.document.is_some());
    let mut readme = Draft::new(
        BlockId::Readme,
        match readme_file {
            Some((path, _)) => format!("README ({path})"),
            None => "README".to_owned(),
        },
        ("ligne non listée", "lignes non listées"),
    );
    match readme_file.and_then(|(path, f)| Some((path, f.document.as_ref()?))) {
        Some((path, lines)) => {
            for line in lines {
                readme.push(vec![line.clone()], 1);
            }
            readme.detail = format!("{path} : titres et premier paragraphe");
        }
        None => readme.detail = "aucun README à la racine".to_owned(),
    }

    // Dépendances.
    let mut dependencies = Draft::new(
        BlockId::Dependencies,
        "Dépendances",
        ("manifeste non listé", "manifestes non listés"),
    );
    let mut kinds: BTreeSet<&str> = BTreeSet::new();
    for (path, manifest) in
        with_role(Role::Manifest).filter_map(|(p, f)| Some((p, f.manifest.as_ref()?)))
    {
        kinds.insert(path.rsplit('/').next().unwrap_or(path));
        let package = manifest
            .package
            .as_ref()
            .map(|p| format!(" ({p})"))
            .unwrap_or_default();
        let mut lines = vec![format!(
            "{path}{package} : {}",
            if manifest.dependencies.is_empty() {
                "aucune dépendance".to_owned()
            } else {
                manifest.dependencies.join(", ")
            }
        )];
        if !manifest.dev_dependencies.is_empty() {
            lines.push(format!("  dev : {}", manifest.dev_dependencies.join(", ")));
        }
        dependencies.push(lines, 1);
    }
    dependencies.detail = if kinds.is_empty() {
        "aucun manifeste".to_owned()
    } else {
        format!(
            "{} : noms et versions",
            kinds.into_iter().collect::<Vec<_>>().join(", ")
        )
    };

    // Symboles.
    let mut symbols = Draft::new(
        BlockId::Symbols,
        "Symboles",
        ("fichier non listé", "fichiers non listés"),
    );
    let mut symbol_files = 0;
    for (path, f) in facts.iter().filter(|(_, f)| !f.symbols.is_empty()) {
        let mut lines = vec![(*path).to_owned()];
        lines.extend(f.symbols.iter().map(|s| format!("  {s}")));
        symbols.push(lines, 1);
        symbol_files += 1;
    }
    symbols.detail = format!(
        "signatures publiques de {}, jamais les corps",
        plural(symbol_files, "fichier", "fichiers")
    );

    // Tests.
    let mut tests = Draft::new(
        BlockId::Tests,
        "Tests",
        ("fichier non listé", "fichiers non listés"),
    );
    let mut test_count = 0;
    let mut test_files = 0;
    for (path, f) in facts.iter().filter(|(_, f)| !f.tests.is_empty()) {
        tests.push(vec![format!("{path} : {}", f.tests.join(" · "))], 1);
        test_count += f.tests.len();
        test_files += 1;
    }
    tests.detail = if test_files == 0 {
        "aucun test trouvé".to_owned()
    } else {
        format!(
            "{} dans {} (noms seulement)",
            plural(test_count, "test", "tests"),
            plural(test_files, "fichier", "fichiers")
        )
    };

    vec![
        header,
        vision,
        decisions,
        tree,
        readme,
        dependencies,
        symbols,
        tests,
    ]
}
