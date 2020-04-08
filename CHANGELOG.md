# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](http://keepachangelog.com/en/1.0.0/)
and this project adheres to [Semantic Versioning](http://semver.org/spec/v2.0.0.html).


## [Unreleased]

[Unreleased]: https://github.com/althonos/nanoset.py/compare/v0.1.4...HEAD

## [v0.1.4] - 2020-04-08

[v0.1.4]: https://github.com/althonos/nanoset.py/compare/v0.1.3...v0.1.4

### Added
- Compilation of Python 3.5 and 3.8 wheels for Mac OSX
  ([#3](https://github.com/althonos/nanoset.py/issues/3)).



## [v0.1.3] - 2019-11-18

[v0.1.2]: https://github.com/althonos/nanoset.py/compare/v0.1.2...v0.1.3

### Added
- Compilation of Python 3.8 wheels for Linux and Windows
  ([#1](https://github.com/althonos/nanoset.py/issues/1)).


## [v0.1.2] - 2019-09-23

[v0.1.2]: https://github.com/althonos/nanoset.py/compare/v0.1.1...v0.1.2

### Added
- Special case to create a `NanoSet` from a `dict` without rehashing.
- Implementation of equality check from `NanoSet` to `frozenset`.


## [v0.1.1] - 2019-09-22

[v0.1.1]: https://github.com/althonos/nanoset.py/compare/v0.1.0...v0.1.1

### Fixed
- Compilation of Rust crate when not building a Python extension module.
- Project metadata for PyPI and `crates.io`.
- OSX deployment scripts not deploying built wheels successfully.

### Added
- `pyproject.toml` file to source distribution.


## [v0.1.0] - 2019-09-21

[v0.1.0]: https://github.com/althonos/nanoset.py/compare/36756b1...v0.1.0

Initial release.
