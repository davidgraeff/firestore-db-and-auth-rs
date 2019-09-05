### Http Rocket Server integration

Because the `sessions` module of this crate is already able to verify access tokens,
it was not much more work to turn this into a Rocket 0.4+ Guard.

The implemented Guard (enabled by the feature "rocket_support") allows access to http paths
if the provided http "Authorization" header contains a valid "Bearer" token.
The above mentioned validations on the token are performed.

See the rust documentation for `firestore_db_and_auth::rocket::FirestoreAuthSessionGuard`.