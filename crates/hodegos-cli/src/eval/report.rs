//! Rapport d'évaluation : verdict par modèle et par niveau, détail par cas et par
//! palier, en Markdown.

use std::fmt::Write as _;

use hodegos_mentors::{Level, ModelTier};

use super::run::{CaseResult, Outcome, tier_name};

/// Part minimale des cas où la notion doit émerger.
pub const MENTION_THRESHOLD: f64 = 0.7;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Retained,
    Rejected,
    /// Pannes, cas injouables ou arrêt sur le budget : à relancer.
    Inconclusive,
}

impl Verdict {
    fn label(self) -> &'static str {
        match self {
            Self::Retained => "retenu",
            Self::Rejected => "non retenu",
            Self::Inconclusive => "non concluant",
        }
    }
}

/// Résultats d'un modèle sur les cas de son niveau.
#[derive(Debug, Clone)]
pub struct Summary {
    pub tier: Option<ModelTier>,
    pub model: Option<String>,
    pub cases: usize,
    pub mentions: usize,
    pub leaks_drafts: usize,
    pub leaks_shown: usize,
    pub invalid: usize,
    pub provider_errors: usize,
    pub casting_mismatches: usize,
    pub incomplete: usize,
    pub cost_usd: f64,
    pub median_ms: u64,
    pub p90_ms: u64,
}

impl Summary {
    pub fn mention_rate(&self) -> f64 {
        if self.cases == 0 {
            0.0
        } else {
            self.mentions as f64 / self.cases as f64
        }
    }

    /// Retenu : aucune solution dans ce que le modèle propose (brouillons compris) et
    /// la notion émerge dans 70 % des cas au moins.
    pub fn verdict(&self) -> Verdict {
        if self.leaks_drafts > 0 {
            Verdict::Rejected
        } else if self.incomplete > 0 || self.model.is_none() {
            Verdict::Inconclusive
        } else if self.mention_rate() >= MENTION_THRESHOLD {
            Verdict::Retained
        } else {
            Verdict::Rejected
        }
    }
}

/// Un résumé par couple (niveau, modèle), dans l'ordre fast puis strong.
pub fn summarize(results: &[CaseResult]) -> Vec<Summary> {
    let mut keys: Vec<(Option<ModelTier>, Option<String>)> = Vec::new();
    for result in results {
        let key = (result.tier, result.model.clone());
        if !keys.contains(&key) {
            keys.push(key);
        }
    }
    keys.sort_by_key(|(tier, model)| (tier.map(tier_rank), model.clone()));
    keys.into_iter()
        .map(|(tier, model)| {
            let group: Vec<&CaseResult> = results
                .iter()
                .filter(|r| r.tier == tier && r.model == model)
                .collect();
            let mut durations: Vec<u64> = group
                .iter()
                .flat_map(|r| r.levels.iter())
                .filter(|l| l.called())
                .map(|l| l.duration_ms)
                .collect();
            durations.sort_unstable();
            Summary {
                tier,
                model,
                cases: group.len(),
                mentions: group.iter().filter(|r| r.mentions()).count(),
                leaks_drafts: group.iter().map(|r| r.leaks_drafts()).sum(),
                leaks_shown: group.iter().map(|r| r.leaks_shown()).sum(),
                invalid: count_levels(&group, |o| *o == Outcome::InvalidResponse),
                provider_errors: count_levels(&group, |o| matches!(o, Outcome::ProviderError(_))),
                casting_mismatches: group.iter().filter(|r| !r.casting_ok()).count(),
                incomplete: group.iter().filter(|r| !r.complete()).count(),
                cost_usd: group.iter().map(|r| r.cost_usd()).sum(),
                median_ms: percentile(&durations, 50),
                p90_ms: percentile(&durations, 90),
            }
        })
        .collect()
}

fn tier_rank(tier: ModelTier) -> u8 {
    match tier {
        ModelTier::Fast => 0,
        ModelTier::Strong => 1,
    }
}

fn count_levels(group: &[&CaseResult], matches: impl Fn(&Outcome) -> bool) -> usize {
    group
        .iter()
        .flat_map(|r| r.levels.iter())
        .filter(|l| matches(&l.outcome))
        .count()
}

/// Centile `p` (plus proche rang) d'une liste triée ; 0 si elle est vide.
fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let rank = (p * sorted.len()).div_ceil(100).max(1);
    sorted[rank.min(sorted.len()) - 1]
}

pub fn money(usd: f64) -> String {
    format!("{usd:.4} $").replace('.', ",")
}

fn seconds(ms: u64) -> String {
    format!("{:.1} s", ms as f64 / 1000.0).replace('.', ",")
}

fn percent(rate: f64) -> String {
    format!("{:.0} %", rate * 100.0)
}

fn level_name(level: Level) -> &'static str {
    match level {
        Level::Indice => "Indice",
        Level::Question => "Question",
        Level::Piste => "Piste",
        Level::Exemple => "Exemple",
    }
}

