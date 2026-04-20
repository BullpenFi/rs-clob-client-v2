# Changelog

All notable changes to this repository will be documented in this file.

The format is based on Keep a Changelog and the release flow is intended to stay compatible with release-plz, matching the official Polymarket Rust SDK workflow as closely as practical for this unofficial Bullpen.fi repository.

## [Unreleased]

### Changed
- Renamed the package to `polymarket-client-sdk` to match the official Rust SDK dependency/import surface for testing and future migration.
- Marked the crate `publish = false` because this repository is an unofficial Bullpen.fi implementation and should not claim the official crates.io package slot.
- Added GitHub Actions workflows for CI, PR title validation, and release-plz style release automation.
- Updated the README to state explicitly that this repository is Bullpen.fi's best-guess Rust CLOB V2 SDK and is intended for testing until an official Polymarket Rust V2 SDK exists.
