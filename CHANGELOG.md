# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.0](https://codeberg.org/wvhulle/lean-tui/compare/v1.0.1...v1.1.0) - 2026-10-16

### Added

- display tactic evaluation time in corner of state nodes when sent by backend
- add heartbeat time and metrics keybind
- nix-recorded demo GIFs

### CI

- publish releases with release-plz and attach musl binaries

### Changed

- record README GIFs with VHS tapes

### Documentation

- warn that LeanPrism must match the project's Lean version
- use absolute Codeberg links so the README renders on crates.io

### Other

- Fix link to CONTRIBUTING.md in README
- Fix link to lean prism
- Fix nix build
