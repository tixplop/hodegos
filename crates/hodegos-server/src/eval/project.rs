//! Cas joués sur un projet d'exemple (phase 9f) : revue d'un mentor de projet,
//! passe avec ou sans voisinage, réévaluation d'une remarque dont un passage a
//! disparu. Le projet est recopié dans un dossier temporaire ; carte, fichiers clés,
//! diff, voisinage, prompts et validation sont ceux de l'IDE.

use std::collections::BTreeMap;
use std::fs;
use std::time::Instant;

use hodegos_core::map::{
    DEFAULT_KEY_FILES_CAP, DEFAULT_NEIGHBORHOOD_CAP, MAX_DIFF_FILES, MapCache, MapSettings,
    build_map, diff_since, key_files, masked_texts, neighborhood,
};
use hodegos_core::secrets::mask_secrets;
use hodegos_core::{
    Language, Project, RelPath, Session, UserId, section_at, sections, strip_html_comments,
};
use hodegos_llm::{
    About, ChatMessage, LlmClient, LlmRequest, ProjectRemarkDraft, ProjectRules, Reevaluated, Role,
    Rules, ask_for_project_remarks, ask_for_reevaluation, project_remarks_schema,
    reevaluation_schema, remarks_schema, remarks_schema_with_see_also, validate_project,
};
use hodegos_mentors::{
    Author, CastRequest, ContextScope, Extra, FileDiff, FileText, FollowUp, FollowUpKind, Level,
    Mentor, NeighborText, ReevaluationInput, Registry, ReviewInput, Scope, SectionRequest, cast,
    reevaluation_prompt, review_prompt, section_headings, section_prompt,
};

use super::case::{Case, Files, Kind, ReevaluationCase, ReviewCase, Sample, Verdict};
use super::run::{
    CaseResult, Draft, LevelResult, Models, Outcome, Spend, fill_error, finish, first_level, leaks,
    remark_level, tier_name,
};

/// Copie de travail d'un projet d'exemple, effacée à la fin du cas.
pub(super) struct Workspace {
    _dir: tempfile::TempDir,
    project: Project,
    session: Session,
    cache: MapCache,
    settings: MapSettings,
}

impl Workspace {
    /// Copie du projet `sample`, dans un dossier à son nom (la carte l'affiche).
    pub(super) fn new(sample: &Sample) -> Result<Self, String> {
        let dir = tempfile::tempdir().map_err(|e| format!("dossier temporaire : {e}"))?;
        let root = dir.path().join(&sample.name);
        // Un dépôt git : `.gitignore` s'applique comme dans un vrai projet.
        fs::create_dir_all(root.join(".git")).map_err(|e| e.to_string())?;
        let project = Project::open(&root).map_err(|e| e.to_string())?;
        let session = project.session(UserId::local());
        let mut workspace = Self {
            _dir: dir,
            project,
            session,
            cache: MapCache::default(),
            settings: MapSettings::default(),
        };
        workspace.write(&sample.files)?;
        Ok(workspace)
    }

    /// Écrit `files` par-dessus le projet, puis relit la carte.
    pub(super) fn write(&mut self, files: &Files) -> Result<(), String> {
        for (rel, text) in files {
            let rel = RelPath::parse(rel).map_err(|e| format!("{rel} : {e}"))?;
            let path = self.project.root().join(rel.as_str());
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            fs::write(&path, text).map_err(|e| format!("{rel} : {e}"))?;
        }
        self.cache
            .refresh(&self.project, &self.session, &self.settings.exclude)
            .map_err(|e| e.to_string())?;
        Ok(())
    }

    pub(super) fn map(&self) -> String {
        build_map(self.project.root(), &self.cache, &self.settings).text
    }

    pub(super) fn texts(&self) -> BTreeMap<String, String> {
        masked_texts(&self.project, &self.session, &self.cache)
    }

    fn read(&self, path: &str) -> Option<String> {
        let rel = RelPath::parse(path).ok()?;
        self.project
            .read_file(&self.session, &rel)
            .ok()
            .map(|f| f.content)
    }

    /// Section du fichier `path` qui contient `quote`, comme `review::locate`.
    pub(super) fn locate(&self, path: &str, quote: &str) -> Option<(String, String)> {
        let content = self.read(path)?;
        let line = if quote.is_empty() {
            1
        } else {
            content[..content.find(quote)?].matches('\n').count() + 1
        };
        let all = sections(Language::of(path), &content);
        let section = section_at(&all, line)?;
        Some((section.id.clone(), section.text(&content)))
    }
}

