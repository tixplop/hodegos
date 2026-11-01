//! Réglages des mentors (phase 8e) : liste, lecture, enregistrement, personnalisation,
//! suppression d'une surcharge et prompt assemblé. Un mentor livré n'est jamais écrit ;
//! `.hodegos/mentors/` n'est créé qu'à l'enregistrement d'un mentor de projet. Le
//! rechargement à chaud existant applique le résultat.

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};

use hodegos_mentors::{
    EjectError, FieldError, Mentor, MentorFile, Origin, Parsed, eject, inventory, read_form,
    render, system_prompt, validate,
};

use crate::AppState;
use crate::protocol::{ErrorCode, MentorFileErrors, MentorFileInfo, MentorItem, ServerMessage};

/// Langage montré dans le prompt assemblé quand le mentor n'en déclare pas.
const EXAMPLE_LANGUAGE: &str = "rust";

/// Refus d'une demande, avec les erreurs champ par champ s'il y en a.
#[derive(Debug)]
pub(crate) struct Refused {
    pub(crate) code: ErrorCode,
    pub(crate) message: String,
    pub(crate) errors: Vec<FieldError>,
}

fn refused(code: ErrorCode, message: impl Into<String>) -> Refused {
    Refused {
        code,
        message: message.into(),
        errors: Vec::new(),
    }
}

fn invalid(errors: Vec<FieldError>) -> Refused {
    Refused {
        code: ErrorCode::MentorInvalid,
        message: hodegos_mentors::join(&errors),
        errors,
    }
}

fn file_id(file: &MentorFile) -> String {
    match &file.result {
        Ok(Parsed::Active(mentor)) => mentor.id.clone(),
        Ok(Parsed::Disabled { id }) => id.clone(),
        Err(_) => stem(&file.name),
    }
}

/// `architecte.md (livré)` ou `/…/architecte.md` → `architecte`.
fn stem(name: &str) -> String {
    let file = name.rsplit('/').next().unwrap_or(name);
    file.split(".md").next().unwrap_or(file).to_owned()
}

/// Fichiers lus, avec les doublons d'un même niveau en erreur, comme le chargeur.
fn files(state: &AppState) -> Vec<(MentorFile, Option<FieldError>)> {
    let mut seen: BTreeMap<(Origin, String), String> = BTreeMap::new();
    inventory(state.mentors.sources())
        .into_iter()
        .map(|file| {
            if file.result.is_err() {
                return (file, None);
            }
            let key = (file.origin, file_id(&file));
            let duplicate = seen.get(&key).map(|first| FieldError {
                field: "id".to_owned(),
                message: format!("id « {} » déjà défini au même niveau par {first}", key.1),
            });
            seen.entry(key).or_insert_with(|| file.name.clone());
            (file, duplicate)
        })
        .collect()
}

pub(crate) fn list(state: &AppState) -> ServerMessage {
    let mut by_id: BTreeMap<String, Vec<MentorFile>> = BTreeMap::new();
    let mut errors = Vec::new();
    for (file, duplicate) in files(state) {
        let id = file_id(&file);
        match (&file.result, duplicate) {
            (Err(field_errors), _) => errors.push(MentorFileErrors {
                origin: file.origin,
                path: file.name.clone(),
                id,
                errors: field_errors.clone(),
            }),
            (Ok(_), Some(duplicate)) => errors.push(MentorFileErrors {
                origin: file.origin,
                path: file.name.clone(),
                id,
                errors: vec![duplicate],
            }),
            (Ok(_), None) => by_id.entry(id).or_default().push(file),
        }
    }
    let items = by_id
        .into_iter()
        .filter_map(|(id, files)| {
            // Le niveau le plus précis l'emporte ; un mentor désactivé garde le nom du
            // niveau qu'il désactive.
            let winner = files.last()?;
            let shown: Option<&Mentor> = files.iter().rev().find_map(|f| match &f.result {
                Ok(Parsed::Active(mentor)) => Some(mentor.as_ref()),
                _ => None,
            });
            Some(MentorItem {
                name: shown.map_or_else(|| id.clone(), |m| m.name.clone()),
                initials: shown.map_or_else(String::new, |m| m.initials.clone()),
                origin: winner.origin,
                path: winner.path.as_ref().map(|p| p.display().to_string()),
                overrides: files[..files.len() - 1].iter().map(|f| f.origin).collect(),
                enabled: matches!(winner.result, Ok(Parsed::Active(_))),
                mode: shown.map(|m| m.mode),
                model: shown.map(|m| m.model),
                on_open: shown.is_some_and(|m| m.on_open),
                scopes: shown.map(|m| m.scopes.clone()).unwrap_or_default(),
                context: shown.map(|m| m.context).unwrap_or_default(),
                project: shown.and_then(|m| m.project.clone()),
                id,
            })
        })
        .collect();
    ServerMessage::Mentors { items, errors }
}

/// Fichier du mentor `id` au niveau `origin`, valide ou en erreur.
fn find(state: &AppState, id: &str, origin: Origin) -> Option<MentorFile> {
    files(state)
        .into_iter()
        .map(|(file, _)| file)
        .filter(|file| file.origin == origin)
        .find(|file| file_id(file) == id)
}

fn file_info(id: &str, file: MentorFile) -> ServerMessage {
    let (form, body) = match read_form(&file.text) {
        Ok((form, body)) => (Some(form), Some(body)),
        Err(_) => (None, None),
    };
    ServerMessage::MentorFile(Box::new(MentorFileInfo {
        id: id.to_owned(),
        origin: file.origin,
        path: file.path.as_ref().map(|p| p.display().to_string()),
        editable: file.origin != Origin::Builtin,
        errors: file.result.err().unwrap_or_default(),
        content: file.text,
        form,
        body,
    }))
}

