//! Commentaires HTML d'un document Markdown (`<!-- … -->`), retirés avant tout envoi
//! au modèle : ce sont des consignes pour la personne, pas son texte.

const OPEN: &str = "<!--";
const CLOSE: &str = "-->";

/// Retire les commentaires HTML ; une ligne qui ne contenait qu'un commentaire
/// disparaît. Un commentaire non fermé court jusqu'à la fin du texte.
pub fn strip_html_comments(text: &str) -> String {
    let mut inside = false;
    let kept: Vec<String> = text
        .split('\n')
        .filter_map(|line| strip_line(line, &mut inside))
        .collect();
    kept.join("\n")
}

/// Même règle pour un diff unifié : les lignes retirées suivent l'ancienne version,
/// les lignes ajoutées la nouvelle, les lignes de contexte les deux.
pub fn strip_html_comments_in_diff(diff: &str) -> String {
    let mut old_inside = false;
    let mut new_inside = false;
    let kept: Vec<String> = diff
        .split('\n')
        .filter_map(|line| {
            if line.starts_with("--- ") || line.starts_with("+++ ") || line.starts_with("@@") {
                return Some(line.to_owned());
            }
            let mut chars = line.chars();
            let (prefix, rest) = match chars.next() {
                Some(c @ ('+' | '-' | ' ')) => (c, chars.as_str()),
                _ => return Some(line.to_owned()),
            };
            let stripped = match prefix {
                '+' => strip_line(rest, &mut new_inside),
                '-' => strip_line(rest, &mut old_inside),
                _ => {
                    let stripped = strip_line(rest, &mut new_inside);
                    old_inside = new_inside;
                    stripped
                }
            };
            stripped.map(|s| format!("{prefix}{s}"))
        })
        .collect();
    kept.join("\n")
}

/// Ligne sans ses commentaires, ou `None` si elle n'était faite que de commentaires.
fn strip_line(line: &str, inside: &mut bool) -> Option<String> {
    let mut out = String::new();
    let mut rest = line;
    let mut had_comment = *inside;
    loop {
        if *inside {
            match rest.find(CLOSE) {
                Some(end) => {
                    rest = &rest[end + CLOSE.len()..];
                    *inside = false;
                }
                None => break,
            }
        } else {
            match rest.find(OPEN) {
                Some(start) => {
                    out.push_str(&rest[..start]);
                    rest = &rest[start + OPEN.len()..];
                    *inside = true;
                    had_comment = true;
                }
                None => {
                    out.push_str(rest);
                    break;
                }
            }
        }
    }
    (!had_comment || !out.trim().is_empty()).then_some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retire_les_commentaires_et_les_lignes_qui_n_en_contenaient_qu_un() {
        let text =
            "## Idée\n\n<!-- En deux ou trois phrases. -->\n\nUn texte <!-- note --> écrit.\n";
        assert_eq!(strip_html_comments(text), "## Idée\n\n\nUn texte  écrit.\n");
    }

    #[test]
    fn commentaire_sur_plusieurs_lignes_ou_non_ferme() {
        assert_eq!(
            strip_html_comments("a\n<!-- début\nmilieu\nfin -->b\nc"),
            "a\nb\nc"
        );
        assert_eq!(strip_html_comments("a\n<!-- jamais fermé\nb"), "a");
        assert_eq!(strip_html_comments("sans commentaire"), "sans commentaire");
    }

    #[test]
    fn diff_sans_commentaires() {
        let diff = "--- avant\n+++ après\n@@ -0,0 +1,3 @@\n+## Idée\n+<!-- consigne\n+suite -->\n+Mon idée\n";
        assert_eq!(
            strip_html_comments_in_diff(diff),
            "--- avant\n+++ après\n@@ -0,0 +1,3 @@\n+## Idée\n+Mon idée\n"
        );
        let mixed = " <!-- a -->\n-ancien <!-- b -->\n+nouveau\n";
        assert_eq!(strip_html_comments_in_diff(mixed), "-ancien \n+nouveau\n");
    }
}
