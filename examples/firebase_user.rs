use firestore_db_and_auth::Credentials;
use firestore_db_and_auth::*;

const TEST_USER_ID: &str = include_str!("test_user_id.txt");

fn main() -> errors::Result<()> {
    let cred = Credentials::from_file("firebase-service-account.json").expect("Read credentials file");

    let user_session = BlockingUserSession::by_user_id(&cred, TEST_USER_ID, false)?;

    println!("users::user_info");
    let user_info_container = users::user_info(&user_session)?;
    assert_eq!(user_info_container.users[0].localId.as_ref().unwrap(), TEST_USER_ID);

    Ok(())
}

#[test]
fn firebase_user_test() {
    main().unwrap();
}
