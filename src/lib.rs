#![deny(warnings)]
#![cfg_attr(feature = "external_doc", feature(external_doc))]
#![cfg_attr(feature = "external_doc", doc(include = "../readme.md"))]

pub mod credentials;
pub mod documents;
pub mod dto;
pub mod errors;
pub mod firebase_rest_to_rust;
pub mod jwt;
pub mod sessions;
pub mod users;

#[cfg(feature = "rocket_support")]
pub mod rocket;

// Forward declarations
pub use credentials::Credentials;
pub use jwt::JWKSet;
pub use sessions::service_account::Session as ServiceSession;
pub use sessions::user::Session as UserSession;

/// Authentication trait.
///
/// This trait is implemented by [`crate::sessions`].
///
/// Firestore document methods in [`crate::documents`] expect an object that implements this `FirebaseAuthBearer` trait.
///
/// Implement this trait for your own data structure and provide the Firestore project id and a valid access token.
/// Your implementation should offer means to periodically, or on-demand, check if a new access token
/// need to be requested via a refresh-token.
pub trait FirebaseAuthBearer {
    /// Return the project ID. This is required for the firebase REST API.
    fn project_id(&self) -> &str;
    /// An access token. Might be expired or in other ways invalid.
    /// An implementation should offer a way to re-new expired access tokens.
    fn access_token(&self) -> String;
    /// Returns a tokio runtime for the blocking API feature
    #[cfg(feature = "blocking")]
    fn rt(&self) -> &tokio::runtime::Runtime;
    /// The async reqwest http client.
    /// The `Client` holds a connection pool internally, so it is advised that it is reused for multiple, successive connections.
    fn client_async(&self) -> &reqwest::Client;
}