fn outcome_label(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Shown => "affiché".to_owned(),
        Outcome::Nothing => "rien".to_owned(),
        Outcome::InvalidResponse => "JSON invalide".to_owned(),
        Outcome::ProviderError(message) => format!("panne : {message}"),
        Outcome::NotReached => "non atteint".to_owned(),
        Outcome::BudgetStopped => "arrêt (--max-cost)".to_owned(),
    }
}

fn ratio((ok, total): (usize, usize)) -> String {
    if total == 0 {
        "—".to_owned()
    } else {
        format!("{ok}/{total}")
    }
}

fn model_label(summary: &Summary) -> String {
    format!(
        "{} · {}",
        summary.tier.map_or("?", tier_name),
        summary.model.as_deref().unwrap_or("aucun modèle")
    )
}

/// Rapport complet, en Markdown.
pub fn markdown(when: &str, results: &[CaseResult], summaries: &[Summary]) -> String {
    let mut out = String::new();
    let total: f64 = results.iter().map(CaseResult::cost_usd).sum();
    let _ = writeln!(out, "# Évaluation des mentors — {when}\n");
    let _ = writeln!(
        out,
        "{} cas, coût total {}. Un modèle est retenu s'il ne propose aucune solution \
         (motifs `must_not`, brouillons écartés compris) et fait émerger la notion \
         (`must_mention`) dans {} des cas au moins.\n",
        results.len(),
        money(total),
        percent(MENTION_THRESHOLD)
    );

    let _ = writeln!(out, "## Verdict par modèle\n");
    let _ = writeln!(
        out,
        "| Niveau · modèle | Cas | Notion émergée | Solution (brouillons) | Solution (affichée) | JSON invalides | Pannes | Casting différent | Coût | Durée médiane | Durée p90 | Verdict |"
    );
    let _ = writeln!(
        out,
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |"
    );
    for s in summaries {
        let _ = writeln!(
            out,
            "| {} | {} | {}/{} ({}) | {} | {} | {} | {} | {} | {} | {} | {} | **{}** |",
            model_label(s),
            s.cases,
            s.mentions,
            s.cases,
            percent(s.mention_rate()),
            s.leaks_drafts,
            s.leaks_shown,
            s.invalid,
            s.provider_errors,
            s.casting_mismatches,
            money(s.cost_usd),
            seconds(s.median_ms),
            seconds(s.p90_ms),
            s.verdict().label()
        );
    }

    let _ = writeln!(out, "\n## Cas\n");
    for r in results {
        let _ = writeln!(out, "### {} — {}\n", r.name, r.title);
        let casting = if r.casting_ok() {
            format!("titulaire `{}`", r.expected_mentor)
        } else {
            format!(
                "titulaire `{}`, **attendu `{}`**",
                r.mentor.as_deref().unwrap_or("aucun"),
                r.expected_mentor
            )
        };
        let _ = writeln!(
            out,
            "`{}` · {} · modèle `{}` · notion {} · coût {}\n",
            r.file,
            casting,
            r.model.as_deref().unwrap_or("—"),
            if r.mentions() {
                "émergée"
            } else {
                "**non émergée**"
            },
            money(r.cost_usd())
        );
        if let Some(error) = &r.error {
            let _ = writeln!(out, "**Cas non joué** : {error}\n");
            continue;
        }
        let _ = writeln!(
            out,
            "| Palier | Issue | Citation valide | Palier respecté | Notion | Solution (brouillons) | Coût | Durée |"
        );
        let _ = writeln!(out, "| --- | --- | --- | --- | --- | --- | --- | --- |");
        for l in &r.levels {
            let leaks = if l.leaks_drafts.is_empty() {
                "non".to_owned()
            } else {
                format!("**oui** : `{}`", l.leaks_drafts.join("`, `"))
            };
            let _ = writeln!(
                out,
                "| {} | {} | {} | {} | {} | {} | {} | {} |",
                level_name(l.level),
                outcome_label(&l.outcome).replace('|', "/"),
                l.valid_quotes.map_or("—".to_owned(), ratio),
                ratio(l.right_level),
                if l.mentions { "oui" } else { "non" },
                leaks,
                money(l.cost_usd),
                seconds(l.duration_ms)
            );
        }
        out.push('\n');
        for l in &r.levels {
            for draft in &l.drafts {
                let status = match draft.rejected {
                    None => "affiché".to_owned(),
                    Some(reason) => format!("écarté : {reason}"),
                };
                let _ = writeln!(out, "**{}** — {status}\n", level_name(l.level));
                if let Some(quote) = draft.quote.as_deref().filter(|q| !q.is_empty()) {
                    let _ = writeln!(out, "Citation : `{}`\n", quote.replace('`', "'"));
                }
                // Le message part tel quel dans un bloc de citation, code compris.
                for line in draft.message.lines() {
                    let _ = writeln!(out, "> {line}");
                }
                out.push('\n');
            }
        }
    }
    out
}

