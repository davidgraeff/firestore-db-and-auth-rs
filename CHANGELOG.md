# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.3.1] - 2019-09-05

### Changed
- The documents::list iterator now iterates over tuples (document, metadata). Metadata
  contains the document name, created and updated fields.

## [0.3.0] - 2019-09-04

### Added
- Improved error handling!
  New error type for FirebaseError: APIError.
  Contains the numeric Google error code, the error string code and an optional context.
  The context is set to the document path if APIError has been returned by any of the
  document APIs.

### Changed
- Renamed userinfo to user_info, userremove to user_remove
- The session object does not need to be mutable anymore
- The credentials API changed. Credentials::new introduced as preferred way to
  construct a Credentials object.

### Fixed
- user_info and user_remove now work as expected. Tests added.

## [0.2] - 2019-09-02

### Added
- Add jwks public keys via Credentials::add_jwks_public_keys(JWKSetDTO).
  This avoids downloading them on each start.
- Add new delete(auth, path, **fail_if_not_existing**) boolean argument
  including a test

### Changed
- FirebaseAuthBearer trait, bearer method now returns a String
  and works on a non mutable reference, thanks to RefCell.
- Credentials module: JWKSetDTO and JWSEntry are now public
- Renamed the rocket guard struct from ApiKey to FirestoreAuthSessionGuard