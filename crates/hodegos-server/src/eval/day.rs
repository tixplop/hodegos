//! Coût d'une journée type (phase 9f) : l'usage décrit par [`DayPlan::typical`] est
//! rejoué minute par minute avec le planificateur des revues automatiques et les
//! réglages par défaut, puis multiplié par les coûts mesurés pendant l'évaluation.

use std::collections::{BTreeMap, HashSet};
use std::fmt::Write as _;

use hodegos_mentors::{Registry, ReviewTrigger, Scope};
use jiff::{SignedDuration, Timestamp};

use super::run::CaseResult;
use crate::scheduler::{Candidate, Context, ReviewOptions, Scheduler};

/// Jours ouvrés par mois.
pub const WORK_DAYS: u32 = 22;
/// Coût mensuel au-delà duquel on réduit les plafonds par défaut (ADR 0010).
pub const MONTHLY_LIMIT_USD: f64 = 10.0;

/// Une journée d'usage, en minutes depuis minuit.
#[derive(Debug, Clone)]
pub struct DayPlan {
    pub saves: Vec<u32>,
    /// Ouvertures du projet ; à la première, des fichiers ont changé depuis la veille.
    pub opens: Vec<u32>,
    pub commits: Vec<u32>,
    /// Passages cités qui disparaissent : autant de réévaluations.
    pub reevaluations: u32,
}

impl DayPlan {
    /// Six heures d'édition (9 h – 12 h, 14 h – 17 h) : 40 enregistrements sur cinq
    /// fichiers, par rafales de cinq à deux minutes d'écart ; deux ouvertures du
    /// projet, quatre commits, deux passages cités qui disparaissent.
    pub fn typical() -> Self {
        let bursts = [540, 580, 620, 660, 840, 880, 920, 960];
        Self {
            saves: bursts
                .iter()
                .flat_map(|start| (0..5).map(move |n| start + 2 * n))
                .collect(),
            opens: vec![540, 840],
            commits: vec![600, 690, 900, 1005],
            reevaluations: 2,
        }
    }

    pub fn describe(&self) -> String {
        format!(
            "{} enregistrements par rafales de cinq (9 h, 9 h 40, 10 h 20, 11 h, 14 h, \
             14 h 40, 15 h 20, 16 h), {} ouvertures du projet (9 h, 14 h), {} commits, \
             {} passages cités qui disparaissent",
            self.saves.len(),
            self.opens.len(),
            self.commits.len(),
            self.reevaluations
        )
    }
}

/// Revues automatiques de la journée, par mentor ; rejoue `auto.rs` : une ouverture
/// marque `open` si des fichiers ont changé depuis la dernière revue du mentor, un
/// enregistrement marque `change` et relance le calme, un commit marque `commit`.
pub(crate) fn simulate(
    plan: &DayPlan,
    options: &ReviewOptions,
    mentors: &[Candidate],
) -> BTreeMap<String, u32> {
    let start = Timestamp::UNIX_EPOCH;
    let at = |minute: u32| start + SignedDuration::from_mins(i64::from(minute));
    let mut scheduler = Scheduler::default();
    let mut last_review: BTreeMap<String, Timestamp> = BTreeMap::new();
    let mut today: BTreeMap<String, u32> = BTreeMap::new();
    let mut last_save: Option<u32> = None;
    let running = HashSet::new();
    let no_keyword = |_: &str| false;
    for minute in 0..24 * 60 {
        let now = at(minute);
        if let Some(index) = plan.opens.iter().position(|m| *m == minute) {
            for mentor in mentors {
                let changed = index == 0
                    || last_save.is_some_and(|save| {
                        last_review.get(&mentor.id).is_none_or(|r| at(save) > *r)
                    });
                if changed {
                    scheduler.mark(&mentor.id, ReviewTrigger::Open, now);
                }
            }
        }
        if plan.saves.contains(&minute) {
            scheduler.changed(now);
            last_save = Some(minute);
            for mentor in mentors {
                scheduler.mark(&mentor.id, ReviewTrigger::Change, now);
            }
        }
        if plan.commits.contains(&minute) {
            for mentor in mentors {
                scheduler.mark(&mentor.id, ReviewTrigger::Commit, now);
            }
        }
        let due = scheduler.due(
            mentors,
            &Context {
                options,
                budget_ok: true,
                running: &running,
                last_review: &last_review,
                today: &today,
                keyword_hit: &no_keyword,
            },
            now,
        );
        for (id, _) in due {
            scheduler.clear(&id);
            last_review.insert(id.clone(), now);
            *today.entry(id).or_insert(0) += 1;
        }
    }
    today
}

