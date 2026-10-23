# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [1.1.1](https://codeberg.org/wvhulle/lean-tui/compare/lean-tui-v1.1.0...lean-tui-v1.1.1) - 2026-10-23

### Build

- move repository tasks from Nu scripts to a cargo xtask

### CI

- stop installing test artifacts and upload release assets with tea
- push final build outputs that cachix watch-exec misses

### Changed

- move recordings into a self-contained module and keep Lean out of flake checks
- iced-style update, view and subscriptions

### Documentation

- slim the README and generate the keybinding reference

### Fixed

- make screen recordings more compact
- *(xtask)* refuse to publish screenshots when no recordings were produced
- *(nix)* include replay fixtures and snapshots in the crate source

### Other

- run lean-prism serve as the Lean language server instead of requiring and importing LeanPrism
- show what to do next based on the connection state, the open file and the editor reported by Lean-Prism

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
