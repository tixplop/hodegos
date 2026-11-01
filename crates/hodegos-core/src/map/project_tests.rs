//! Carte d'un projet d'exemple : blocs, ordre, plafond, exclusions, mise à jour.

use std::collections::BTreeSet;
use std::fs;
use std::path::Path;

use super::*;
use crate::{Project, RelPath, UserId};

fn write(root: &Path, path: &str, content: &str) {
    let path = root.join(path);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, content).unwrap();
}

fn example() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let root = dir.path();
    write(root, ".gitignore", "target/\nsecret.txt\n");
    write(root, "target/debug/genere.rs", "pub fn genere() {}\n");
    write(root, "secret.txt", "mot de passe\n");
    write(root, ".env", "API_KEY=abcdef1234567\n");
    write(root, "config/prod.key", "clé\n");
    write(
        root,
        "config/cle.txt",
        "-----BEGIN RSA PRIVATE KEY-----\nabc\n",
    );
    write(
        root,
        "docs/vision.md",
        "# Troc\n\n## Idée\n<!-- Décrivez votre idée -->\nPrêter entre voisins.\ntoken = \"abcd1234efgh\"\n",
    );
    write(
        root,
        "docs/adr/0001-stockage.md",
        "# Stockage\n\nStatut : accepté\n\n## Décision\n\nUn seul module écrit data/emprunts.json.\n",
    );
    write(
        root,
        "README.md",
        "# Troc\n\nPrêter des objets.\n\n## Lancer\n",
    );
    write(
        root,
        "Cargo.toml",
        "[package]\nname = \"troc\"\nversion = \"0.1.0\"\n[dependencies]\nserde = \"1\"\n",
    );
    write(
        root,
        "src/lib.rs",
        "pub fn reserver(id: u32) -> bool {\n    id > 0\n}\n\n#[cfg(test)]\nmod tests {\n    #[test]\n    fn reserve() {}\n}\n",
    );
    write(root, "tests/flux.rs", "#[test]\nfn parcours_complet() {}\n");
    write(root, "data/emprunts.json", "[]\n");
    dir
}

fn map_of(root: &Path, settings: &MapSettings) -> (MapCache, ProjectMap) {
    let project = Project::open(root).unwrap();
    let session = project.session(UserId::local());
    let mut cache = MapCache::default();
    cache
        .refresh(&project, &session, &settings.exclude)
        .unwrap();
    let map = build_map(project.root(), &cache, settings);
    (cache, map)
}

#[test]
fn blocs_dans_l_ordre_sans_ignores_ni_secrets() {
    let dir = example();
    let (_, map) = map_of(dir.path(), &MapSettings::default());
    let ids: Vec<BlockId> = map.blocks.iter().map(|b| b.id).collect();
    assert_eq!(ids, BlockId::ALL.to_vec());
    let text = &map.text;
    let order: Vec<usize> = [
        "carte du projet",
        "## Vision",
        "## Décisions",
        "## Arborescence",
        "## README",
        "## Dépendances",
        "## Symboles",
    ]
    .iter()
    .map(|heading| {
        text.find(heading)
            .unwrap_or_else(|| panic!("{heading} absent"))
    })
    .collect();
    assert!(order.windows(2).all(|w| w[0] < w[1]), "{text}");

    // Tests désactivés par défaut ; l'en-tête cite l'outil de test.
    assert!(!text.contains("## Tests"));
    let tests = map.blocks.iter().find(|b| b.id == BlockId::Tests).unwrap();
    assert!(!tests.enabled);
    assert_eq!(tests.tokens, 0);
    assert!(text.contains("Tests : cargo test"));

    // Ignorés, secrets par leur nom ou leur contenu : jamais dans la carte.
    for absent in [
        "target",
        "genere",
        "secret.txt",
        ".env",
        "prod.key",
        "cle.txt",
        "abcdef1234567",
        "abcd1234efgh",
        "Décrivez",
    ] {
        assert!(!text.contains(absent), "{absent} dans la carte :\n{text}");
    }
    assert!(text.contains("[secret masqué]"));
    assert!(text.contains("Prêter entre voisins."));
    assert!(text.contains("docs/adr/0001-stockage.md — Stockage (accepté)"));
    assert!(text.contains("  Un seul module écrit data/emprunts.json."));
    assert!(text.contains("src/ : lib.rs"));
    assert!(text.contains("Cargo.toml (troc 0.1.0) : serde 1"));
    assert!(text.contains("  pub fn reserver(id: u32) -> bool"));
    assert!(!text.contains("id > 0"));
    assert_eq!(map.tokens, estimate_tokens(text));
    assert!(map.tokens <= map.cap);
}

