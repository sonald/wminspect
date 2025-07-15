# Release Notes for wminspect v0.4.0

## Overview

This release focuses on improving the continuous integration pipeline, release automation, and distribution methods for wminspect. The tool now has better CI/CD workflows and is available through multiple distribution channels.

## 🚀 New Features

### Continuous Integration & Release
- **GitHub Actions build matrix**: Added support for both stable and beta Rust toolchains
- **Automated release binaries**: Implemented `cargo zip` for binary distribution
- **Docker image**: Created multi-stage Docker build for easy trial and deployment
- **crates.io publishing**: Prepared package for publication to the Rust package registry

### Infrastructure Improvements
- **Enhanced CI pipeline**: Improved clippy checks and test coverage reporting
- **Release automation**: Added comprehensive release workflow and scripts
- **Documentation**: Enhanced installation and usage documentation

## 📦 Distribution

### Docker
```bash
# Build locally
docker build -t wminspect .

# Run with X11 support
docker run --rm -it \
  -v /tmp/.X11-unix:/tmp/.X11-unix:rw \
  -e DISPLAY=$DISPLAY \
  wminspect --help
```

### crates.io
```bash
# Install from crates.io
cargo install wminspect
```

### Binary Releases
Pre-built binaries are available in the GitHub releases.

## 🔧 Technical Details

### Build Matrix
- **Stable Rust**: Primary supported toolchain
- **Beta Rust**: Forward compatibility testing

### CI/CD Pipeline
- Formatting checks with `cargo fmt`
- Linting with `cargo clippy`
- Comprehensive test suite
- Code coverage reporting
- Automated binary generation

## 📋 Requirements

- Rust 1.70+ (2024 edition)
- X11 development libraries:
  - `libxcb1-dev`
  - `libxcb-util0-dev`
  - `libxcb-ewmh-dev`
  - `libxcb-keysyms1-dev`
  - `libxcb-icccm4-dev`

## 🔄 Upgrading from v0.3.1

This release is primarily focused on infrastructure improvements. The core functionality remains the same, but users now have more installation options:

1. **From source**: `cargo install wminspect`
2. **Docker**: Use the provided Docker image
3. **Binary**: Download pre-built binaries from GitHub releases

## 🛠️ Development

### Release Process
A new `release.sh` script automates the release process:
```bash
./release.sh 0.4.0
```

### Docker Development
Use the provided Docker Compose configuration for development:
```bash
docker-compose up -d
```

## 📚 Documentation

- **Docker usage**: See `DOCKER.md` for detailed Docker instructions
- **API documentation**: Available at https://docs.rs/wminspect
- **Source code**: https://github.com/sianscao/wminspect

## 🙏 Contributors

Thanks to all contributors who helped improve the CI/CD pipeline and release process!

---

For detailed changelog, see [CHANGELOG.md](CHANGELOG.md).
