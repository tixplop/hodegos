//! Carte du projet (phase 9a) : calculée sans modèle au lancement, tenue à jour après
//! chaque enregistrement et chaque changement vu sur le disque, persistée dans
//! `<id-projet>/map.json` ; réglages propres au projet dans `<id-projet>/context.json`.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use hodegos_core::map::{
    BlockId, DEFAULT_MAP_CAP, DEFAULT_NEIGHBORHOOD_CAP, Exclusions, MAX_MAP_CAP, MIN_MAP_CAP,
    MapCache, MapSettings, Neighborhood, ProjectMap, build_map, neighborhood,
};
use hodegos_core::{RelPath, UserId};
use jiff::Timestamp;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;
use tokio::sync::mpsc;

use crate::AppState;
use crate::protocol::{ErrorCode, MapReason, ProjectMapInfo, ServerMessage};
use crate::settings::Refused;
use crate::store::write_json;

/// Regroupement des changements avant un recalcul.
const SETTLE: Duration = Duration::from_millis(500);
/// Dossiers observés au plus pour la carte.
const MAX_WATCHED_DIRS: usize = 5_000;
/// Version de `map.json` : la 2 garde les imports (phase 9b) ; une autre est ignorée.
const MAP_FILE_VERSION: u32 = 2;
/// Bornes du plafond du voisinage.
const NEIGHBORHOOD_CAPS: std::ops::RangeInclusive<usize> = 500..=16_000;

/// Réglages de `[context]` dans `config.toml`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ContextOptions {
    /// Plafond de la carte, en jetons estimés.
    pub map_cap: usize,
    /// Plafond du voisinage d'un fichier (phase 9b).
    pub neighborhood_cap: usize,
}

impl Default for ContextOptions {
    fn default() -> Self {
        Self {
            map_cap: DEFAULT_MAP_CAP,
            neighborhood_cap: DEFAULT_NEIGHBORHOOD_CAP,
        }
    }
}

#[derive(Deserialize, Default)]
struct RawConfig {
    #[serde(default)]
    context: RawContext,
}

#[derive(Deserialize, Default)]
struct RawContext {
    map_cap: Option<usize>,
    neighborhood_cap: Option<usize>,
}

/// Lit `[context]` dans `config.toml` ; une valeur illisible ou hors bornes laisse la
/// valeur par défaut, avec la raison.
pub fn load_context(config_dir: &Path) -> (ContextOptions, Option<String>) {
    let path = config_dir.join("config.toml");
    let Ok(text) = fs::read_to_string(&path) else {
        return (ContextOptions::default(), None);
    };
    let raw: RawConfig = match toml::from_str(&text) {
        Ok(raw) => raw,
        Err(err) => {
            return (
                ContextOptions::default(),
                Some(format!(
                    "{} : [context] illisible ({}), valeurs par défaut",
                    path.display(),
                    err.message()
                )),
            );
        }
    };
    let mut options = ContextOptions::default();
    let mut warnings = Vec::new();
    match raw.context.map_cap {
        Some(cap) if (MIN_MAP_CAP..=MAX_MAP_CAP).contains(&cap) => options.map_cap = cap,
        Some(cap) => warnings.push(format!(
            "[context] map_cap = {cap} : de {MIN_MAP_CAP} à {MAX_MAP_CAP}, {DEFAULT_MAP_CAP} retenu"
        )),
        None => {}
    }
    match raw.context.neighborhood_cap {
        Some(cap) if NEIGHBORHOOD_CAPS.contains(&cap) => options.neighborhood_cap = cap,
        Some(cap) => warnings.push(format!(
            "[context] neighborhood_cap = {cap} : de {} à {}, {DEFAULT_NEIGHBORHOOD_CAP} retenu",
            NEIGHBORHOOD_CAPS.start(),
            NEIGHBORHOOD_CAPS.end()
        )),
        None => {}
    }
    (
        options,
        (!warnings.is_empty()).then(|| warnings.join(" ; ")),
    )
}

/// Réglages de la carte propres au projet (`context.json`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Prefs {
    /// `None` : le plafond de `config.toml`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    map_cap: Option<usize>,
    /// `None` : les blocs désactivés par défaut (Tests).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    disabled_blocks: Option<Vec<BlockId>>,
    #[serde(default)]
    exclude: Vec<String>,
}

/// Contenu de `map.json`.
#[derive(Serialize, Deserialize)]
struct MapFile {
    version: u32,
    updated_at: String,
    reason: MapReason,
    cache: MapCache,
}

struct Built {
    map: ProjectMap,
    updated_at: Timestamp,
    reason: MapReason,
}