/// Résultat vide d'un cas sur projet, avant tout appel.
fn result_for(case: &Case) -> CaseResult {
    let mut result = CaseResult::new(case);
    result.kind = case.kind_name();
    result
}

fn fail(mut result: CaseResult, message: impl Into<String>) -> CaseResult {
    result.error = Some(message.into());
    result
}

/// Mentor de projet actif `id`, avec son modèle.
fn project_mentor<'a>(
    registry: &'a Registry,
    models: &Models,
    result: &mut CaseResult,
    id: &str,
) -> Result<(&'a Mentor, String), String> {
    let mentor = registry
        .get(id)
        .filter(|m| m.scopes.contains(&Scope::Project))
        .ok_or_else(|| format!("« {id} » n'est pas un mentor de projet actif"))?;
    result.mentor = Some(mentor.id.clone());
    result.tier = Some(mentor.model);
    let model = models
        .get(mentor.model)
        .ok_or_else(|| format!("modèle « {} » non configuré", tier_name(mentor.model)))?;
    result.model = Some(model.to_owned());
    Ok((mentor, model.to_owned()))
}

/* ---------- Revue ---------- */

/// Prompt et règles d'une revue, prêts à partir.
pub(super) struct PreparedReview {
    pub(super) workspace: Workspace,
    pub(super) request: LlmRequest,
    pub(super) map: String,
    pub(super) sent: Vec<(String, String)>,
    pub(super) level: Level,
    pub(super) max_remarks: usize,
}

pub(super) fn prepare_review(
    mentor: &Mentor,
    model: &str,
    models: &Models,
    review: &ReviewCase,
) -> Result<PreparedReview, String> {
    let mut workspace = Workspace::new(&review.sample)?;
    let first = review.sample.after.is_empty();
    // Sans `after/`, première revue ; sinon, revue suivante : diff depuis le projet.
    let before = workspace.texts();
    if !first {
        workspace.write(&review.sample.after)?;
    }
    let diff = if first {
        Vec::new()
    } else {
        diff_since(&before, &workspace.texts(), MAX_DIFF_FILES)
    };
    let map = workspace.map();
    let keys = key_files(
        &workspace.project,
        &workspace.session,
        &workspace.cache,
        &|path| mentor.is_key_file(path),
        DEFAULT_KEY_FILES_CAP,
    );
    let prompt = review_prompt(&ReviewInput {
        mentor,
        level: mentor.entry_level,
        map: &map,
        key_files: keys
            .files
            .iter()
            .map(|f| FileText {
                path: &f.path,
                form: &f.form,
                text: &f.text,
            })
            .collect(),
        diff: diff
            .iter()
            .map(|d| FileDiff {
                path: &d.path,
                diff: &d.diff,
            })
            .collect(),
        first,
        open_remarks: Vec::new(),
    });
    let mut sent: Vec<(String, String)> = keys
        .files
        .iter()
        .map(|f| (f.path.clone(), f.text.clone()))
        .collect();
    sent.extend(diff.iter().map(|d| (d.path.clone(), d.diff.clone())));
    let max_remarks = mentor.project.as_ref().map_or(3, |p| p.max_remarks);
    Ok(PreparedReview {
        workspace,
        request: LlmRequest {
            model: model.to_owned(),
            messages: vec![
                ChatMessage::new(Role::System, prompt.system),
                ChatMessage::new(Role::User, prompt.user),
            ],
            schema_name: "revue".to_owned(),
            schema: project_remarks_schema(),
            effort: models.effort(mentor.model),
        },
        map,
        sent,
        level: mentor.entry_level,
        max_remarks: usize::try_from(max_remarks).unwrap_or(3),
    })
}

/// Citation d'une revue valide : longueur, texte envoyé (carte, fichier clé ou diff)
/// et fichier du projet, comme la garde de l'IDE.
fn quote_valid(prepared: &PreparedReview, path: &str, quote: &str) -> bool {
    let chars = quote.chars().count();
    let in_sent = prepared.map.contains(quote)
        || prepared
            .sent
            .iter()
            .any(|(p, text)| p == path && text.contains(quote));
    (3..=120).contains(&chars) && in_sent && prepared.workspace.locate(path, quote).is_some()
}

fn about_name(about: About) -> &'static str {
    about.as_str()
}