/// Résumé affiché dans le terminal.
pub fn terminal(results: &[CaseResult], summaries: &[Summary]) -> String {
    let mut out = String::new();
    for r in results {
        let state = match &r.error {
            Some(error) => format!("non joué : {error}"),
            None => format!(
                "notion {}, solution {}, {}",
                if r.mentions() { "oui" } else { "non" },
                if r.leaks_drafts() > 0 { "OUI" } else { "non" },
                money(r.cost_usd())
            ),
        };
        let casting = if r.casting_ok() {
            ""
        } else {
            " (casting différent)"
        };
        let _ = writeln!(out, "  {}{casting} : {state}", r.name);
    }
    out.push('\n');
    for s in summaries {
        let _ = writeln!(
            out,
            "{} : notion {}/{} ({}), solution {} (affichée {}), {} → {}",
            model_label(s),
            s.mentions,
            s.cases,
            percent(s.mention_rate()),
            s.leaks_drafts,
            s.leaks_shown,
            money(s.cost_usd),
            s.verdict().label()
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::run::LevelResult;

    fn level(level: Level, mentions: bool, leaks: &[&str], ms: u64) -> LevelResult {
        LevelResult {
            level,
            outcome: Outcome::Shown,
            shown: vec!["Regardez.".to_owned()],
            drafts: vec![],
            valid_quotes: Some((1, 1)),
            right_level: (1, 1),
            mentions,
            leaks_shown: vec![],
            leaks_drafts: leaks.iter().map(|s| (*s).to_owned()).collect(),
            cost_usd: 0.001,
            duration_ms: ms,
        }
    }

    fn case(name: &str, mentions: bool, leaks: &[&str]) -> CaseResult {
        CaseResult {
            name: name.to_owned(),
            title: format!("Cas {name}"),
            file: "src/a.ts".to_owned(),
            expected_mentor: "mentor-langage".to_owned(),
            mentor: Some("mentor-langage".to_owned()),
            tier: Some(ModelTier::Fast),
            model: Some("test/rapide".to_owned()),
            error: None,
            levels: vec![
                level(Level::Indice, false, &[], 800),
                level(Level::Question, mentions, leaks, 1200),
            ],
        }
    }

    #[test]
    fn retenu_a_soixante_dix_pour_cent_sans_solution() {
        let mut results: Vec<CaseResult> =
            (0..7).map(|n| case(&format!("{n}"), true, &[])).collect();
        results.extend((7..10).map(|n| case(&format!("{n}"), false, &[])));
        let summaries = summarize(&results);
        assert_eq!(summaries.len(), 1);
        assert_eq!(summaries[0].mentions, 7);
        assert_eq!(summaries[0].verdict(), Verdict::Retained);

        results[6] = case("6", false, &[]);
        assert_eq!(summarize(&results)[0].verdict(), Verdict::Rejected);
    }

    #[test]
    fn une_seule_solution_en_brouillon_suffit_a_ecarter_le_modele() {
        let mut results: Vec<CaseResult> =
            (0..10).map(|n| case(&format!("{n}"), true, &[])).collect();
        results[3] = case("3", true, &["getTime() ==="]);
        let summary = &summarize(&results)[0];
        assert_eq!(summary.leaks_drafts, 1);
        assert_eq!(summary.verdict(), Verdict::Rejected);
    }

    #[test]
    fn une_panne_rend_le_verdict_non_concluant() {
        let mut results = vec![case("1", true, &[])];
        results[0].levels[1].outcome = Outcome::ProviderError("délai dépassé".to_owned());
        assert_eq!(summarize(&results)[0].verdict(), Verdict::Inconclusive);
    }

    #[test]
    fn un_resume_par_niveau_et_par_modele() {
        let mut strong = case("2", true, &[]);
        strong.tier = Some(ModelTier::Strong);
        strong.model = Some("test/fort".to_owned());
        let summaries = summarize(&[strong, case("1", true, &[])]);
        assert_eq!(summaries.len(), 2);
        assert_eq!(summaries[0].tier, Some(ModelTier::Fast));
        assert_eq!(summaries[1].model.as_deref(), Some("test/fort"));
    }

    #[test]
    fn centiles_des_durees() {
        assert_eq!(percentile(&[], 50), 0);
        assert_eq!(percentile(&[100, 200, 300, 400], 50), 200);
        let many: Vec<u64> = (1..=10).map(|n| n * 100).collect();
        assert_eq!(percentile(&many, 90), 900);
    }

    #[test]
    fn rapport_markdown() {
        let mut results = vec![case("01-a", true, &[]), case("02-b", false, &["solution"])];
        results[1].mentor = Some("architecte".to_owned());
        let text = markdown("2026-10-29 12:00", &results, &summarize(&results));
        assert!(text.starts_with("# Évaluation des mentors — 2026-10-29 12:00"));
        assert!(
            text.contains("| fast · test/rapide | 2 | 1/2 (50 %) | 1 | 0 |"),
            "{text}"
        );
        assert!(text.contains("**non retenu**"));
        assert!(text.contains("### 01-a — Cas 01-a"));
        assert!(text.contains("titulaire `architecte`, **attendu `mentor-langage`**"));
        assert!(text.contains("| Question | affiché | 1/1 | 1/1 | non | **oui** : `solution` |"));
    }
}
