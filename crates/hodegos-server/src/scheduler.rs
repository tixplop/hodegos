//! Revues automatiques (phase 9d, ADR 0010) : quand un mentor de projet doit relire,
//! d'après ses déclencheurs, le calme, l'intervalle minimal, le plafond quotidien et le
//! budget. Logique pure, sans E/S ni horloge réelle : `auto.rs` la nourrit et la teste
//! avec une horloge simulée.

use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::path::Path;
use std::sync::Mutex;
use std::time::Duration;

use hodegos_mentors::{Mode, ReviewTrigger};
use jiff::{SignedDuration, Timestamp};
use serde::Deserialize;

/// Réglages des revues automatiques (`[project_review]` de `config.toml`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewOptions {
    pub enabled: bool,
    pub triggers: Vec<ReviewTrigger>,
    /// Calme après la dernière modification avant une revue `change`.
    pub calm: Duration,
    /// Intervalle minimal entre deux revues d'un mentor.
    pub min_interval: Duration,
    /// Revues automatiques par jour (UTC) et par mentor.
    pub daily_max: u32,
    /// Période de la boucle qui interroge le planificateur.
    pub tick: Duration,
}

impl Default for ReviewOptions {
    fn default() -> Self {
        Self {
            enabled: true,
            triggers: vec![
                ReviewTrigger::Create,
                ReviewTrigger::Open,
                ReviewTrigger::Change,
            ],
            calm: Duration::from_secs(180),
            min_interval: Duration::from_secs(1200),
            daily_max: 12,
            tick: Duration::from_secs(1),
        }
    }
}

#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    project_review: RawReview,
}

#[derive(Deserialize, Default)]
struct RawReview {
    enabled: Option<bool>,
    triggers: Option<Vec<String>>,
    calm_secs: Option<u64>,
    min_interval_secs: Option<u64>,
    daily_max: Option<u32>,
}

/// Lit `[project_review]` ; une valeur hors limites garde le défaut, avec la raison.
pub fn load_review_options(config_dir: &Path) -> (ReviewOptions, Option<String>) {
    let mut options = ReviewOptions::default();
    let path = config_dir.join("config.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return (options, None);
    };
    let raw: RawConfig = match toml::from_str(&text) {
        Ok(raw) => raw,
        Err(err) => {
            return (
                options,
                Some(format!(
                    "{} : [project_review] illisible ({}), valeurs par défaut",
                    path.display(),
                    err.message()
                )),
            );
        }
    };
    let raw = raw.project_review;
    let mut warnings = Vec::new();
    if let Some(enabled) = raw.enabled {
        options.enabled = enabled;
    }
    if let Some(names) = raw.triggers {
        let mut triggers = Vec::new();
        for name in names {
            match ReviewTrigger::from_name(&name) {
                Some(trigger) if !triggers.contains(&trigger) => triggers.push(trigger),
                Some(_) => {}
                None => warnings.push(format!("[project_review] déclencheur « {name} » inconnu")),
            }
        }
        triggers.sort();
        options.triggers = triggers;
    }
    match raw.calm_secs {
        Some(secs) if secs <= 3600 => options.calm = Duration::from_secs(secs),
        Some(secs) => warnings.push(format!(
            "[project_review] calm_secs = {secs} : 3600 au plus"
        )),
        None => {}
    }
    match raw.min_interval_secs {
        Some(secs) if secs <= 86_400 => options.min_interval = Duration::from_secs(secs),
        Some(secs) => warnings.push(format!(
            "[project_review] min_interval_secs = {secs} : 86400 au plus"
        )),
        None => {}
    }
    match raw.daily_max {
        Some(max) if max <= 200 => options.daily_max = max,
        Some(max) => warnings.push(format!("[project_review] daily_max = {max} : 200 au plus")),
        None => {}
    }
    (
        options,
        (!warnings.is_empty()).then(|| warnings.join(" ; ")),
    )
}

/// Horloge des revues automatiques ; simulée dans les tests.
pub trait Clock: Send + Sync {
    fn now(&self) -> Timestamp;
}

/// Horloge du système.
#[derive(Debug, Default)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> Timestamp {
        Timestamp::now()
    }
}

/// Horloge qu'on avance à la main (tests).
#[derive(Debug)]
pub struct ManualClock(Mutex<Timestamp>);

