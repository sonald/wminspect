##  0.4.0 (2024-12-19)

#### Continuous Integration & Release

*   Added GitHub Actions build matrix for stable and beta Rust toolchains
*   Enhanced CI pipeline with improved clippy checks and test coverage
*   Added automated release binary generation using cargo zip
*   Implemented multi-platform Docker image for easy trial
*   Published to crates.io for easier distribution

#### Infrastructure Improvements

*   Enhanced documentation for installation and usage
*   Improved project structure and build configuration
*   Added comprehensive release automation

##  0.3.1 (2024-12-19)

#### Upgrading to Rust 2024 Edition

*   Updated to Rust 2024 edition
*   Upgraded clap from 2.27 to 4.x with new API
*   Updated bincode from 1.x to 2.x
*   Replaced timer crate with crossbeam-channel
*   Updated xcb-util to 0.4 with required features
*   Added serde derive feature for automatic serialization
*   Fixed various compilation issues and warnings

#### Breaking Changes

*   Command line argument handling updated to clap 4.x API
*   Serialization format may have changed due to bincode upgrade
*   Timer functionality replaced with crossbeam-channel

##  0.1.0 (2017-11-29)

#### Features

*   se/deser are complete ([1f566a55](1f566a55))
*   use a better way to represent rule ([a2aa24f7](a2aa24f7))
*   make serialization work ([24e23450](24e23450))
*   allow to dump filter grammar ([23fa6927](23fa6927))
*   add macros mod ([86db7ccc](86db7ccc))
*   use clap for command line parsing ([a09e1013](a09e1013))

#### Bug Fixes

*   clap uses Arg name to index option ([0d7263ba](0d7263ba))



