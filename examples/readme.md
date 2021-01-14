# Examples

All examples expects a "firebase-service-account.json" file in the working directory.

## Create/Read/Write document example

A document is created / read / and writen to in full and partially in this example.
A service account session is used as well as a firebase impersonated user session.

* Build and run with `cargo run --example create_read_write_document`.

## Own authentication mechanism example

This example shows how to use your own implementation of FirebaseAuthBearer and avoid using any of the `session`
types.

* Build and run with `cargo run --example own_auth`.

## Firebase user interaction example

This example shows how to print all available information about a firebase user,
identified by the firebase user id.

* Build and run with `cargo run --example firebase_user`.

## Rocket Protected Route example

[Rocket](https://rocket.rs) is a an easy to use web-framework for Rust.

This example shows how to protect an http route by only allowing logged in users.

* Build and run with `cargo run --example rocket_http_protected_route`.
* Surf to http://127.0.0.1:8000/create_test_user. A firebase user "_test" will be created and an access token is printed.
* Surf to http://127.0.0.1:8000/hello?auth=A_FIREBASE_ACCESS_TOKEN and to http://127.0.0.1:8000/hello
* Surf to http://127.0.0.1:8000/remove_test_user to remove the created test user again.
