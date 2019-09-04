
### Use your own authentication implementation

You do not need the `sessions` module for using the Firestore API of this crate.
All Firestore methods in `documents` expect an object that implements the `FirebaseAuthBearer` trait.

That trait looks like this:

```rust
pub trait FirebaseAuthBearer<'a> {
    fn projectid(&'a self) -> &'a str;
    fn bearer(&'a self) -> &'a str;
}
```

Just implement this trait for your own data structure and provide the Firestore project id and a valid access token.
