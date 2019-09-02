# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2] - 2019-09-02

### Added
- Add jwks public keys via Credentials::add_jwks_public_keys(JWKSetDTO).
  This avoids downloading them on each start.

### Changed
- FirebaseAuthBearer trait, bearer method now returns a String
  and works on a non mutable reference, thanks to RefCell.
- Credentials module: JWKSetDTO and JWSEntry are now public