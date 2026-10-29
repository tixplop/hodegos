#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Paliers et conversation : `hint_next`, `remark_reply`, `remark_dismiss`, remarques
//! détachées. Avec `FakeLlmClient` : aucun appel réseau.

mod common;

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use common::{Socket, fake_llm, next, next_within, open_socket_with, project_in, send_raw};
use hodegos_llm::{BudgetConfig, FakeLlmClient, LlmError, LlmResponse};
use hodegos_server::{Llm, Server};
use serde_json::{Value, json};

const PATH: &str = "src/emprunts/reserver.ts";

const RESERVER: &str = "\
import { Emprunt, Objet } from \"../modele\";

export function reserver(objet: Objet, debut: Date, emprunts: Emprunt[]) {
  const dejaPris = emprunts.find((e) => e.debut === debut);
  return !dejaPris;
}
";

fn sample() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join(PATH);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, RESERVER).unwrap();
    dir
}

fn ok(content: Value, cost: f64) -> Result<LlmResponse, LlmError> {
    Ok(LlmResponse {
        content: content.to_string(),
        cost_usd: cost,
    })
}

/// Réponse de la passe : une remarque au palier indice sur la comparaison de dates.
fn first_remark() -> Result<LlmResponse, LlmError> {
    ok(
        json!({ "remarks": [{ "quote": "e.debut === debut", "level": "indice", "kind": "observation",
            "message": "Regardez comment cette ligne compare deux dates.", "confidence": 0.9 }] }),
        0.002,
    )
}

fn reply(level: &str, message: &str) -> Result<LlmResponse, LlmError> {
    ok(
        json!({ "replies": [{ "level": level, "message": message, "confidence": 0.8 }] }),
        0.001,
    )
}

/// Envoie une demande et renvoie sa réponse ; les messages spontanés reçus avant
/// sont rangés dans `pushed`.
async fn request_with(
    socket: &mut Socket,
    kind: &str,
    payload: Value,
    pushed: &mut Vec<Value>,
) -> Value {
    send_raw(
        socket,
        &json!({ "v": 1, "type": kind, "id": "c", "payload": payload }).to_string(),
    )
    .await;
    loop {
        let message = next(socket).await;
        if message["re"] == "c" {
            return message;
        }
        pushed.push(message);
    }
}

async fn request(socket: &mut Socket, kind: &str, payload: Value) -> Value {
    request_with(socket, kind, payload, &mut Vec::new()).await
}

/// Ouvre le fichier et consomme son `casting`.
async fn open(socket: &mut Socket) -> Value {
    let file = request(socket, "open_file", json!({ "path": PATH })).await;
    assert_eq!(file["type"], "file", "{file}");
    assert_eq!(next(socket).await["type"], "casting");
    file
}

/// Enregistre et renvoie les messages jusqu'au `status` qui clôt la passe.
async fn save(socket: &mut Socket, content: &str, base: &Value) -> Vec<Value> {
    let mut messages = Vec::new();
    let saved = request_with(
        socket,
        "save",
        json!({ "path": PATH, "content": content, "base_version": base }),
        &mut messages,
    )
    .await;
    assert_eq!(saved["type"], "saved", "{saved}");
    let version = saved["payload"]["version"].clone();
    messages.push(saved);
    loop {
        let message = next(socket).await;
        let done = message["type"] == "status";
        messages.push(message);
        if done {
            messages.push(json!({ "version": version }));
            return messages;
        }
    }
}

/// Ouvre le fichier, l'enregistre modifié et renvoie la remarque produite, avec la
/// version enregistrée.
async fn remark_after_save(socket: &mut Socket) -> (Value, Value) {
    let file = open(socket).await;
    let edited = RESERVER.replace("return !dejaPris;", "return dejaPris === undefined;");
    let messages = save(socket, &edited, &file["payload"]["version"]).await;
    let remarks = messages
        .iter()
        .find(|m| m["type"] == "remarks")
        .expect("aucune remarque");
    let version = messages.last().unwrap()["version"].clone();
    (remarks["payload"]["items"][0].clone(), version)
}

/// `remark_update` en réponse à la demande, puis le `status` qui suit.
async fn hint_next(socket: &mut Socket, id: &Value, confirm: bool) -> Value {
    let reply = request(
        socket,
        "hint_next",
        json!({ "remark_id": id, "confirm_over_budget": confirm }),
    )
    .await;
    assert_eq!(next(socket).await["type"], "status");
    reply
}

