//! Ce qui ne part jamais vers le modèle : fichiers sensibles exclus, lignes de
//! secrets masquées.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

/// Ligne qui remplace une ligne contenant un secret.
pub const MASK: &str = "[secret masqué]";

/// Pourquoi un fichier n'est jamais envoyé.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Exclusion {
    /// `.env*`, `*.pem`, `*.key`, clé SSH (`id_rsa`, `id_ed25519.pub`…).
    SensitiveName,
    /// Bloc `-----BEGIN … PRIVATE KEY-----`.
    PrivateKey,
}

#[allow(clippy::expect_used)]
fn regex(pattern: &str) -> Regex {
    // Motifs constants, vérifiés par les tests.
    Regex::new(pattern).expect("motif de secret valide")
}

static PRIVATE_KEY: LazyLock<Regex> =
    LazyLock::new(|| regex(r"-----BEGIN [A-Z0-9 ]*PRIVATE KEY-----"));

/// Jetons reconnaissables et affectations d'une valeur littérale à un nom de secret.
static SECRET_LINE: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    [
        r"\b(AKIA|ASIA)[0-9A-Z]{16}\b",
        r"\bgh[pousr]_[A-Za-z0-9]{36,}\b",
        r"\bgithub_pat_[A-Za-z0-9_]{22,}\b",
        r"\bsk-[A-Za-z0-9_-]{20,}",
        r"\bxox[abprs]-[A-Za-z0-9-]{10,}",
        r#"(?i)(password|passwd|pwd|secret|token|api[_-]?key|access[_-]?key|private[_-]?key)["']?\s*[:=]\s*["'][^"'\s]{4,}["']"#,
        // Variable d'environnement en majuscules, comme dans un fichier .env.
        r"^\s*(export\s+)?[A-Z0-9_]*(PASSWORD|SECRET|TOKEN|API_?KEY)[A-Z0-9_]*\s*=\s*[^\s$]{4,}",
    ]
    .iter()
    .map(|p| regex(p))
    .collect()
});

/// Raison d'exclure un fichier, d'après son nom et son contenu.
pub fn exclusion(path: &str, content: &str) -> Option<Exclusion> {
    let name = Path::new(path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let extension = Path::new(name).extension().and_then(|e| e.to_str());
    let ssh_key = name.starts_with("id_") && matches!(extension, None | Some("pub"));
    if name.starts_with(".env") || matches!(extension, Some("pem" | "key")) || ssh_key {
        return Some(Exclusion::SensitiveName);
    }
    if PRIVATE_KEY.is_match(content) {
        return Some(Exclusion::PrivateKey);
    }
    None
}

/// Remplace par [`MASK`] chaque ligne qui ressemble à un secret ; renvoie aussi le
/// nombre de lignes masquées.
pub fn mask_secrets(text: &str) -> (String, usize) {
    let mut masked = 0;
    let lines: Vec<&str> = text
        .split('\n')
        .map(|line| {
            if SECRET_LINE.iter().any(|r| r.is_match(line)) {
                masked += 1;
                MASK
            } else {
                line
            }
        })
        .collect();
    (lines.join("\n"), masked)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exclut_les_fichiers_sensibles_par_leur_nom() {
        for path in [
            ".env",
            ".env.local",
            "config/.env.production",
            "cle.pem",
            "tls/server.key",
            "id_rsa",
            "id_ed25519.pub",
        ] {
            assert_eq!(
                exclusion(path, ""),
                Some(Exclusion::SensitiveName),
                "{path}"
            );
        }
        for path in [
            "src/main.rs",
            "environnement.md",
            "id_utils.py",
            "keys.rs",
            "monkey.ts",
        ] {
            assert_eq!(exclusion(path, "fn main() {}"), None, "{path}");
        }
    }

    #[test]
    fn exclut_un_fichier_contenant_une_cle_privee() {
        let content = "const K = `\n-----BEGIN OPENSSH PRIVATE KEY-----\nabc\n`;";
        assert_eq!(
            exclusion("src/cle.ts", content),
            Some(Exclusion::PrivateKey)
        );
        assert_eq!(exclusion("src/a.ts", "-----BEGIN PUBLIC KEY-----"), None);
    }

    #[test]
    fn masque_les_lignes_de_secrets() {
        let text = "\
const a = 1;
const aws = \"AKIAIOSFODNN7EXAMPLE\";
const gh = 'ghp_0123456789abcdefghijABCDEFGHIJ012345';
const or = \"sk-or-v1-0123456789abcdef0123\";
const password = \"hunter22\";
apiKey: 'abcd1234',
OPENROUTER_API_KEY=abcdef123456
export DB_PASSWORD=s3cr3t!
const password = process.env.PASSWORD;
token = demander_jeton()
DB_PASSWORD=$PASSWORD";
        let (masked, count) = mask_secrets(text);
        let lines: Vec<&str> = masked.lines().collect();
        assert_eq!(count, 7, "{masked}");
        assert_eq!(lines[0], "const a = 1;");
        for line in &lines[1..8] {
            assert_eq!(*line, MASK);
        }
        assert_eq!(lines[8], "const password = process.env.PASSWORD;");
        assert_eq!(lines[9], "token = demander_jeton()");
        assert_eq!(lines[10], "DB_PASSWORD=$PASSWORD");
    }

    #[test]
    fn les_motifs_sont_valides() {
        assert!(LazyLock::force(&SECRET_LINE).len() >= 7);
        assert!(PRIVATE_KEY.is_match("-----BEGIN RSA PRIVATE KEY-----"));
    }
}
