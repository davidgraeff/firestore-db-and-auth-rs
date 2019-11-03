# Rocket Protected Route Example

[Rocket](https://rocket.rs) is a an easy to use web-framework for Rust.
This example only compiles with Rust nightly (because Rocket requires nightly)
and expects a "firebase-service-account.json" file in the working directory.

It shows how to protect an http route by only allowing logged in users.

* Build and run with `cargo run`.
* Surf to http://127.0.0.1:8000/create_test_user. A firebase user "_test" will be created and an access token is printed.
* Surf to http://127.0.0.1:8000/hello?auth=A_FIREBASE_ACCESS_TOKEN and to http://127.0.0.1:8000/hello
* Surf to http://127.0.0.1:8000/remove_test_user to remove the created test user again.
