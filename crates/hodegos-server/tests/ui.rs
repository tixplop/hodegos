#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Le front ne charge aucune ressource externe : aucune URL `http(s)://` dans ui/,
//! hors du code empaqueté (ui/vendor/) et des espaces de noms XML.

use std::fs;
use std::path::{Path, PathBuf};

const ALLOWED: &[&str] = &["http://www.w3.org/"];

fn files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.is_dir() {
            if path.file_name().is_some_and(|n| n == "vendor") {
                continue;
            }
            files(&path, out);
        } else if path
            .extension()
            .and_then(|e| e.to_str())
            .is_some_and(|e| matches!(e, "html" | "css" | "js" | "svg"))
        {
            out.push(path);
        }
    }
}

#[test]
fn aucune_url_externe_dans_le_front() {
    let ui = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ui");
    let mut found = Vec::new();
    files(&ui, &mut found);
    assert!(found.len() > 5, "fichiers du front introuvables");

    let mut offending = Vec::new();
    for path in found {
        let text = fs::read_to_string(&path).unwrap();
        for (index, _) in text.match_indices("http") {
            let rest = &text[index..];
            if (rest.starts_with("http://") || rest.starts_with("https://"))
                && !ALLOWED.iter().any(|allowed| rest.starts_with(allowed))
            {
                let line = text[..index].lines().count();
                offending.push(format!("{}:{line}", path.display()));
            }
        }
    }
    assert!(offending.is_empty(), "URL externes : {offending:?}");
}