struct Inner {
    cache: MapCache,
    /// Le cache lu dans `map.json` doit encore être confronté au disque.
    stale: bool,
    prefs: Prefs,
    built: Option<Built>,
}

/// Chemins touchés à prendre en compte, et pourquoi.
pub(crate) struct Touch {
    paths: Vec<RelPath>,
    reason: MapReason,
}

pub(crate) struct MapState {
    /// `<id-projet>/` ; `None` : rien n'est persisté.
    dir: Option<PathBuf>,
    default_cap: usize,
    neighborhood_cap: usize,
    inner: Mutex<Inner>,
    touches: mpsc::UnboundedSender<Touch>,
}

impl MapState {
    pub(crate) fn open(
        dir: Option<PathBuf>,
        options: ContextOptions,
    ) -> (Self, mpsc::UnboundedReceiver<Touch>) {
        let read = |name: &str| {
            dir.as_ref()
                .and_then(|dir| fs::read_to_string(dir.join(name)).ok())
        };
        let prefs = read("context.json")
            .and_then(|text| match serde_json::from_str::<Prefs>(&text) {
                Ok(prefs) => Some(prefs),
                Err(err) => {
                    tracing::warn!(error = %err, "context.json illisible, réglages par défaut");
                    None
                }
            })
            .unwrap_or_default();
        let cache = read("map.json")
            .and_then(|text| serde_json::from_str::<MapFile>(&text).ok())
            .filter(|file| file.version == MAP_FILE_VERSION)
            .map(|file| file.cache)
            .unwrap_or_default();
        let (touches, receiver) = mpsc::unbounded_channel();
        let state = Self {
            dir,
            default_cap: options.map_cap,
            neighborhood_cap: options.neighborhood_cap,
            inner: Mutex::new(Inner {
                cache,
                stale: true,
                prefs,
                built: None,
            }),
            touches,
        };
        (state, receiver)
    }

    /// Signale des chemins modifiés ; la carte sera recalculée au calme.
    pub(crate) fn touched(&self, paths: Vec<RelPath>, reason: MapReason) {
        let _ = self.touches.send(Touch { paths, reason });
    }

    fn settings(&self, prefs: &Prefs) -> MapSettings {
        let exclude = Exclusions::new(&prefs.exclude).unwrap_or_else(|reason| {
            tracing::warn!(%reason, "exclusions de la carte ignorées");
            Exclusions::default()
        });
        MapSettings {
            cap: prefs.map_cap.unwrap_or(self.default_cap),
            disabled: prefs
                .disabled_blocks
                .as_ref()
                .map(|blocks| blocks.iter().copied().collect())
                .unwrap_or_else(|| MapSettings::default().disabled),
            exclude,
        }
    }

    fn info(&self, inner: &Inner, built: &Built) -> ProjectMapInfo {
        ProjectMapInfo {
            blocks: built.map.blocks.clone(),
            tokens: built.map.tokens,
            cap: built.map.cap,
            default_cap: self.default_cap,
            exclude: inner.prefs.exclude.clone(),
            updated_at: built.updated_at.to_string(),
            reason: built.reason,
            text: built.map.text.clone(),
        }
    }

    fn persist(&self, inner: &Inner) {
        let (Some(dir), Some(built)) = (&self.dir, &inner.built) else {
            return;
        };
        let file = MapFile {
            version: MAP_FILE_VERSION,
            updated_at: built.updated_at.to_string(),
            reason: built.reason,
            cache: inner.cache.clone(),
        };
        if let Err(err) = write_json(&dir.join("map.json"), &file) {
            tracing::warn!(error = %err, "map.json non écrit");
        }
    }
}

fn updated(built: &Built) -> ServerMessage {
    ServerMessage::MapUpdated {
        tokens: built.map.tokens,
        cap: built.map.cap,
        updated_at: built.updated_at.to_string(),
        reason: built.reason,
    }
}

