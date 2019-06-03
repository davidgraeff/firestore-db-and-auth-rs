use std::env;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use firestore_db_and_auth::{credentials, sessions, from_binary};

fn main() {
    let dest_path = Path::new("service_account_session.bin");
    let mut f = File::create(&dest_path).unwrap();

    f.write_all(b"
        pub fn message() -> &'static str {
            \"Hello, World!\"
        }
    ").unwrap();
 
    // Perform test 
    let session : sessions::service_account::Session = from_binary!("../../service_account_session.bin");
} 