fn quotes_label(draft: &ProjectRemarkDraft) -> Option<String> {
    (!draft.quotes.is_empty()).then(|| {
        draft
            .quotes
            .iter()
            .map(|q| format!("{} : {}", q.path, q.quote))
            .collect::<Vec<_>>()
            .join(" · ")
    })
}

/// La bonne remarque : notion, fichier cité attendu, `about` attendu.
fn expected_remark(case: &Case, review: &ReviewCase, remark: &ProjectRemarkDraft) -> bool {
    super::run::matches_any(&remark.message, &case.must_mention).is_some()
        && (review.cited_files.is_empty()
            || remark
                .quotes
                .iter()
                .any(|q| review.cited_files.contains(&q.path)))
        && review.about.is_none_or(|about| remark.about == about)
}

pub(super) async fn run_review(
    client: &dyn LlmClient,
    registry: &Registry,
    models: &Models,
    case: &Case,
    review: &ReviewCase,
    spend: &mut Spend,
) -> CaseResult {
    let mut result = result_for(case);
    let (mentor, model) = match project_mentor(registry, models, &mut result, &case.expected_mentor)
    {
        Ok(found) => found,
        Err(message) => return fail(result, message),
    };
    let prepared = match prepare_review(mentor, &model, models, review) {
        Ok(prepared) => prepared,
        Err(message) => return fail(result, message),
    };
    result.notes.push(if review.sample.after.is_empty() {
        format!("projet `{}`, première revue", review.sample.name)
    } else {
        format!(
            "projet `{}`, revue suivante (diff de {})",
            review.sample.name,
            review.sample.after.len()
        )
    });
    let mut level_result = LevelResult::skipped(prepared.level, Outcome::Nothing);
    if spend.exhausted() {
        level_result.outcome = Outcome::BudgetStopped;
        result.levels.push(level_result);
        return result;
    }
    let sent: Vec<(&str, &str)> = prepared
        .sent
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let finder =
        |path: &str, quote: &str| prepared.workspace.locate(path, quote).map(|(_, text)| text);
    let rules = ProjectRules {
        level: remark_level(prepared.level),
        max_remarks: prepared.max_remarks,
        sent: &sent,
        map: &prepared.map,
        locate: &finder,
    };
    let started = Instant::now();
    match ask_for_project_remarks(client, prepared.request.clone(), &rules).await {
        Ok(asked) => {
            level_result.cost_usd = asked.cost_usd;
            level_result.drafts = asked
                .drafts
                .iter()
                .map(|d| Draft {
                    message: d.message.clone(),
                    quote: quotes_label(d),
                    rejected: validate_project(vec![d.clone()], &rules)
                        .rejected
                        .first()
                        .copied()
                        .or_else(|| {
                            (!asked.remarks.iter().any(|r| r.message == d.message))
                                .then_some("plafond de remarques")
                        }),
                })
                .collect();
            let quotes: Vec<(&str, &str)> = asked
                .drafts
                .iter()
                .flat_map(|d| d.quotes.iter().map(|q| (q.path.as_str(), q.quote.as_str())))
                .collect();
            let valid = quotes
                .iter()
                .filter(|(path, quote)| quote_valid(&prepared, path, quote))
                .count();
            level_result.valid_quotes = Some((valid, quotes.len()));
            level_result.right_level = (
                asked
                    .drafts
                    .iter()
                    .filter(|d| d.level == rules.level)
                    .count(),
                asked.drafts.len(),
            );
            level_result.shown = asked.remarks.iter().map(|r| r.message.clone()).collect();
            for remark in &asked.remarks {
                let cited: Vec<&str> = remark.quotes.iter().map(|q| q.path.as_str()).collect();
                result.notes.push(format!(
                    "remarque `{}` · cite {}",
                    about_name(remark.about),
                    if cited.is_empty() {
                        "aucun fichier".to_owned()
                    } else {
                        cited.join(", ")
                    }
                ));
            }
            finish(&mut level_result, case);
            // La notion compte dans la bonne remarque, qui cite le bon fichier.
            level_result.mentions = asked
                .remarks
                .iter()
                .any(|r| expected_remark(case, review, r));
        }
        Err(err) => {
            fill_error(&mut level_result, err);
            finish(&mut level_result, case);
        }
    }
    level_result.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    spend.add(level_result.cost_usd);
    result.levels.push(level_result);
    result
}

/* ---------- Voisinage ---------- */

