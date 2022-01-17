use firestore_db_and_auth::{documents, errors, Credentials, FirebaseAuthBearer};

/// Define your own structure that will implement the FirebaseAuthBearer trait
struct MyOwnSession {
    /// The google credentials
    pub credentials: Credentials,
    pub client: reqwest::Client,
    access_token: String,
}

#[async_trait::async_trait]
impl FirebaseAuthBearer for MyOwnSession {
    fn project_id(&self) -> &str {
        &self.credentials.project_id
    }
    /// An access token. If a refresh token is known and the access token expired,
    /// the implementation should try to refresh the access token before returning.
    async fn access_token(&self) -> String {
        self.access_token.clone()
    }
    /// The access token, unchecked. Might be expired or in other ways invalid.
    async fn access_token_unchecked(&self) -> String {
        self.access_token.clone()
    }
    /// The reqwest http client.
    /// The `Client` holds a connection pool internally, so it is advised that it is reused for multiple, successive connections.
    fn client(&self) -> &reqwest::Client {
        &self.client
    }
}

#[tokio::main]
async fn main() -> errors::Result<()> {
    let credentials = Credentials::from_file("firebase-service-account.json").await?;
    #[derive(serde::Serialize)]
    struct TestData {
        an_int: u32,
    }
    let t = TestData { an_int: 12 };

    let session = MyOwnSession {
        credentials,
        client: reqwest::Client::new(),
        access_token: "The access token".to_owned(),
    };

    // Use any of the document functions with your own session object
    documents::write(
        &session,
        "tests",
        Some("test_doc"),
        &t,
        documents::WriteOptions::default(),
    ).await?;
    Ok(())
}

#[tokio::test]
async fn own_auth_test() {
    if let Err(APIError(code, str_code, context)) = main() {
        assert_eq!(str_code, "Request had invalid authentication credentials. Expected OAuth 2 access token, login cookie or other valid authentication credential. See https://developers.google.com/identity/sign-in/web/devconsole-project.");
        assert_eq!(context, "test_doc");
        assert_eq!(code, 401);
        return;
    }
    panic!("Expected a failure with invalid access token");
}
