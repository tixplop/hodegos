#![allow(clippy::unwrap_used, clippy::expect_used)]

//! Garde « pas de solution » au palier exemple, sur des réponses enregistrées
//! (`tests/fixtures/exemple/`) rejouées par `FakeLlmClient` : aucun appel réseau.

use std::fs;
use std::path::Path;

use hodegos_llm::{
    ChatMessage, FakeLlmClient, LlmRequest, LlmResponse, RemarkLevel, Role, Rules, ask_for_reply,
    reply_schema,
};
use serde::Deserialize;

#[derive(Deserialize)]
struct Fixture {
    language: String,
    section: String,
    /// La réponse doit-elle passer la garde ?
    kept: bool,
    /// Contenu brut renvoyé par le modèle.
    response: String,
}

#[tokio::test]
async fn reponses_enregistrees_au_palier_exemple() {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/exemple");
    let mut seen = 0;
    for entry in fs::read_dir(&dir).unwrap() {
        let path = entry.unwrap().path();
        let fixture: Fixture = serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        let fake = FakeLlmClient::new([Ok(LlmResponse {
            content: fixture.response,
            cost_usd: 0.001,
        })]);
        let request = LlmRequest {
            model: "test/fort".to_owned(),
            messages: vec![ChatMessage::new(Role::User, fixture.language)],
            schema_name: "reponse".to_owned(),
            schema: reply_schema(),
        };
        let rules = Rules {
            section_text: &fixture.section,
            level: RemarkLevel::Exemple,
            max_remarks: 1,
        };
        let replied = ask_for_reply(&fake, request, &rules).await.unwrap();
        assert_eq!(
            replied.reply.is_some(),
            fixture.kept,
            "{} : {:?}",
            path.display(),
            replied.rejected
        );
        if !fixture.kept {
            assert!(
                replied.rejected.iter().all(|r| r.starts_with("garde")),
                "{} : {:?}",
                path.display(),
                replied.rejected
            );
        }
        seen += 1;
    }
    assert!(
        seen >= 4,
        "réponses enregistrées introuvables dans {}",
        dir.display()
    );
}