/// Revues d'un mentor dans la journée et leur coût.
#[derive(Debug, Clone)]
pub struct MentorDay {
    pub id: String,
    pub reviews: u32,
    pub cost_per_review: f64,
    /// Coût mesuré sur ses propres cas (sinon : moyenne des autres revues).
    pub measured: bool,
}

#[derive(Debug, Clone)]
pub struct TypicalDay {
    pub plan: DayPlan,
    pub options: ReviewOptions,
    pub mentors: Vec<MentorDay>,
    pub cost_per_reevaluation: f64,
    /// Surcoût moyen d'une passe avec le voisinage, et passes de la journée.
    pub neighborhood_extra: f64,
    pub passes: u32,
}

impl TypicalDay {
    pub fn daily(&self) -> f64 {
        self.mentors
            .iter()
            .map(|m| f64::from(m.reviews) * m.cost_per_review)
            .sum::<f64>()
            + f64::from(self.plan.reevaluations) * self.cost_per_reevaluation
            + f64::from(self.passes) * self.neighborhood_extra
    }

    pub fn monthly(&self) -> f64 {
        self.daily() * f64::from(WORK_DAYS)
    }
}

fn mean(values: &[f64]) -> Option<f64> {
    (!values.is_empty()).then(|| values.iter().sum::<f64>() / values.len() as f64)
}

/// Journée type des mentors de projet actifs de `registry`, aux coûts mesurés dans
/// `results` ; `None` si aucune revue n'a été jouée.
pub fn typical_day(registry: &Registry, results: &[CaseResult]) -> Option<TypicalDay> {
    let played = |kind: &'static str| {
        results
            .iter()
            .filter(move |r| r.kind == kind && r.complete() && r.levels.iter().any(|l| l.called()))
    };
    let reviews: Vec<&CaseResult> = played("review").collect();
    let all_reviews = mean(&reviews.iter().map(|r| r.cost_usd()).collect::<Vec<_>>())?;
    let candidates: Vec<Candidate> = registry
        .active()
        .filter(|m| m.scopes.contains(&Scope::Project))
        .map(|m| Candidate {
            id: m.id.clone(),
            mode: m.mode,
            triggers: m
                .project
                .as_ref()
                .map(|p| p.triggers.clone())
                .unwrap_or_default(),
        })
        .collect();
    let plan = DayPlan::typical();
    let options = ReviewOptions::default();
    let counts = simulate(&plan, &options, &candidates);
    let mentors = candidates
        .iter()
        .map(|c| {
            let own: Vec<f64> = reviews
                .iter()
                .filter(|r| r.expected_mentor == c.id)
                .map(|r| r.cost_usd())
                .collect();
            MentorDay {
                id: c.id.clone(),
                reviews: counts.get(&c.id).copied().unwrap_or(0),
                cost_per_review: mean(&own).unwrap_or(all_reviews),
                measured: !own.is_empty(),
            }
        })
        .collect();
    let reevaluations: Vec<f64> = played("reevaluation").map(|r| r.cost_usd()).collect();
    let extras: Vec<f64> = played("neighborhood")
        .filter(|r| r.levels.len() == 2)
        .map(|r| (r.levels[1].cost_usd - r.levels[0].cost_usd).max(0.0))
        .collect();
    Some(TypicalDay {
        passes: u32::try_from(plan.saves.len()).unwrap_or(u32::MAX),
        plan,
        options,
        mentors,
        cost_per_reevaluation: mean(&reevaluations).unwrap_or(all_reviews),
        neighborhood_extra: mean(&extras).unwrap_or(0.0),
    })
}