#[tokio::test]
async fn une_remarque_monte_de_l_indice_a_l_exemple_puis_s_arrete() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let (remark, _) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();

    fake.push(reply(
        "question",
        "Que renvoie new Date(1) === new Date(1) ?",
    ));
    let update = hint_next(&mut socket, &id, false).await;
    assert_eq!(update["type"], "remark_update", "{update}");
    let payload = &update["payload"];
    assert_eq!(payload["level"], "question");
    assert_eq!(payload["levels_given"], json!(["indice", "question"]));
    assert_eq!(payload["thread"].as_array().unwrap().len(), 2);
    assert_eq!(payload["thread"][1]["author"], "mentor");

    // Le prompt porte le fil, le palier demandé et la consigne propre au fil.
    let request = fake.requests().last().unwrap().clone();
    assert!(request.messages[0].content.contains("Suite d'une remarque"));
    let user = &request.messages[1].content;
    assert!(user.contains("<section id=\"fn:reserver\">"), "{user}");
    assert!(
        user.contains("<citation>e.debut === debut</citation>"),
        "{user}"
    );
    assert!(
        user.contains("<paliers_donnes>indice</paliers_donnes>"),
        "{user}"
    );
    assert!(user.ends_with("Palier demandé : question."), "{user}");
    assert_eq!(request.schema_name, "reponse");

    fake.push(reply(
        "piste",
        "=== compare des références d'objets ; cherchez comment comparer la valeur d'une date.",
    ));
    let update = hint_next(&mut socket, &id, false).await;
    assert_eq!(update["payload"]["level"], "piste");

    // Un exemple qui reprend le code de la section est écarté : la remarque ne bouge pas.
    fake.push(reply(
        "exemple",
        "```ts\nconst dejaPris = emprunts.find((e) => e.debut.getTime() === debut.getTime());\n```",
    ));
    let update = hint_next(&mut socket, &id, false).await;
    assert_eq!(update["type"], "remark_update");
    assert_eq!(update["payload"]["level"], "piste");
    assert_eq!(update["payload"]["thread"].as_array().unwrap().len(), 3);

    fake.push(reply(
        "exemple",
        "Un cas voisin :\n```ts\nconst a = { x: 1 };\nconst b = { x: 1 };\nconsole.log(a === b, a.x === b.x);\n```",
    ));
    let update = hint_next(&mut socket, &id, false).await;
    assert_eq!(update["payload"]["level"], "exemple");
    assert_eq!(
        update["payload"]["levels_given"],
        json!(["indice", "question", "piste", "exemple"])
    );

    // Au dernier palier, rien ne part vers le modèle.
    let calls = fake.requests().len();
    let refused = hint_next(&mut socket, &id, false).await;
    assert_eq!(refused["type"], "error");
    assert_eq!(refused["payload"]["code"], "bad_message");
    assert_eq!(fake.requests().len(), calls);

    // Le fil survit au redémarrage du serveur.
    drop(socket);
    let server = Server::bind(
        project_in(dir.path()),
        common::options(dir.path(), 0, fake_llm(fake.clone())),
    )
    .await
    .unwrap();
    let port = server.port();
    let cookie = common::session_cookie(&server).await;
    tokio::spawn(server.serve(std::future::pending()));
    let origin = format!("http://127.0.0.1:{port}");
    let mut socket = common::connect(port, Some(&origin), Some(&cookie))
        .await
        .unwrap();
    next(&mut socket).await;
    let file = open(&mut socket).await;
    let remarks = file["payload"]["remarks"].as_array().unwrap();
    assert_eq!(remarks[0]["level"], "exemple");
    assert_eq!(remarks[0]["thread"].as_array().unwrap().len(), 4);
}

#[tokio::test]
async fn une_reponse_relance_le_mentor_au_meme_palier() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let (remark, _) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();

    fake.push(reply(
        "indice",
        "Relisez ce que compare chaque côté de ===.",
    ));
    let answered = request(
        &mut socket,
        "remark_reply",
        json!({ "remark_id": id, "text": "  Je compare les deux dates, non ?  " }),
    )
    .await;
    assert_eq!(answered["type"], "remark_update", "{answered}");
    assert_eq!(answered["payload"]["status"], "answered");
    let thread = answered["payload"]["thread"].as_array().unwrap();
    assert_eq!(thread.len(), 2);
    assert_eq!(thread[1]["author"], "user");
    assert_eq!(thread[1]["text"], "Je compare les deux dates, non ?");

    let relaunched = next(&mut socket).await;
    assert_eq!(relaunched["type"], "remark_update", "{relaunched}");
    assert!(relaunched.get("re").is_none());
    let payload = &relaunched["payload"];
    assert_eq!(payload["level"], "indice");
    assert_eq!(payload["levels_given"], json!(["indice"]));
    assert_eq!(payload["thread"][2]["author"], "mentor");
    assert_eq!(
        payload["thread"][2]["text"],
        "Relisez ce que compare chaque côté de ===."
    );
    assert_eq!(next(&mut socket).await["type"], "status");

    let request = fake.requests().last().unwrap().clone();
    let user = &request.messages[1].content;
    assert!(
        user.contains("<message auteur=\"personne\">Je compare les deux dates, non ?</message>"),
        "{user}"
    );
    assert!(user.contains("reposez la question autrement"), "{user}");
    assert!(user.ends_with("Palier demandé : indice."));

    // Une réponse vide est refusée sans rien enregistrer.
    let empty = request_with(
        &mut socket,
        "remark_reply",
        json!({ "remark_id": id, "text": "   " }),
        &mut Vec::new(),
    )
    .await;
    assert_eq!(empty["payload"]["code"], "bad_message");
}

