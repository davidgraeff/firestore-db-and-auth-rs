use firestore_db_and_auth::*;

const TEST_USER_ID: &str = include_str!("test_user_id.txt");

#[test]
fn user_info() -> errors::Result<()> {
    let cred = credentials::Credentials::from_file("firebase-service-account.json").expect("Read credentials file");

    let user_session = sessions::user::Session::by_user_id(&cred, TEST_USER_ID, false)?;

    println!("users::user_info");
    let user_info_container = users::user_info(&user_session)?;
    assert_eq!(user_info_container.users[0].localId.as_ref().unwrap(), TEST_USER_ID);

    Ok(())
}