impl ManualClock {
    pub fn new(start: Timestamp) -> Self {
        Self(Mutex::new(start))
    }

    pub fn advance(&self, by: Duration) {
        if let Ok(mut now) = self.0.lock() {
            *now = now
                .checked_add(SignedDuration::try_from(by).unwrap_or(SignedDuration::ZERO))
                .unwrap_or(*now);
        }
    }
}

impl Clock for ManualClock {
    fn now(&self) -> Timestamp {
        self.0
            .lock()
            .map(|now| *now)
            .unwrap_or_else(|_| Timestamp::now())
    }
}

/// Ce qu'il faut savoir d'un mentor de projet pour décider.
#[derive(Debug, Clone)]
pub(crate) struct Candidate {
    pub(crate) id: String,
    pub(crate) mode: Mode,
    /// Déclencheurs que le mentor accepte (`project.triggers`).
    pub(crate) triggers: Vec<ReviewTrigger>,
}

/// État du projet au moment de décider.
pub(crate) struct Context<'a> {
    pub(crate) options: &'a ReviewOptions,
    pub(crate) budget_ok: bool,
    /// Mentors dont une revue est en cours.
    pub(crate) running: &'a HashSet<String>,
    /// Dernière revue de chaque mentor (demandée ou non).
    pub(crate) last_review: &'a BTreeMap<String, Timestamp>,
    /// Revues automatiques du jour (UTC), par mentor.
    pub(crate) today: &'a BTreeMap<String, u32>,
    /// Un mot-clé du mentor figure dans le diff depuis sa dernière revue (`strong-signal`).
    pub(crate) keyword_hit: &'a dyn Fn(&str) -> bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Pending {
    pub(crate) triggers: BTreeSet<ReviewTrigger>,
    pub(crate) since: Timestamp,
}

/// Revues en attente et dernière modification.
#[derive(Debug, Default)]
pub(crate) struct Scheduler {
    pending: BTreeMap<String, Pending>,
    last_change: Option<Timestamp>,
}

/// Ordre de priorité du déclencheur annoncé quand plusieurs se regroupent.
const PRIORITY: [ReviewTrigger; 4] = [
    ReviewTrigger::Create,
    ReviewTrigger::Open,
    ReviewTrigger::Commit,
    ReviewTrigger::Change,
];

fn elapsed(since: Timestamp, now: Timestamp) -> Duration {
    Duration::try_from(now.duration_since(since)).unwrap_or(Duration::ZERO)
}

impl Scheduler {
    /// Le mentor `id` doit relire, pour `trigger`.
    pub(crate) fn mark(&mut self, id: &str, trigger: ReviewTrigger, now: Timestamp) {
        let pending = self.pending.entry(id.to_owned()).or_insert(Pending {
            triggers: BTreeSet::new(),
            since: now,
        });
        pending.triggers.insert(trigger);
    }

    /// Une modification enregistrée : le calme repart de maintenant.
    pub(crate) fn changed(&mut self, now: Timestamp) {
        self.last_change = Some(now);
    }

    /// Calme atteint depuis la dernière modification.
    pub(crate) fn calm(&self, options: &ReviewOptions, now: Timestamp) -> bool {
        self.last_change
            .is_none_or(|last| elapsed(last, now) >= options.calm)
    }

    pub(crate) fn pending(&self) -> &BTreeMap<String, Pending> {
        &self.pending
    }

    /// Revue lancée (ou abandonnée) : plus rien n'attend pour ce mentor.
    pub(crate) fn clear(&mut self, id: &str) {
        self.pending.remove(id);
    }

