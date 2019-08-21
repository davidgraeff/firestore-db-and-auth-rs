use firestore_db_and_auth::{credentials, errors, sessions::service_account::Session};
use std::path::Path;

extern crate bincode;

fn main() -> errors::Result<()> {
    let target_path = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "service_account_session.bin".to_owned());
    let target_path = Path::new(&target_path);
    println!(
        "write binary file to: {}",
        target_path.canonicalize()?.to_str().unwrap()
    );
    let cred = credentials::Credentials::from_file(
        &std::env::args()
            .nth(1)
            .unwrap_or_else(|| "firebase-service-account.json".to_owned()),
    )?;

    {
        use std::fs::File;
        use std::io::Write;
        let mut target = File::create(&target_path)?;
        assert!(cred.private_key_der.len() > 0);

        let d = bincode::serialize(&cred)?;
        println!("Binary file size: {}", d.len());
        target.write_all(&d).unwrap();
    }

    // Sanity checks, you would use `let session : sessions::service_account::Session = from_binary!("../../service_account_session.bin");`
    let session: Session = Session::from_binary(target_path.to_str().unwrap())?;

    assert_eq!(cred.api_key, session.credentials.api_key);
    assert_eq!(cred.client_email, session.credentials.client_email);
    Ok(())
}