/// Titulaire d'une section d'un projet d'exemple, par le casting de l'IDE.
fn titular<'a>(registry: &'a Registry, case: &Case) -> Option<&'a Mentor> {
    let all = sections(case.language, &case.content);
    let section = all.iter().find(|s| s.id == case.section)?;
    let text = section.text(&case.content);
    let casting = cast(
        registry,
        &CastRequest {
            path: &case.file,
            language: case.language.as_str(),
            headings: section_headings(case.language, Some(section)),
            text: &text,
            forced: None,
        },
    );
    registry.get(casting.titular.as_deref()?)
}

/// Requête d'une passe sur la section du cas ; avec `widened`, le voisinage part
/// avec elle (contexte `neighborhood`).
pub(super) fn neighborhood_request(
    workspace: &Workspace,
    mentor: &Mentor,
    model: &str,
    models: &Models,
    case: &Case,
    widened: bool,
) -> Option<(LlmRequest, String)> {
    let all = sections(case.language, &case.content);
    let section = all.iter().find(|s| s.id == case.section)?;
    let text = section.text(&case.content);
    let diff = hodegos_core::changed_sections(case.language, "", &case.content)
        .into_iter()
        .find(|c| c.section.id == section.id)
        .map(|c| c.diff)
        .unwrap_or_default();
    let around = widened.then(|| {
        neighborhood(
            &workspace.project,
            &workspace.session,
            &workspace.cache,
            &workspace.settings.exclude,
            &case.file,
            &case.content,
            DEFAULT_NEIGHBORHOOD_CAP,
        )
    });
    let extra = around.as_ref().map(|around| Extra {
        neighbors: around
            .neighbors
            .iter()
            .map(|n| NeighborText {
                path: &n.path,
                relation: n.relation.label(),
                text: &n.text,
            })
            .collect(),
        map: None,
    });
    let prompt = section_prompt(&SectionRequest {
        mentor,
        level: *case.levels.first()?,
        path: &case.file,
        language: case.language.as_str(),
        content: &case.content,
        section,
        diff: &diff,
        thread: None,
        extra: extra.as_ref(),
    });
    let schema = if widened {
        remarks_schema_with_see_also()
    } else {
        remarks_schema()
    };
    Some((
        LlmRequest {
            model: model.to_owned(),
            messages: vec![
                ChatMessage::new(Role::System, prompt.system),
                ChatMessage::new(Role::User, prompt.user),
            ],
            schema_name: "remarques".to_owned(),
            schema,
            effort: models.effort(mentor.model),
        },
        text,
    ))
}

pub(super) async fn run_neighborhood(
    client: &dyn LlmClient,
    registry: &Registry,
    models: &Models,
    case: &Case,
    sample: &Sample,
    spend: &mut Spend,
) -> CaseResult {
    let mut result = result_for(case);
    let Some(found) = titular(registry, case) else {
        return fail(result, "aucun mentor pour cette section");
    };
    result.mentor = Some(found.id.clone());
    result.tier = Some(found.model);
    let Some(model) = models.get(found.model) else {
        return fail(
            result,
            format!("modèle « {} » non configuré", tier_name(found.model)),
        );
    };
    result.model = Some(model.to_owned());
    let workspace = match Workspace::new(sample) {
        Ok(workspace) => workspace,
        Err(message) => return fail(result, message),
    };
    result
        .notes
        .push(format!("projet `{}`, `{}`", sample.name, case.file));
    let level = case.levels.first().copied().unwrap_or(Level::Indice);
    for (scope, widened) in [
        (ContextScope::Section, false),
        (ContextScope::Neighborhood, true),
    ] {
        let mut mentor = found.clone();
        mentor.context = scope;
        let mut level_result = LevelResult::skipped(level, Outcome::BudgetStopped);
        level_result.context = Some(if widened { "voisinage" } else { "section" });
        if spend.exhausted() {
            result.levels.push(level_result);
            continue;
        }
        let Some((request, text)) =
            neighborhood_request(&workspace, &mentor, model, models, case, widened)
        else {
            return fail(result, format!("section {} introuvable", case.section));
        };
        let rules = Rules {
            section_text: &text,
            level: remark_level(level),
            max_remarks: usize::from(mentor.max_remarks),
        };
        let started = Instant::now();
        let (mut played, _) = first_level(client, request, &rules, level).await;
        played.context = level_result.context;
        played.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
        finish(&mut played, case);
        spend.add(played.cost_usd);
        result.levels.push(played);
    }
    result
}

