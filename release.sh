#!/bin/bash

set -e

VERSION=${1:-"0.4.0"}
TAG="v${VERSION}"

echo "Preparing release ${TAG}..."

# Check if we're on main branch
CURRENT_BRANCH=$(git branch --show-current)
if [ "$CURRENT_BRANCH" != "main" ]; then
    echo "Warning: Not on main branch (current: $CURRENT_BRANCH)"
    read -p "Continue anyway? (y/N) " -n 1 -r
    echo
    if [[ ! $REPLY =~ ^[Yy]$ ]]; then
        exit 1
    fi
fi

# Update version in Cargo.toml
sed -i "s/version = \".*\"/version = \"${VERSION}\"/" Cargo.toml

# Check if everything builds
echo "Building project..."
cargo build --release

# Run tests
echo "Running tests..."
cargo test

# Run clippy
echo "Running clippy..."
cargo clippy --all-targets --all-features -- -D warnings

# Check formatting
echo "Checking formatting..."
cargo fmt --all -- --check

# Commit version bump
git add Cargo.toml
git commit -m "Bump version to ${VERSION}"

# Create and push tag
echo "Creating tag ${TAG}..."
git tag -a "${TAG}" -m "Release ${TAG}"

echo "To complete the release:"
echo "1. Push the commits: git push origin main"
echo "2. Push the tag: git push origin ${TAG}"
echo "3. The GitHub Actions workflow will handle the rest"
echo ""
echo "Manual steps (if needed):"
echo "- cargo publish --token \$CRATES_TOKEN"
echo "- docker build -t wminspect:${TAG} ."
echo "- docker tag wminspect:${TAG} wminspect:latest"