#[test]
fn exclure_un_chemin_ou_desactiver_un_bloc_change_la_carte() {
    let dir = example();
    let (_, before) = map_of(dir.path(), &MapSettings::default());
    assert!(before.text.contains("data/ : emprunts.json"));

    let settings = MapSettings {
        disabled: BTreeSet::from([BlockId::Symbols]),
        exclude: Exclusions::new(&["data/**".to_owned()]).unwrap(),
        ..MapSettings::default()
    };
    let (_, after) = map_of(dir.path(), &settings);
    assert!(!after.text.contains("emprunts.json\n"));
    assert!(!after.text.contains("data/ :"));
    assert!(!after.text.contains("## Symboles"));
    assert!(after.text.contains("## Tests"));
    assert!(after.text.contains("tests/flux.rs : parcours_complet"));
    assert!(after.text.contains("src/lib.rs : reserve"));
    let symbols = after
        .blocks
        .iter()
        .find(|b| b.id == BlockId::Symbols)
        .unwrap();
    assert!(!symbols.enabled);
}

#[test]
fn au_dela_du_plafond_les_derniers_blocs_sont_raccourcis_par_lignes_entieres() {
    let dir = example();
    for index in 0..300 {
        write(
            dir.path(),
            &format!("src/module_{index:03}.rs"),
            &format!("pub fn fonction_{index}(entree: &str) -> Option<String> {{ None }}\n"),
        );
    }
    let full = MapSettings {
        cap: 1_000_000,
        ..MapSettings::default()
    };
    let (_, whole) = map_of(dir.path(), &full);
    let settings = MapSettings {
        cap: MIN_MAP_CAP,
        ..MapSettings::default()
    };
    let (_, map) = map_of(dir.path(), &settings);
    assert!(map.tokens <= MIN_MAP_CAP, "{} jetons", map.tokens);

    let symbols = map
        .blocks
        .iter()
        .find(|b| b.id == BlockId::Symbols)
        .unwrap();
    assert!(symbols.truncated);
    let vision = map.blocks.iter().find(|b| b.id == BlockId::Vision).unwrap();
    assert!(!vision.truncated);
    assert!(map.text.contains("Prêter entre voisins."));
    assert!(map.text.contains("fichiers non listés"));

    // Aucune ligne coupée : chaque ligne vient de la carte entière, sauf les mentions.
    let lines: BTreeSet<&str> = whole.text.lines().collect();
    for line in map.text.lines() {
        assert!(
            lines.contains(line) || line.starts_with("… "),
            "ligne coupée : {line}"
        );
    }
}

#[test]
fn seuls_les_fichiers_touches_sont_relus() {
    let dir = example();
    let root = dir.path();
    let project = Project::open(root).unwrap();
    let session = project.session(UserId::local());
    let exclude = Exclusions::default();
    let mut cache = MapCache::default();
    let first = cache.refresh(&project, &session, &exclude).unwrap();
    assert_eq!(first.extracted, cache.len());
    assert!(first.changed);

    // Rien n'a changé : rien n'est réanalysé.
    let again = cache.refresh(&project, &session, &exclude).unwrap();
    assert_eq!(again.extracted, 0);
    assert!(!again.changed);

    // Un fichier modifié : lui seul.
    write(root, "src/lib.rs", "pub fn annuler() {}\n");
    let rel = |p: &str| RelPath::parse(p).unwrap();
    let one = cache
        .refresh_paths(&project, &session, &exclude, &[rel("src/lib.rs")])
        .unwrap();
    assert_eq!(one.extracted, 1);
    let map = build_map(root, &cache, &MapSettings::default());
    assert!(map.text.contains("pub fn annuler()"));
    assert!(!map.text.contains("reserver"));

    // Un dossier apparu : ses fichiers entrent, il est à observer.
    write(
        root,
        "src/emprunts/annuler.rs",
        "pub fn annuler_tout() {}\n",
    );
    let added = cache
        .refresh_paths(&project, &session, &exclude, &[rel("src/emprunts")])
        .unwrap();
    assert_eq!(added.extracted, 1);
    assert_eq!(added.new_dirs, vec![rel("src/emprunts")]);

    // Un fichier supprimé, puis un dossier entier ignoré par un nouveau .gitignore.
    fs::remove_file(root.join("README.md")).unwrap();
    let removed = cache
        .refresh_paths(&project, &session, &exclude, &[rel("README.md")])
        .unwrap();
    assert!(removed.changed);
    assert_eq!(removed.extracted, 0);
    write(root, ".gitignore", "target/\nsecret.txt\nsrc/emprunts/\n");
    cache
        .refresh_paths(&project, &session, &exclude, &[rel(".gitignore")])
        .unwrap();
    let map = build_map(root, &cache, &MapSettings::default());
    assert!(!map.text.contains("README"));
    assert!(!map.text.contains("annuler_tout"));
    assert!(map.text.contains("aucun README") || !map.text.contains("## README"));
}

#[test]
fn le_cache_relu_evite_de_tout_reanalyser() {
    let dir = example();
    let project = Project::open(dir.path()).unwrap();
    let session = project.session(UserId::local());
    let exclude = Exclusions::default();
    let mut cache = MapCache::default();
    cache.refresh(&project, &session, &exclude).unwrap();
    let json = serde_json::to_string(&cache).unwrap();
    let mut reloaded: MapCache = serde_json::from_str(&json).unwrap();
    assert_eq!(reloaded, cache);
    let refresh = reloaded.refresh(&project, &session, &exclude).unwrap();
    assert_eq!(refresh.extracted, 0);
}