/// Recalcule la carte (en entier si `paths` est `None`) ; renvoie `map_updated` si
/// elle a changé. Bloquant : à appeler hors des fils de tokio.
fn recompute(
    state: &AppState,
    paths: Option<&[RelPath]>,
    reason: MapReason,
) -> Option<ServerMessage> {
    let map = &state.map;
    let session = state.project.session(UserId::local());
    let Ok(mut inner) = map.inner.lock() else {
        return None;
    };
    let settings = map.settings(&inner.prefs);
    let full = paths.is_none() || inner.stale;
    let result = match paths {
        Some(paths) if !full => {
            inner
                .cache
                .refresh_paths(&state.project, &session, &settings.exclude, paths)
        }
        _ => inner
            .cache
            .refresh(&state.project, &session, &settings.exclude),
    };
    let refresh = match result {
        Ok(refresh) => refresh,
        Err(err) => {
            tracing::warn!(error = %err, "carte du projet non recalculée");
            return None;
        }
    };
    inner.stale = false;
    for dir in &refresh.new_dirs {
        if let Err(err) = state.watcher.track_dir(&session, dir) {
            tracing::debug!(%dir, error = %err, "dossier non observé");
        }
    }
    let rebuilt = build_map(state.project.root(), &inner.cache, &settings);
    let unchanged = inner
        .built
        .as_ref()
        .is_some_and(|b| b.map == rebuilt && !refresh.changed);
    if unchanged && reason != MapReason::Settings {
        return None;
    }
    let built = Built {
        map: rebuilt,
        updated_at: Timestamp::now(),
        reason,
    };
    let message = updated(&built);
    inner.built = Some(built);
    map.persist(&inner);
    tracing::info!(
        raison = ?reason,
        jetons = inner.built.as_ref().map_or(0, |b| b.map.tokens),
        relus = refresh.extracted,
        "carte du projet"
    );
    Some(message)
}

/// Carte calculée au besoin ; bloquant.
fn current(state: &AppState) -> Option<ProjectMapInfo> {
    let built = state
        .map
        .inner
        .lock()
        .ok()
        .is_some_and(|inner| inner.built.is_some());
    // Premier calcul : personne ne l'a encore affichée, rien n'est poussé.
    if !built {
        recompute(state, None, MapReason::Open);
    }
    let inner = state.map.inner.lock().ok()?;
    let built = inner.built.as_ref()?;
    Some(state.map.info(&inner, built))
}

/// Voisinage de `path` (contenu enregistré `content`) et, si `with_map`, texte et
/// jetons de la carte, d'après la carte à jour ; bloquant.
pub(crate) fn widen(
    state: &AppState,
    path: &str,
    content: &str,
    with_map: bool,
) -> (Neighborhood, Option<(String, usize)>) {
    let built = current(state);
    let Ok(inner) = state.map.inner.lock() else {
        return (Neighborhood::default(), None);
    };
    let session = state.project.session(UserId::local());
    let settings = state.map.settings(&inner.prefs);
    let around = neighborhood(
        &state.project,
        &session,
        &inner.cache,
        &settings.exclude,
        path,
        content,
        state.map.neighborhood_cap,
    );
    let map = with_map
        .then_some(built)
        .flatten()
        .map(|info| (info.text, info.tokens));
    (around, map)
}

/// Calcul initial, observation des dossiers du projet, puis recalcul après chaque
/// changement, au calme.
pub(crate) async fn run(state: Arc<AppState>, mut touches: mpsc::UnboundedReceiver<Touch>) {
    let mut changes = state.changes.subscribe();
    let first = state.clone();
    let _ = tokio::task::spawn_blocking(move || {
        watch_all(&first);
        current(&first);
    })
    .await;
    loop {
        let mut batch: Vec<Touch> = Vec::new();
        let mut full = false;
        tokio::select! {
            touch = touches.recv() => match touch {
                Some(touch) => batch.push(touch),
                None => return,
            },
            changed = changes.recv() => match changed {
                Ok(paths) => batch.push(Touch { paths: paths.to_vec(), reason: MapReason::External }),
                Err(RecvError::Lagged(_)) => full = true,
                Err(RecvError::Closed) => return,
            },
        }
        tokio::time::sleep(SETTLE).await;
        while let Ok(touch) = touches.try_recv() {
            batch.push(touch);
        }
        loop {
            match changes.try_recv() {
                Ok(paths) => batch.push(Touch {
                    paths: paths.to_vec(),
                    reason: MapReason::External,
                }),
                Err(tokio::sync::broadcast::error::TryRecvError::Lagged(_)) => full = true,
                Err(_) => break,
            }
        }
        let reason = if batch.iter().any(|t| t.reason == MapReason::Save) {
            MapReason::Save
        } else {
            MapReason::External
        };
        let paths: Vec<RelPath> = batch
            .into_iter()
            .flat_map(|t| t.paths)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let worker = state.clone();
        let message = tokio::task::spawn_blocking(move || {
            recompute(&worker, (!full).then_some(paths.as_slice()), reason)
        })
        .await
        .ok()
        .flatten();
        if let Some(message) = message {
            let _ = state.events.send(message);
        }
    }
}

