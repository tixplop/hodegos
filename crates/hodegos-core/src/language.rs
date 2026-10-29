use std::path::Path;

/// Langage d'un fichier, déduit de son extension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Rust,
    TypeScript,
    JavaScript,
    Python,
    Markdown,
    /// Tout autre fichier texte, ouvert sans coloration.
    Text,
}

impl Language {
    pub fn of(path: &str) -> Self {
        let extension = Path::new(path)
            .extension()
            .and_then(|e| e.to_str())
            .map(str::to_ascii_lowercase);
        match extension.as_deref() {
            Some("rs") => Self::Rust,
            Some("ts" | "tsx" | "mts" | "cts") => Self::TypeScript,
            Some("js" | "jsx" | "mjs" | "cjs") => Self::JavaScript,
            Some("py" | "pyi") => Self::Python,
            Some("md" | "markdown") => Self::Markdown,
            _ => Self::Text,
        }
    }

    /// Nom employé par le protocole et par les mentors (`triggers.languages`).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Rust => "rust",
            Self::TypeScript => "typescript",
            Self::JavaScript => "javascript",
            Self::Python => "python",
            Self::Markdown => "markdown",
            Self::Text => "text",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deduit_le_langage_de_l_extension() {
        for (path, language) in [
            ("src/main.rs", Language::Rust),
            ("src/emprunts/reserver.ts", Language::TypeScript),
            ("ui/Vue.tsx", Language::TypeScript),
            ("ui/app.js", Language::JavaScript),
            ("outil.mjs", Language::JavaScript),
            ("script.py", Language::Python),
            ("docs/adr/0001.md", Language::Markdown),
            ("LISEZMOI.MD", Language::Markdown),
            ("Cargo.toml", Language::Text),
            ("Makefile", Language::Text),
        ] {
            assert_eq!(Language::of(path), language, "{path}");
        }
    }
}