#[tokio::test]
async fn sans_fournisseur_la_reponse_est_gardee_sans_relance() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let (remark, _) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();

    // Panne au palier suivant : erreur, remarque inchangée, aucune nouvelle tentative.
    fake.push(Err(LlmError::Timeout));
    let failed = hint_next(&mut socket, &id, false).await;
    assert_eq!(failed["type"], "error");
    assert_eq!(failed["payload"]["code"], "provider_error");
    assert_eq!(fake.requests().len(), 2);

    // Panne pendant la relance : la réponse reste, le second remark_update est inchangé.
    fake.push(Err(LlmError::Timeout));
    let answered = request(
        &mut socket,
        "remark_reply",
        json!({ "remark_id": id, "text": "Je ne vois pas." }),
    )
    .await;
    assert_eq!(answered["payload"]["thread"].as_array().unwrap().len(), 2);
    let second = next(&mut socket).await;
    assert_eq!(second["type"], "remark_update");
    assert_eq!(second["payload"], answered["payload"]);
    let status = next(&mut socket).await;
    assert_eq!(status["payload"]["provider_ok"], false);
    assert_eq!(fake.requests().len(), 3);
}

#[tokio::test]
async fn fermer_une_remarque_la_retire_du_fichier() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let (remark, _) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();

    let closed = request(
        &mut socket,
        "remark_dismiss",
        json!({ "remark_id": id, "reason": "resolved" }),
    )
    .await;
    assert_eq!(closed["type"], "remark_update");
    assert_eq!(closed["payload"]["status"], "resolved");
    let file = open(&mut socket).await;
    assert_eq!(file["payload"]["remarks"], json!([]));

    let calls = fake.requests().len();
    let refused = hint_next(&mut socket, &id, false).await;
    assert_eq!(refused["payload"]["code"], "bad_message");
    assert_eq!(fake.requests().len(), calls);

    let unknown = request(
        &mut socket,
        "remark_dismiss",
        json!({ "remark_id": "rm_inconnue", "reason": "ignored" }),
    )
    .await;
    assert_eq!(unknown["payload"]["code"], "not_found");
}

#[tokio::test]
async fn une_citation_disparue_detache_la_remarque_qui_reste_lisible() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let (remark, version) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();

    // Une ligne ajoutée au-dessus déplace l'ancre sans la détacher.
    let moved = RESERVER
        .replace("return !dejaPris;", "return dejaPris === undefined;")
        .replace("export function", "// Réservation.\nexport function");
    let messages = save(&mut socket, &moved, &version).await;
    assert!(
        messages.iter().all(|m| m["type"] != "remark_update"),
        "{messages:?}"
    );
    let version = messages.last().unwrap()["version"].clone();
    let file = open(&mut socket).await;
    assert_eq!(file["payload"]["remarks"][0]["anchor"]["line_hint"], 5);

    // La citation disparaît : la remarque passe en detached.
    let rewritten = moved.replace("e.debut === debut", "memeJour(e.debut, debut)");
    let messages = save(&mut socket, &rewritten, &version).await;
    let update = messages
        .iter()
        .find(|m| m["type"] == "remark_update")
        .expect("aucun remark_update");
    assert_eq!(update["payload"]["id"], id);
    assert_eq!(update["payload"]["status"], "detached");
    let version = messages.last().unwrap()["version"].clone();

    let file = open(&mut socket).await;
    let remarks = file["payload"]["remarks"].as_array().unwrap();
    assert_eq!(remarks.len(), 1);
    assert_eq!(remarks[0]["status"], "detached");
    assert_eq!(
        remarks[0]["thread"][0]["text"],
        "Regardez comment cette ligne compare deux dates."
    );

    // Détachement définitif : la citation qui revient ne la raccroche pas.
    save(&mut socket, &moved, &version).await;
    let file = open(&mut socket).await;
    assert_eq!(file["payload"]["remarks"][0]["status"], "detached");

    let calls = fake.requests().len();
    let refused = hint_next(&mut socket, &id, false).await;
    assert_eq!(refused["payload"]["code"], "bad_message");
    let reply = request(
        &mut socket,
        "remark_reply",
        json!({ "remark_id": id, "text": "Et maintenant ?" }),
    )
    .await;
    assert_eq!(reply["payload"]["code"], "bad_message");
    assert_eq!(fake.requests().len(), calls);

    // Elle peut encore être fermée.
    let closed = request(
        &mut socket,
        "remark_dismiss",
        json!({ "remark_id": id, "reason": "ignored" }),
    )
    .await;
    assert_eq!(closed["payload"]["status"], "ignored");
}

