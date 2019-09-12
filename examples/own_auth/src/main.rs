use firestore_db_and_auth::{Credentials, FirebaseAuthBearer, documents};

/// Define your own structure that will implement the FirebaseAuthBearer trait
struct MyOwnSession {
    /// The google credentials
    pub credentials: Credentials,
    pub client: reqwest::Client,
    access_token: String,
}

impl FirebaseAuthBearer for MyOwnSession {
    fn project_id(&self) -> &str {
        &self.credentials.project_id
    }
    /// An access token. If a refresh token is known and the access token expired,
    /// the implementation should try to refresh the access token before returning.
    fn access_token(&self) -> String {
        self.access_token.clone()
    }
    /// The access token, unchecked. Might be expired or in other ways invalid.
    fn access_token_unchecked(&self) -> String {
        self.access_token.clone()
    }
    /// The reqwest http client.
    /// The `Client` holds a connection pool internally, so it is advised that it is reused for multiple, successive connections.
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

fn main() {
    let credentials = Credentials::from_file("firebase-service-account.json").unwrap();
    #[derive(serde::Serialize)]
    struct TestData {
        an_int: u32
    };
    let t = TestData {
        an_int: 12
    };
    
    let session = MyOwnSession {
        credentials,
        client: reqwest::Client::new(),
        access_token: "The access token".to_owned()
    };

    // Use any of the document functions with your own session object
    documents::write(&session, "tests", Some("test_doc"), &t, documents::WriteOptions::default()).unwrap();
}
