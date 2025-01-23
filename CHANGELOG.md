# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.4.0] - 2025-01-23

### Changed
- Made the crate executor independent, removing the direct dependency on embassy_time (by [@afresquet](https://github.com/afresquet))
- Updated documentation to reflect the new executor-independent implementation
- Simplified examples to show proper clock and delay usage

### Removed
- Removed `embassy` feature as it's no longer needed