/// Observe tous les dossiers non ignorés : une modification faite ailleurs, même dans
/// un dossier jamais ouvert, met la carte à jour.
fn watch_all(state: &AppState) {
    let session = state.project.session(UserId::local());
    let dirs = match state.project.list_all_dirs(&session, MAX_WATCHED_DIRS) {
        Ok((dirs, truncated)) => {
            if truncated {
                tracing::warn!(
                    "plus de {MAX_WATCHED_DIRS} dossiers : la carte ne suit pas les suivants"
                );
            }
            dirs
        }
        Err(err) => {
            tracing::warn!(error = %err, "dossiers du projet non listés");
            Vec::new()
        }
    };
    for dir in std::iter::once(RelPath::root()).chain(dirs) {
        if let Err(err) = state.watcher.track_dir(&session, &dir) {
            tracing::debug!(%dir, error = %err, "dossier non observé");
        }
    }
}

/// `project_map_get`.
pub(crate) async fn get(state: Arc<AppState>) -> Result<ServerMessage, Refused> {
    tokio::task::spawn_blocking(move || current(&state))
        .await
        .ok()
        .flatten()
        .map(|info| ServerMessage::ProjectMap(Box::new(info)))
        .ok_or_else(|| Refused {
            code: ErrorCode::Internal,
            message: "carte du projet indisponible (voir le journal)".to_owned(),
        })
}

/// `project_map_set` : valide, écrit `context.json`, recalcule tout (les exclusions ont
/// pu changer) et prévient toutes les fenêtres.
pub(crate) async fn set(
    state: Arc<AppState>,
    map_cap: usize,
    disabled_blocks: Vec<BlockId>,
    exclude: Vec<String>,
) -> Result<ServerMessage, Refused> {
    let invalid = |message: String| Refused {
        code: ErrorCode::InvalidSettings,
        message,
    };
    if !(MIN_MAP_CAP..=MAX_MAP_CAP).contains(&map_cap) {
        return Err(invalid(format!(
            "plafond de la carte : de {MIN_MAP_CAP} à {MAX_MAP_CAP} jetons"
        )));
    }
    if disabled_blocks.contains(&BlockId::Header) {
        return Err(invalid(
            "l'en-tête de la carte ne se désactive pas".to_owned(),
        ));
    }
    let exclude: Vec<String> = exclude.iter().map(|p| p.trim().to_owned()).collect();
    Exclusions::new(&exclude).map_err(invalid)?;
    let prefs = Prefs {
        map_cap: (map_cap != state.map.default_cap).then_some(map_cap),
        disabled_blocks: Some(
            disabled_blocks
                .into_iter()
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect(),
        ),
        exclude,
    };
    if let Some(dir) = &state.map.dir
        && let Err(err) = write_json(&dir.join("context.json"), &prefs)
    {
        tracing::warn!(error = %err, "context.json non écrit");
        return Err(Refused {
            code: ErrorCode::Internal,
            message: format!("réglages de la carte non enregistrés : {err}"),
        });
    }
    if let Ok(mut inner) = state.map.inner.lock() {
        inner.prefs = prefs;
    }
    tokio::task::spawn_blocking(move || {
        if let Some(message) = recompute(&state, None, MapReason::Settings) {
            let _ = state.events.send(message);
        }
        current(&state)
    })
    .await
    .ok()
    .flatten()
    .map(|info| ServerMessage::ProjectMap(Box::new(info)))
    .ok_or_else(|| Refused {
        code: ErrorCode::Internal,
        message: "carte du projet indisponible (voir le journal)".to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plafond_lu_dans_config_toml() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(load_context(dir.path()), (ContextOptions::default(), None));
        fs::write(
            dir.path().join("config.toml"),
            "[context]\nmap_cap = 6000\n",
        )
        .unwrap();
        assert_eq!(load_context(dir.path()).0.map_cap, 6000);
        fs::write(dir.path().join("config.toml"), "[context]\nmap_cap = 10\n").unwrap();
        let (options, warning) = load_context(dir.path());
        assert_eq!(options.map_cap, DEFAULT_MAP_CAP);
        assert!(warning.unwrap().contains("map_cap = 10"));
        fs::write(dir.path().join("config.toml"), "[llm]\nfast = \"x\"\n").unwrap();
        assert_eq!(load_context(dir.path()).0.map_cap, DEFAULT_MAP_CAP);
        fs::write(
            dir.path().join("config.toml"),
            "[context]\nneighborhood_cap = 2000\n",
        )
        .unwrap();
        assert_eq!(load_context(dir.path()).0.neighborhood_cap, 2000);
        fs::write(
            dir.path().join("config.toml"),
            "[context]\nneighborhood_cap = 1\n",
        )
        .unwrap();
        let (options, warning) = load_context(dir.path());
        assert_eq!(options.neighborhood_cap, DEFAULT_NEIGHBORHOOD_CAP);
        assert!(warning.unwrap().contains("neighborhood_cap = 1"));
    }
}