fn usd(value: f64) -> String {
    super::report::money(value)
}

/// Section « Journée type » du rapport.
pub fn markdown(day: &TypicalDay) -> String {
    let mut out = String::new();
    let o = &day.options;
    let _ = writeln!(out, "## Coût d'une journée type\n");
    let _ = writeln!(
        out,
        "Usage : {}. Réglages par défaut : calme {} min, intervalle {} min, {} revues par \
         jour et par mentor, déclencheurs {}. Rejoué avec le planificateur de l'IDE ; \
         coûts mesurés ci-dessus.\n",
        day.plan.describe(),
        o.calm.as_secs() / 60,
        o.min_interval.as_secs() / 60,
        o.daily_max,
        o.triggers
            .iter()
            .map(|t| t.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    let _ = writeln!(
        out,
        "| Poste | Nombre par jour | Coût unitaire | Par jour |"
    );
    let _ = writeln!(out, "| --- | --- | --- | --- |");
    for m in &day.mentors {
        let _ = writeln!(
            out,
            "| Revues `{}`{} | {} | {} | {} |",
            m.id,
            if m.measured { "" } else { " (coût estimé)" },
            m.reviews,
            usd(m.cost_per_review),
            usd(f64::from(m.reviews) * m.cost_per_review)
        );
    }
    let _ = writeln!(
        out,
        "| Réévaluations | {} | {} | {} |",
        day.plan.reevaluations,
        usd(day.cost_per_reevaluation),
        usd(f64::from(day.plan.reevaluations) * day.cost_per_reevaluation)
    );
    let _ = writeln!(
        out,
        "| Surcoût du voisinage (passes) | {} | {} | {} |",
        day.passes,
        usd(day.neighborhood_extra),
        usd(f64::from(day.passes) * day.neighborhood_extra)
    );
    let monthly = day.monthly();
    let _ = writeln!(
        out,
        "\n**Par jour : {} · par mois ({} jours ouvrés) : {}** — {} {} par mois.\n",
        usd(day.daily()),
        WORK_DAYS,
        usd(monthly),
        if monthly <= MONTHLY_LIMIT_USD {
            "tient dans"
        } else {
            "**dépasse**"
        },
        usd(MONTHLY_LIMIT_USD)
    );
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use hodegos_mentors::Mode;

    fn mentor(id: &str, triggers: &[ReviewTrigger]) -> Candidate {
        Candidate {
            id: id.to_owned(),
            mode: Mode::Proactive,
            triggers: triggers.to_vec(),
        }
    }

    #[test]
    fn la_journee_type_respecte_calme_intervalle_et_plafond() {
        let plan = DayPlan::typical();
        assert_eq!(plan.saves.len(), 40);
        let all = [
            ReviewTrigger::Create,
            ReviewTrigger::Open,
            ReviewTrigger::Change,
        ];
        let mentors = [
            mentor("archi", &all),
            mentor("testeur", &[ReviewTrigger::Change]),
        ];
        let counts = simulate(&plan, &ReviewOptions::default(), &mentors);
        // Ouverture à 9 h, puis une revue après chaque rafale au calme, l'intervalle
        // de 20 min respecté ; l'ouverture de 14 h n'a rien de nouveau à relire.
        assert_eq!(counts["archi"], 9, "{counts:?}");
        assert_eq!(counts["testeur"], 8, "{counts:?}");

        let tight = ReviewOptions {
            daily_max: 3,
            ..ReviewOptions::default()
        };
        assert_eq!(simulate(&plan, &tight, &mentors)["archi"], 3);
        let off = ReviewOptions {
            enabled: false,
            ..ReviewOptions::default()
        };
        assert!(simulate(&plan, &off, &mentors).is_empty());
    }
}