    /// Revues à lancer maintenant, avec le déclencheur annoncé. Les demandes que le
    /// mentor ne prendra jamais (déclencheur refusé, `on-demand`, `strong-signal` sans
    /// mot-clé) sont oubliées ; les autres attendent leur tour.
    pub(crate) fn due(
        &mut self,
        mentors: &[Candidate],
        ctx: &Context<'_>,
        now: Timestamp,
    ) -> Vec<(String, ReviewTrigger)> {
        let mut due = Vec::new();
        let calm = self.calm(ctx.options, now);
        let ids: Vec<String> = self.pending.keys().cloned().collect();
        for id in ids {
            let Some(mentor) = mentors.iter().find(|m| m.id == id) else {
                self.pending.remove(&id);
                continue;
            };
            let Some(pending) = self.pending.get_mut(&id) else {
                continue;
            };
            pending.triggers.retain(|t| {
                ctx.options.triggers.contains(t)
                    && mentor.triggers.contains(t)
                    && match mentor.mode {
                        Mode::Proactive => true,
                        Mode::StrongSignal => *t != ReviewTrigger::Create,
                        Mode::OnDemand => false,
                    }
            });
            if pending.triggers.is_empty() || !ctx.options.enabled {
                self.pending.remove(&id);
                continue;
            }
            if !ctx.budget_ok || ctx.running.contains(&id) {
                continue;
            }
            let only_change = pending.triggers.iter().all(|t| *t == ReviewTrigger::Change);
            if only_change && !calm {
                continue;
            }
            let rested = ctx
                .last_review
                .get(&id)
                .is_none_or(|last| elapsed(*last, now) >= ctx.options.min_interval);
            let room = ctx.today.get(&id).copied().unwrap_or(0) < ctx.options.daily_max;
            if !rested || !room {
                continue;
            }
            if mentor.mode == Mode::StrongSignal && !(ctx.keyword_hit)(&id) {
                self.pending.remove(&id);
                continue;
            }
            let trigger = PRIORITY
                .into_iter()
                .find(|t| pending.triggers.contains(t))
                .unwrap_or(ReviewTrigger::Change);
            due.push((id, trigger));
        }
        due
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(minutes: i64) -> Timestamp {
        Timestamp::from_second(1_793_000_000 + minutes * 60).unwrap()
    }

    fn mentor(id: &str, mode: Mode) -> Candidate {
        Candidate {
            id: id.to_owned(),
            mode,
            triggers: vec![
                ReviewTrigger::Create,
                ReviewTrigger::Open,
                ReviewTrigger::Change,
                ReviewTrigger::Commit,
            ],
        }
    }

    struct World {
        options: ReviewOptions,
        budget_ok: bool,
        running: HashSet<String>,
        last_review: BTreeMap<String, Timestamp>,
        today: BTreeMap<String, u32>,
        hit: bool,
    }

    impl World {
        fn new() -> Self {
            Self {
                options: ReviewOptions::default(),
                budget_ok: true,
                running: HashSet::new(),
                last_review: BTreeMap::new(),
                today: BTreeMap::new(),
                hit: false,
            }
        }

        fn due(
            &self,
            scheduler: &mut Scheduler,
            mentors: &[Candidate],
            now: Timestamp,
        ) -> Vec<(String, ReviewTrigger)> {
            let hit = self.hit;
            let keyword = move |_: &str| hit;
            scheduler.due(
                mentors,
                &Context {
                    options: &self.options,
                    budget_ok: self.budget_ok,
                    running: &self.running,
                    last_review: &self.last_review,
                    today: &self.today,
                    keyword_hit: &keyword,
                },
                now,
            )
        }
    }

    #[test]
    fn creation_et_ouverture_partent_aussitot() {
        let world = World::new();
        let mentors = [mentor("archi", Mode::Proactive)];
        let mut scheduler = Scheduler::default();
        assert!(
            world.due(&mut scheduler, &mentors, at(0)).is_empty(),
            "rien en attente"
        );
        scheduler.mark("archi", ReviewTrigger::Create, at(0));
        scheduler.mark("archi", ReviewTrigger::Open, at(0));
        assert_eq!(
            world.due(&mut scheduler, &mentors, at(0)),
            vec![("archi".to_owned(), ReviewTrigger::Create)],
            "deux déclencheurs, une revue"
        );
    }

    #[test]
    fn une_modification_attend_le_calme_puis_l_intervalle() {
        let mut world = World::new();
        let mentors = [mentor("archi", Mode::Proactive)];
        let mut scheduler = Scheduler::default();
        world.last_review.insert("archi".to_owned(), at(0));
        scheduler.changed(at(10));
        scheduler.mark("archi", ReviewTrigger::Change, at(10));
        assert!(
            world.due(&mut scheduler, &mentors, at(12)).is_empty(),
            "pas encore au calme"
        );
        scheduler.changed(at(12));
        assert!(
            world.due(&mut scheduler, &mentors, at(14)).is_empty(),
            "le calme repart"
        );
        assert!(
            world.due(&mut scheduler, &mentors, at(16)).is_empty(),
            "calme, mais intervalle de 20 min"
        );
        assert_eq!(world.due(&mut scheduler, &mentors, at(20)).len(), 1);
        // En cours : on attend la fin.
        world.running.insert("archi".to_owned());
        assert!(world.due(&mut scheduler, &mentors, at(21)).is_empty());
    }

    #[test]
    fn plafond_quotidien_et_budget() {
        let mut world = World::new();
        let mentors = [mentor("archi", Mode::Proactive)];
        let mut scheduler = Scheduler::default();
        scheduler.mark("archi", ReviewTrigger::Open, at(0));
        world.today.insert("archi".to_owned(), 12);
        assert!(
            world.due(&mut scheduler, &mentors, at(0)).is_empty(),
            "plafond du jour"
        );
        world.today.clear();
        world.budget_ok = false;
        assert!(
            world.due(&mut scheduler, &mentors, at(0)).is_empty(),
            "budget atteint"
        );
        world.budget_ok = true;
        assert_eq!(
            world.due(&mut scheduler, &mentors, at(0)).len(),
            1,
            "le lendemain, ou budget relevé"
        );
    }

    #[test]
    fn modes_et_declencheurs_refuses() {
        let mut world = World::new();
        let mut proactive = mentor("archi", Mode::Proactive);
        proactive.triggers = vec![ReviewTrigger::Change];
        let mentors = [
            proactive,
            mentor("demande", Mode::OnDemand),
            mentor("signal", Mode::StrongSignal),
        ];
        let mut scheduler = Scheduler::default();
        for id in ["archi", "demande", "signal"] {
            scheduler.mark(id, ReviewTrigger::Create, at(0));
        }
        // Le mentor renonce à create ; on-demand jamais ; strong-signal pas à la création.
        assert!(world.due(&mut scheduler, &mentors, at(0)).is_empty());
        assert!(scheduler.pending().is_empty());

        scheduler.mark("signal", ReviewTrigger::Open, at(1));
        assert!(
            world.due(&mut scheduler, &mentors, at(1)).is_empty(),
            "sans mot-clé, oublié"
        );
        assert!(scheduler.pending().is_empty());
        world.hit = true;
        scheduler.mark("signal", ReviewTrigger::Open, at(2));
        assert_eq!(
            world.due(&mut scheduler, &mentors, at(2)),
            vec![("signal".to_owned(), ReviewTrigger::Open)]
        );

        // Commit désactivé par défaut ; revues automatiques coupées.
        scheduler.clear("signal");
        scheduler.mark("signal", ReviewTrigger::Commit, at(3));
        assert!(world.due(&mut scheduler, &mentors, at(3)).is_empty());
        world.options.triggers.push(ReviewTrigger::Commit);
        scheduler.mark("signal", ReviewTrigger::Commit, at(4));
        assert_eq!(world.due(&mut scheduler, &mentors, at(4)).len(), 1);
        world.options.enabled = false;
        assert!(world.due(&mut scheduler, &mentors, at(5)).is_empty());
    }

    #[test]
    fn reglages_lus_dans_config_toml() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_review_options(dir.path()).0, ReviewOptions::default());
        fs::write(
            dir.path().join("config.toml"),
            "[project_review]\nenabled = false\ntriggers = [\"change\", \"commit\"]\ncalm_secs = 60\nmin_interval_secs = 600\ndaily_max = 3\n",
        )
        .unwrap();
        let (options, warning) = load_review_options(dir.path());
        assert!(!options.enabled);
        assert_eq!(
            options.triggers,
            vec![ReviewTrigger::Change, ReviewTrigger::Commit]
        );
        assert_eq!(options.calm, Duration::from_secs(60));
        assert_eq!(options.daily_max, 3);
        assert!(warning.is_none());
        fs::write(
            dir.path().join("config.toml"),
            "[project_review]\ntriggers = [\"save\"]\ndaily_max = 1000\n",
        )
        .unwrap();
        let (options, warning) = load_review_options(dir.path());
        assert_eq!(options.daily_max, 12);
        assert!(warning.unwrap().contains("save"));
    }
}