pub(crate) fn get(state: &AppState, id: &str, origin: Origin) -> Result<ServerMessage, Refused> {
    let file = find(state, id, origin).ok_or_else(|| {
        refused(
            ErrorCode::NotFound,
            format!("mentor {id} ({origin}) introuvable"),
        )
    })?;
    Ok(file_info(id, file))
}

fn dir(state: &AppState, origin: Origin) -> Result<PathBuf, Refused> {
    if origin == Origin::Builtin {
        return Err(refused(
            ErrorCode::InvalidSettings,
            "un mentor livré n'est jamais modifié : personnalisez-le",
        ));
    }
    state
        .mentors
        .sources()
        .dir(origin)
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            refused(
                ErrorCode::InvalidSettings,
                format!("aucun dossier de mentors {origin}"),
            )
        })
}

/// Contenu à écrire : le fichier brut, ou le formulaire et la posture.
pub(crate) enum Content {
    Raw(String),
    Form(Box<hodegos_mentors::MentorForm>, String),
}

pub(crate) fn save(
    state: &AppState,
    origin: Origin,
    id: &str,
    content: Content,
) -> Result<ServerMessage, Refused> {
    let dir = dir(state, origin)?;
    let text = match content {
        Content::Raw(text) => text,
        Content::Form(form, body) => {
            render(&form, &body).map_err(|err| refused(ErrorCode::Internal, err.to_string()))?
        }
    };
    // Les règles du chargeur, à l'identique.
    let parsed = validate(&text, origin, &format!("{id}.md")).map_err(invalid)?;
    let written_id = match &parsed {
        Parsed::Active(mentor) => mentor.id.clone(),
        Parsed::Disabled { id } => id.clone(),
    };
    if written_id != id {
        return Err(invalid(vec![FieldError {
            field: "id".to_owned(),
            message: format!(
                "id « {written_id} » : l'identifiant d'un mentor ne change pas ; dupliquez-le pour en créer un autre"
            ),
        }]));
    }
    // Le fichier qui définit déjà cet id à ce niveau, sinon `<id>.md`, s'il est libre.
    let target = match find(state, id, origin).and_then(|f| f.path) {
        Some(path) => path,
        None => {
            let path = dir.join(format!("{id}.md"));
            if path.exists() {
                return Err(invalid(vec![FieldError {
                    field: "id".to_owned(),
                    message: format!("{} existe déjà et définit un autre mentor", path.display()),
                }]));
            }
            path
        }
    };
    write_atomic(&dir, &target, &text)
        .map_err(|err| refused(ErrorCode::Internal, format!("{} : {err}", target.display())))?;
    tracing::info!(id, %origin, path = %target.display(), "mentor enregistré");
    state.mentors.reload();
    get(state, id, origin)
}

/// Seule écriture de Hodegos dans `.hodegos/mentors/` : à la demande, en créant le
/// dossier si besoin.
fn write_atomic(dir: &Path, path: &Path, text: &str) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let mut file = tempfile::NamedTempFile::new_in(dir)?;
    file.write_all(text.as_bytes())?;
    file.as_file().sync_all()?;
    file.persist(path).map_err(|e| e.error)?;
    Ok(())
}

pub(crate) fn personalize(
    state: &AppState,
    id: &str,
    to: Origin,
) -> Result<ServerMessage, Refused> {
    let dir = dir(state, to)?;
    let registry = state.mentors.registry();
    eject(&registry, id, &dir).map_err(|err| match err {
        EjectError::Unknown(_) => refused(ErrorCode::NotFound, err.to_string()),
        EjectError::Exists(_) => refused(ErrorCode::AlreadyExists, err.to_string()),
        EjectError::Io { .. } => refused(ErrorCode::Internal, err.to_string()),
    })?;
    tracing::info!(id, %to, "mentor personnalisé");
    state.mentors.reload();
    get(state, id, to)
}

/// Supprime le fichier du niveau `origin` : le mentor revient au niveau inférieur.
pub(crate) fn delete(state: &AppState, id: &str, origin: Origin) -> Result<ServerMessage, Refused> {
    dir(state, origin)?;
    let path = find(state, id, origin)
        .and_then(|f| f.path)
        .ok_or_else(|| {
            refused(
                ErrorCode::NotFound,
                format!("mentor {id} ({origin}) introuvable"),
            )
        })?;
    fs::remove_file(&path)
        .map_err(|err| refused(ErrorCode::Internal, format!("{} : {err}", path.display())))?;
    tracing::info!(id, %origin, "surcharge de mentor supprimée");
    state.mentors.reload();
    Ok(list(state))
}

/// Cadre commun puis posture, au palier d'entrée du mentor.
pub(crate) fn prompt(
    state: &AppState,
    id: &str,
    content: Option<&str>,
) -> Result<ServerMessage, Refused> {
    let registry = state.mentors.registry();
    let draft;
    let mentor: &Mentor = match content {
        Some(text) => match validate(text, Origin::User, &format!("{id}.md")).map_err(invalid)? {
            Parsed::Active(mentor) => {
                draft = mentor;
                &draft
            }
            Parsed::Disabled { .. } => {
                return Err(refused(
                    ErrorCode::BadMessage,
                    "mentor désactivé : aucun prompt",
                ));
            }
        },
        None => registry
            .get(id)
            .ok_or_else(|| refused(ErrorCode::NotFound, format!("aucun mentor actif « {id} »")))?,
    };
    let language = mentor
        .triggers
        .languages
        .first()
        .map_or(EXAMPLE_LANGUAGE, String::as_str);
    Ok(ServerMessage::MentorPrompt {
        text: system_prompt(
            mentor,
            mentor.entry_level,
            language,
            false,
            !mentor.context.is_section(),
        ),
    })
}
