# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.5.0] - 2025-01-24

### Changed
- Removed `futures` crate dependency for lighter weight and simpler code
- Simplified timeout handling - users now wrap calls with their async runtime's timeout mechanism (e.g., `embassy_time::with_timeout`)
- Improved error handling with idiomatic `.map_err()` usage
- Changed `Now` trait documentation comment from `//` to `///` for proper doc generation

### Added
- Comprehensive clippy lints configuration in `Cargo.toml` (all, pedantic, nursery, cargo)
- Clippy checks in CI workflow to enforce code quality
- Better documentation explaining timeout handling approach with examples

### Fixed
- Updated all inline comments to accurately match code behavior
- Made `new()` function `const fn` for compile-time initialization

## [0.4.0] - 2025-01-23

### Changed
- Made the crate executor independent, removing the direct dependency on embassy_time (by [@afresquet](https://github.com/afresquet))
- Updated documentation to reflect the new executor-independent implementation
- Simplified examples to show proper clock and delay usage

### Removed
- Removed `embassy` feature as it's no longer needed