#[tokio::test]
async fn au_dela_du_budget_une_demande_explicite_attend_confirmation() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut llm: Llm = fake_llm(fake.clone());
    // La passe coûte 0,002 $ : le plafond est atteint juste après.
    llm.config.budget = BudgetConfig {
        monthly_cap_usd: 0.001,
        warn_at: 0.8,
    };
    let mut socket = open_socket_with(dir.path(), llm).await;
    let (remark, _) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();
    let calls = fake.requests().len();

    let refused = hint_next(&mut socket, &id, false).await;
    assert_eq!(refused["payload"]["code"], "budget_exceeded");
    let refused = request(
        &mut socket,
        "remark_reply",
        json!({ "remark_id": id, "text": "Une idée ?" }),
    )
    .await;
    assert_eq!(refused["payload"]["code"], "budget_exceeded");
    assert_eq!(fake.requests().len(), calls);
    let file = open(&mut socket).await;
    assert_eq!(
        file["payload"]["remarks"][0]["thread"]
            .as_array()
            .unwrap()
            .len(),
        1,
        "rien n'est enregistré sans confirmation"
    );

    fake.push(reply("question", "Que renvoie cette comparaison ?"));
    let confirmed = hint_next(&mut socket, &id, true).await;
    assert_eq!(confirmed["payload"]["level"], "question");
    assert_eq!(fake.requests().len(), calls + 1);
}

#[tokio::test]
async fn le_mentor_choisi_depuis_le_panneau_tient_le_fil() {
    let dir = sample();
    let fake = Arc::new(FakeLlmClient::new([first_remark()]));
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    open(&mut socket).await;
    // Le casting renvoyé porte sur la section du curseur : on le place dans la fonction.
    send_raw(
        &mut socket,
        &json!({ "v": 1, "type": "cursor", "payload": { "path": PATH, "line": 4, "col": 1 } })
            .to_string(),
    )
    .await;
    assert_eq!(next(&mut socket).await["type"], "casting");
    let casting = request(
        &mut socket,
        "cast_override",
        json!({ "path": PATH, "section_id": "fn:reserver", "mentor_id": "architecte" }),
    )
    .await;
    assert_eq!(casting["payload"]["titular"]["id"], "architecte");
    let (remark, _) = remark_after_save(&mut socket).await;
    assert_eq!(remark["mentor_id"], "architecte");

    fake.push(reply(
        "question",
        "Qu'attendez-vous de cette fonction si l'objet est déjà pris ?",
    ));
    let update = hint_next(&mut socket, &remark["id"], false).await;
    assert_eq!(update["payload"]["level"], "question");
    let request = fake.requests().last().unwrap().clone();
    assert_eq!(request.model, "test/fort");
    assert!(
        request.messages[0]
            .content
            .starts_with("Vous êtes Architecte")
    );
}

#[tokio::test]
async fn une_seule_reprise_a_la_fois_par_remarque() {
    let dir = sample();
    let fake = Arc::new(
        FakeLlmClient::new([
            first_remark(),
            reply("question", "Que renvoie cette comparaison ?"),
        ])
        .with_delay(Duration::from_millis(200)),
    );
    let mut socket = open_socket_with(dir.path(), fake_llm(fake.clone())).await;
    let (remark, _) = remark_after_save(&mut socket).await;
    let id = remark["id"].clone();

    for n in 1..=2 {
        send_raw(
            &mut socket,
            &json!({ "v": 1, "type": "hint_next", "id": format!("h-{n}"), "payload": { "remark_id": id } })
                .to_string(),
        )
        .await;
    }
    let mut replies = Vec::new();
    while let Some(message) = next_within(&mut socket, Duration::from_secs(2)).await {
        if message["re"].is_string() {
            replies.push(message);
        }
    }
    let busy = replies.iter().find(|m| m["re"] == "h-2").unwrap();
    assert_eq!(busy["payload"]["code"], "bad_message");
    let done = replies.iter().find(|m| m["re"] == "h-1").unwrap();
    assert_eq!(done["payload"]["level"], "question");
    assert_eq!(fake.requests().len(), 2);
}