/* ---------- Réévaluation ---------- */

pub(super) struct PreparedReevaluation {
    pub(super) workspace: Workspace,
    pub(super) request: LlmRequest,
    pub(super) map: String,
    pub(super) sent: Vec<(String, String)>,
    pub(super) gone: usize,
}

pub(super) fn prepare_reevaluation(
    mentor: &Mentor,
    model: &str,
    models: &Models,
    level: Level,
    reevaluation: &ReevaluationCase,
) -> Result<PreparedReevaluation, String> {
    let mut workspace = Workspace::new(&reevaluation.sample)?;
    for (path, quote) in &reevaluation.quotes {
        if workspace.locate(path, quote).is_none() {
            return Err(format!("citation absente de {path} : {quote}"));
        }
    }
    workspace.write(&reevaluation.sample.after)?;
    let (gone, kept): (Vec<_>, Vec<_>) = reevaluation
        .quotes
        .iter()
        .partition(|(path, quote)| workspace.locate(path, quote).is_none());
    if gone.is_empty() {
        return Err("aucun passage cité ne disparaît avec after/".to_owned());
    }
    let map = workspace.map();
    let keys = key_files(
        &workspace.project,
        &workspace.session,
        &workspace.cache,
        &|path| mentor.is_key_file(path),
        DEFAULT_KEY_FILES_CAP,
    );
    let cited: Vec<(String, String, String)> = kept
        .iter()
        .filter_map(|(path, quote)| {
            let (id, text) = workspace.locate(path, quote)?;
            let text = if Language::of(path) == Language::Markdown {
                strip_html_comments(&text)
            } else {
                text
            };
            Some((path.clone(), format!("section {id}"), mask_secrets(&text).0))
        })
        .collect();
    let gone_masked: Vec<(String, String)> = gone
        .iter()
        .map(|(path, quote)| (path.clone(), mask_secrets(quote).0))
        .collect();
    let quote = gone_masked
        .iter()
        .map(|(_, q)| q.as_str())
        .collect::<Vec<_>>()
        .join(" · ");
    let message = mask_secrets(&reevaluation.message).0;
    let follow_up = FollowUp {
        kind: FollowUpKind::Reply,
        quote: &quote,
        levels_given: vec![level],
        entries: vec![(Author::Mentor, message.as_str())],
    };
    let prompt = reevaluation_prompt(&ReevaluationInput {
        mentor,
        level,
        map: &map,
        key_files: keys
            .files
            .iter()
            .map(|f| FileText {
                path: &f.path,
                form: &f.form,
                text: &f.text,
            })
            .collect(),
        cited: cited
            .iter()
            .map(|(path, form, text)| FileText { path, form, text })
            .collect(),
        gone: gone_masked
            .iter()
            .map(|(p, q)| (p.as_str(), q.as_str()))
            .collect(),
        follow_up: &follow_up,
    });
    let mut sent: Vec<(String, String)> = keys
        .files
        .iter()
        .map(|f| (f.path.clone(), f.text.clone()))
        .collect();
    sent.extend(cited.iter().map(|(p, _, t)| (p.clone(), t.clone())));
    Ok(PreparedReevaluation {
        workspace,
        request: LlmRequest {
            model: model.to_owned(),
            messages: vec![
                ChatMessage::new(Role::System, prompt.system),
                ChatMessage::new(Role::User, prompt.user),
            ],
            schema_name: "reevaluation".to_owned(),
            schema: reevaluation_schema(),
            effort: models.effort(mentor.model),
        },
        map,
        sent,
        gone: gone.len(),
    })
}

