### Firestore Auth: Background information

**JWT**: Firestore Auth makes use of the *OAuth Grant Code Flow* and uses *JWT*s (Json web Tokens)
as access tokens. Such a token is signed by Google and consists of a few encoded fields including
a valid-until field. This allows to verify access tokens locally without any database access.

The Firebase API requires an access token, it accepts two types:

1. A custom created JWT, signed with the private key of a Google service account
2. An access token from Firestore Auth, bound to a user (in this crate called "user session")

If you do not have an user session access token, but you need to perform an action
impersonated, this crate offers `Session::by_user_id`. This will again create a custom, signed JWT,
like with option 1, but exchanges this JWT for a refresh token and access token tuple.
The actual database operation will be performed with those tokens.

About token validation:

Validation happens via the public keys of the corresponding Google service account (https://www.googleapis.com/service_accounts/v1/jwk/service.account@address).
The public keys are downloaded and cached the very first time you create a `credentials::Credentials` object.

To avoid this roundtrip on start it is **strongly** recommended to use the `Credentials::new(..)` function
to create the credentials object. Find more information further down.
