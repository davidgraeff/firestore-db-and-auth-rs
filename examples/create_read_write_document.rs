use firestore_db_and_auth::{documents, dto, errors, sessions, Credentials, FirebaseAuthBearer, ServiceSession};

use firestore_db_and_auth::documents::WriteResult;
use serde::{Deserialize, Serialize};

use futures::stream::StreamExt;

mod utils;

#[derive(Debug, Serialize, Deserialize)]
struct DemoDTO {
    a_string: String,
    an_int: u32,
    a_timestamp: String,
}

/// Test if merge works. a_timestamp is not defined here,
/// as well as an Option is used.
#[derive(Debug, Serialize, Deserialize)]
struct DemoDTOPartial {
    #[serde(skip_serializing_if = "Option::is_none")]
    a_string: Option<String>,
    an_int: u32,
}

async fn write_document(session: &mut ServiceSession, doc_id: &str) -> errors::Result<WriteResult> {
    println!("Write document");

    let obj = DemoDTO {
        a_string: "abcd".to_owned(),
        an_int: 14,
        a_timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
    };

    documents::write(session, "tests", Some(doc_id), &obj, documents::WriteOptions::default()).await
}

async fn write_partial_document(session: &mut ServiceSession, doc_id: &str) -> errors::Result<WriteResult> {
    println!("Partial write document");

    let obj = DemoDTOPartial {
        a_string: None,
        an_int: 16,
    };

    documents::write(
        session,
        "tests",
        Some(doc_id),
        &obj,
        documents::WriteOptions { merge: true },
    )
    .await
}

fn check_write(result: WriteResult, doc_id: &str) {
    assert_eq!(result.document_id, doc_id);
    let duration = chrono::Utc::now().signed_duration_since(result.update_time.unwrap());
    assert!(
        duration.num_seconds() < 60,
        "now = {}, updated: {}, created: {}",
        chrono::Utc::now(),
        result.update_time.unwrap(),
        result.create_time.unwrap()
    );
}

async fn service_account_session(cred: Credentials) -> errors::Result<()> {
    let mut session = ServiceSession::new(cred).await.unwrap();
    let b = session.access_token().await.to_owned();

    let doc_id = "service_test";
    check_write(write_document(&mut session, doc_id).await?, doc_id);

    // Check if cached value is used
    assert_eq!(session.access_token().await, b);

    println!("Read and compare document");
    let read: DemoDTO = documents::read(&mut session, "tests", doc_id).await?;

    assert_eq!(read.a_string, "abcd");
    assert_eq!(read.an_int, 14);

    check_write(write_partial_document(&mut session, doc_id).await?, doc_id);
    println!("Read and compare document");
    let read: DemoDTOPartial = documents::read(&mut session, "tests", doc_id).await?;

    // Should be updated
    assert_eq!(read.an_int, 16);
    // Should still exist, because of the merge
    assert_eq!(read.a_string, Some("abcd".to_owned()));

    Ok(())
}