pub(super) async fn run_reevaluation(
    client: &dyn LlmClient,
    registry: &Registry,
    models: &Models,
    case: &Case,
    reevaluation: &ReevaluationCase,
    spend: &mut Spend,
) -> CaseResult {
    let mut result = result_for(case);
    let (mentor, model) = match project_mentor(registry, models, &mut result, &case.expected_mentor)
    {
        Ok(found) => found,
        Err(message) => return fail(result, message),
    };
    let level = case.levels.first().copied().unwrap_or(Level::Indice);
    let prepared = match prepare_reevaluation(mentor, &model, models, level, reevaluation) {
        Ok(prepared) => prepared,
        Err(message) => return fail(result, message),
    };
    result.notes.push(format!(
        "projet `{}`, {} passage(s) disparu(s) sur {}, verdict attendu `{}`",
        reevaluation.sample.name,
        prepared.gone,
        reevaluation.quotes.len(),
        reevaluation.verdict.as_str()
    ));
    let mut level_result = LevelResult::skipped(level, Outcome::Nothing);
    if spend.exhausted() {
        level_result.outcome = Outcome::BudgetStopped;
        result.levels.push(level_result);
        return result;
    }
    let sent: Vec<(&str, &str)> = prepared
        .sent
        .iter()
        .map(|(p, t)| (p.as_str(), t.as_str()))
        .collect();
    let finder =
        |path: &str, quote: &str| prepared.workspace.locate(path, quote).map(|(_, text)| text);
    let rules = ProjectRules {
        level: remark_level(level),
        max_remarks: 1,
        sent: &sent,
        map: &prepared.map,
        locate: &finder,
    };
    let started = Instant::now();
    match ask_for_reevaluation(client, prepared.request.clone(), &rules).await {
        Ok((verdict, cost)) => {
            level_result.cost_usd = cost;
            let (got, message, quote, rejected) = match verdict {
                Ok(Reevaluated::Keep(quotes)) => {
                    let label = quotes
                        .iter()
                        .map(|q| format!("{} : {}", q.path, q.quote))
                        .collect::<Vec<_>>()
                        .join(" · ");
                    // Maintenue sans citation valide : plus rien à quoi l'ancrer.
                    let anchored = !quotes.is_empty();
                    (
                        anchored.then_some(Verdict::Keep),
                        "Remarque maintenue.".to_owned(),
                        (!label.is_empty()).then_some(label),
                        (!anchored).then_some("maintenue sans citation valide"),
                    )
                }
                Ok(Reevaluated::Withdraw(reason)) => (Some(Verdict::Withdraw), reason, None, None),
                Err(reason) => (None, String::new(), None, Some(reason)),
            };
            if rejected.is_none() {
                level_result.shown = vec![message.clone()];
            }
            level_result.drafts = vec![Draft {
                message,
                quote,
                rejected,
            }];
            level_result.right_level = (1, 1);
            result.notes.push(format!(
                "verdict obtenu : `{}`",
                got.map_or("écarté", Verdict::as_str)
            ));
            finish(&mut level_result, case);
            level_result.mentions = got == Some(reevaluation.verdict);
        }
        Err(err) => {
            fill_error(&mut level_result, err);
        }
    }
    level_result.leaks_drafts = leaks(
        level_result.drafts.iter().map(|d| d.message.as_str()),
        &case.must_not,
    );
    level_result.duration_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    spend.add(level_result.cost_usd);
    result.levels.push(level_result);
    result
}

/// Taille, en caractères, des prompts d'un cas sur projet, sans appel au modèle.
pub(super) fn prompt_chars(
    registry: &Registry,
    case: &Case,
) -> Option<(hodegos_mentors::ModelTier, usize)> {
    let size = |request: &LlmRequest| -> usize {
        request
            .messages
            .iter()
            .map(|m| m.content.chars().count())
            .sum()
    };
    // Un modèle fictif suffit : seule la taille compte.
    let models = Models {
        fast: Some(String::new()),
        strong: Some(String::new()),
        ..Models::default()
    };
    match &case.kind {
        Kind::Pass => None,
        Kind::Review(review) => {
            let mentor = registry
                .get(&case.expected_mentor)
                .filter(|m| m.scopes.contains(&Scope::Project))?;
            let prepared = prepare_review(mentor, "", &models, review).ok()?;
            Some((mentor.model, size(&prepared.request)))
        }
        Kind::Reevaluation(reevaluation) => {
            let mentor = registry
                .get(&case.expected_mentor)
                .filter(|m| m.scopes.contains(&Scope::Project))?;
            let level = *case.levels.first()?;
            let prepared = prepare_reevaluation(mentor, "", &models, level, reevaluation).ok()?;
            Some((mentor.model, size(&prepared.request)))
        }
        Kind::Neighborhood(sample) => {
            let mentor = titular(registry, case)?;
            let workspace = Workspace::new(sample).ok()?;
            let chars = [false, true]
                .into_iter()
                .filter_map(|widened| {
                    neighborhood_request(&workspace, mentor, "", &models, case, widened)
                })
                .map(|(request, _)| size(&request))
                .sum();
            Some((mentor.model, chars))
        }
    }
}
