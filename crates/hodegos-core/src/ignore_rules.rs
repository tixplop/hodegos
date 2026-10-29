use std::path::{Path, PathBuf};

use ignore::Match;
use ignore::gitignore::{Gitignore, GitignoreBuilder};

use crate::RelPath;

/// Règles `.gitignore` qui s'appliquent dans un dossier du projet : celles de la racine
/// (avec `.git/info/exclude`), puis celles de chaque dossier jusqu'à lui.
///
/// Les fichiers `.gitignore` sont lus même hors d'un dépôt git. Les règles globales
/// de git (`core.excludesFile`) ne sont pas lues.
pub(crate) struct IgnoreRules {
    matchers: Vec<Gitignore>,
}

impl IgnoreRules {
    pub(crate) fn for_dir(root: &Path, dir: &RelPath) -> Self {
        let mut matchers = Vec::new();
        let mut current = root.to_path_buf();
        push_matcher(
            &mut matchers,
            &current,
            &[
                current.join(".git/info/exclude"),
                current.join(".gitignore"),
            ],
        );
        for part in dir.components() {
            current.push(part);
            push_matcher(&mut matchers, &current, &[current.join(".gitignore")]);
        }
        Self { matchers }
    }

    /// `path` doit être sous le dossier pour lequel les règles ont été construites.
    /// La règle la plus profonde l'emporte, comme dans git.
    pub(crate) fn is_ignored(&self, path: &Path, is_dir: bool) -> bool {
        let mut ignored = false;
        for matcher in &self.matchers {
            match matcher.matched_path_or_any_parents(path, is_dir) {
                Match::Ignore(_) => ignored = true,
                Match::Whitelist(_) => ignored = false,
                Match::None => {}
            }
        }
        ignored
    }
}

fn push_matcher(matchers: &mut Vec<Gitignore>, dir: &Path, files: &[PathBuf]) {
    let mut builder = GitignoreBuilder::new(dir);
    let mut any = false;
    for file in files.iter().filter(|f| f.is_file()) {
        if let Some(err) = builder.add(file) {
            tracing::warn!(file = %file.display(), error = %err, "règle d'ignorance illisible");
        }
        any = true;
    }
    if !any {
        return;
    }
    match builder.build() {
        Ok(matcher) => matchers.push(matcher),
        Err(err) => {
            tracing::warn!(dir = %dir.display(), error = %err, "règles d'ignorance illisibles")
        }
    }
}