async fn user_account_session(cred: Credentials) -> errors::Result<()> {
    let user_session = utils::user_session_with_cached_refresh_token(&cred).await?;

    assert_eq!(user_session.user_id, utils::TEST_USER_ID);
    assert_eq!(user_session.project_id(), cred.project_id);

    println!("user::Session::by_access_token");
    let user_session =
        sessions::user::Session::by_access_token(&cred, &user_session.access_token_unchecked().await).await?;

    assert_eq!(user_session.user_id, utils::TEST_USER_ID);

    let obj = DemoDTO {
        a_string: "abc".to_owned(),
        an_int: 12,
        a_timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
    };

    // Test writing
    println!("user::Session documents::write");
    let doc_id = "user_doc";
    check_write(
        documents::write(
            &user_session,
            "tests",
            Some(doc_id),
            &obj,
            documents::WriteOptions::default(),
        )
        .await?,
        doc_id,
    );

    // Test reading
    println!("user::Session documents::read");
    let read: DemoDTO = documents::read(&user_session, "tests", doc_id).await?;

    assert_eq!(read.a_string, "abc");
    assert_eq!(read.an_int, 12);

    // Query for all documents with field "a_string" and value "abc"
    let results: Vec<dto::Document> = documents::query(
        &user_session,
        "tests",
        "abc".into(),
        dto::FieldOperator::EQUAL,
        "a_string",
    )
    .await?
    .collect();
    assert_eq!(results.len(), 1);
    let doc: DemoDTO = documents::read_by_name(&user_session, &results.get(0).unwrap().name).await?;
    assert_eq!(doc.a_string, "abc");

    let mut count = 0;
    let list_it = documents::list(&user_session, "tests".to_owned())
        .collect::<Vec<errors::Result<(DemoDTO, _)>>>()
        .await;
    for _doc in list_it {
        count += 1;
    }
    assert_eq!(count, 2);

    // test if the call fails for a non existing document
    println!("user::Session documents::delete");
    let r = documents::delete(&user_session, "tests/non_existing", true).await;
    assert!(r.is_err());
    match r.err().unwrap() {
        errors::FirebaseError::APIError(code, message, context) => {
            assert_eq!(code, 404);
            assert!(message.contains("No document to update"));
            assert_eq!(context, "tests/non_existing");
        }
        _ => panic!("Expected an APIError"),
    };

    documents::delete(&user_session, &("tests/".to_owned() + doc_id), false).await?;

    // Check if document is indeed removed
    println!("user::Session documents::query");
    let count = documents::query(
        &user_session,
        "tests",
        "abc".into(),
        dto::FieldOperator::EQUAL,
        "a_string",
    )
    .await?
    .count();
    assert_eq!(count, 0);

    println!("user::Session documents::query for f64");
    let f: f64 = 13.37;

    let count = documents::query(&user_session, "tests", f.into(), dto::FieldOperator::EQUAL, "a_float").await?;

    let count = count.count();
    assert_eq!(count, 0);

    Ok(())
}

#[tokio::main]
async fn main() -> errors::Result<()> {
    // Search for a credentials file in the root directory
    use std::path::PathBuf;
    let mut credential_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    credential_file.push("firebase-service-account.json");
    let cred = Credentials::from_file(credential_file.to_str().unwrap()).await?;

    // Only download the public keys once, and cache them.
    let jwkset = utils::from_cache_file(credential_file.with_file_name("cached_jwks.jwks").as_path(), &cred).await?;
    cred.add_jwks_public_keys(&jwkset).await;
    cred.verify().await?;

    // Perform some db operations via a service account session
    service_account_session(cred.clone()).await?;

    // Perform some db operations via a firebase user session
    user_account_session(cred).await?;

    Ok(())
}

/// For integration tests and doc code snippets: Create a Credentials instance.
/// Necessary public jwk sets are downloaded or re-used if already present.
#[cfg(test)]
async fn valid_test_credentials() -> errors::Result<Credentials> {
    use std::path::PathBuf;
    let mut jwks_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    jwks_path.push("firebase-service-account.jwks");

    let cred: Credentials = Credentials::new(include_str!("../firebase-service-account.json")).await?;

    // Only download the public keys once, and cache them.
    let jwkset = utils::from_cache_file(jwks_path.as_path(), &cred).await?;
    cred.add_jwks_public_keys(&jwkset).await;
    cred.verify().await?;

    Ok(cred)
}

#[tokio::test]
async fn valid_test_credentials_test() -> errors::Result<()> {
    valid_test_credentials().await?;
    Ok(())
}

#[tokio::test]
async fn service_account_session_test() -> errors::Result<()> {
    service_account_session(valid_test_credentials().await?).await?;
    Ok(())
}

#[tokio::test]
async fn user_account_session_test() -> errors::Result<()> {
    user_account_session(valid_test_credentials().await?).await?;
    Ok(())
}
