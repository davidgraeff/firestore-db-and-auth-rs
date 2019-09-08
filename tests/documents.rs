use serde::{Deserialize, Serialize};

use firestore_db_and_auth::errors::FirebaseError;
use firestore_db_and_auth::*;

const TEST_USER_ID: &str = include_str!("test_user_id.txt");

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

#[test]
fn service_account_session() -> errors::Result<()> {
    let cred = credentials::Credentials::from_file("firebase-service-account.json").expect("Read credentials file");
    cred.verify()?;

    let mut session = sessions::service_account::Session::new(cred).unwrap();
    let b = session.access_token().to_owned();

    // Check if cached value is used
    assert_eq!(session.access_token(), b);

    println!("Write document");

    let obj = DemoDTO {
        a_string: "abcd".to_owned(),
        an_int: 14,
        a_timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
    };

    documents::write(
        &mut session,
        "tests",
        Some("service_test"),
        &obj,
        documents::WriteOptions::default(),
    )?;

    println!("Read and compare document");
    let read: DemoDTO = documents::read(&mut session, "tests", "service_test")?;

    assert_eq!(read.a_string, "abcd");
    assert_eq!(read.an_int, 14);

    println!("Partial write document");

    let obj = DemoDTOPartial {
        a_string: None,
        an_int: 16,
    };

    documents::write(
        &mut session,
        "tests",
        Some("service_test"),
        &obj,
        documents::WriteOptions { merge: true },
    )?;

    println!("Read and compare document");
    let read: DemoDTOPartial = documents::read(&mut session, "tests", "service_test")?;

    // Should be updated
    assert_eq!(read.an_int, 16);
    // Should still exist, because of the merge
    assert_eq!(read.a_string, Some("abcd".to_owned()));

    Ok(())
}

#[test]
fn user_account_session() -> errors::Result<()> {
    let cred = credentials::Credentials::from_file("firebase-service-account.json").expect("Read credentials file");

    println!("Refresh token from file");
    // Read refresh token from file if possible instead of generating a new refresh token each time
    let refresh_token: String = match std::fs::read_to_string("refresh-token-for-tests.txt") {
        Ok(v) => v,
        Err(e) => {
            if e.kind() != std::io::ErrorKind::NotFound {
                return Err(errors::FirebaseError::IO(e));
            }
            String::new()
        }
    };

    // Generate a new refresh token if necessary
    println!("Generate new user auth token");
    let user_session: sessions::user::Session = if refresh_token.is_empty() {
        let session = sessions::user::Session::by_user_id(&cred, TEST_USER_ID, true)?;
        std::fs::write("refresh-token-for-tests.txt", &session.refresh_token.as_ref().unwrap())?;
        session
    } else {
        println!("user::Session::by_refresh_token");
        sessions::user::Session::by_refresh_token(&cred, &refresh_token)?
    };

    assert_eq!(user_session.user_id, TEST_USER_ID);
    assert_eq!(user_session.project_id(), cred.project_id);

    println!("user::Session::by_access_token");
    let user_session = sessions::user::Session::by_access_token(&cred, &user_session.access_token_unchecked())?;

    assert_eq!(user_session.user_id, TEST_USER_ID);

    let obj = DemoDTO {
        a_string: "abc".to_owned(),
        an_int: 12,
        a_timestamp: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Nanos, true),
    };

    // Test writing
    println!("user::Session documents::write");
    let result = documents::write(
        &user_session,
        "tests",
        Some("test"),
        &obj,
        documents::WriteOptions::default(),
    )?;
    assert_eq!(result.document_id, "test");
    let duration = chrono::Utc::now().signed_duration_since(result.update_time.unwrap());
    assert!(
        duration.num_seconds() < 60,
        "now = {}, updated: {}, created: {}",
        chrono::Utc::now(),
        result.update_time.unwrap(),
        result.create_time.unwrap()
    );

    // Test reading
    println!("user::Session documents::read");
    let read: DemoDTO = documents::read(&user_session, "tests", "test")?;

    assert_eq!(read.a_string, "abc");
    assert_eq!(read.an_int, 12);

    // Query for all documents with field "a_string" and value "abc"
    let results: Vec<dto::Document> = documents::query(
        &user_session,
        "tests",
        "abc".into(),
        dto::FieldOperator::EQUAL,
        "a_string",
    )?
    .collect();
    assert_eq!(results.len(), 1);
    let doc: DemoDTO = documents::read_by_name(&user_session, results.get(0).unwrap().name.as_ref().unwrap())?;
    assert_eq!(doc.a_string, "abc");

    let mut count = 0;
    let list_it: documents::List<DemoDTO, _> = documents::list(&user_session, "tests".to_owned());
    for _doc in list_it {
        count += 1;
    }
    assert_eq!(count, 2);

    // test if the call fails for a non existing document
    println!("user::Session documents::delete");
    let r = documents::delete(&user_session, "tests/non_existing", true);
    assert!(r.is_err());
    match r.err().unwrap() {
        FirebaseError::APIError(code, message, context) => {
            assert_eq!(code, 404);
            assert!(message.contains("No document to update"));
            assert_eq!(context, "tests/non_existing");
        }
        _ => panic!("Expected an APIError"),
    };

    documents::delete(&user_session, "tests/test", false)?;

    // Check if document is indeed removed
    println!("user::Session documents::query");
    let count = documents::query(
        &user_session,
        "tests",
        "abc".into(),
        dto::FieldOperator::EQUAL,
        "a_string",
    )?
    .count();
    assert_eq!(count, 0);

    println!("user::Session documents::query for f64");
    let f: f64 = 13.37;
    let count = documents::query(&user_session, "tests", f.into(), dto::FieldOperator::EQUAL, "a_float")?.count();
    assert_eq!(count, 0);

    Ok(())
}
