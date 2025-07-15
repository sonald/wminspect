#!/bin/bash

# Script to run coverage analysis locally
# Requires nightly rust and grcov

set -e

echo "Installing grcov if not present..."
cargo install grcov

echo "Setting up environment for coverage..."
export CARGO_INCREMENTAL=0
export RUSTFLAGS="-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Cpanic=abort -Zpanic_abort_tests"
export RUSTDOCFLAGS="-Zprofile -Ccodegen-units=1 -Cinline-threshold=0 -Clink-dead-code -Coverflow-checks=off -Cpanic=abort"

echo "Cleaning previous coverage data..."
find . -name "*.gcda" -delete
find . -name "*.gcno" -delete
rm -rf target/coverage

echo "Running tests with coverage..."
cargo +nightly test --verbose --no-default-features
cargo +nightly test --verbose --features x11 || echo "X11 tests failed, continuing without X11"

echo "Generating coverage report..."
mkdir -p target/coverage
grcov . --binary-path ./target/debug/deps/ -s . -t html --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/
grcov . --binary-path ./target/debug/deps/ -s . -t lcov --branch --ignore-not-existing --ignore '../*' --ignore "/*" -o target/coverage/tests.lcov

echo "Coverage report generated in target/coverage/"
echo "Open target/coverage/index.html in your browser to view the HTML report"

# Calculate coverage percentage
if command -v lcov &> /dev/null; then
    lcov --summary target/coverage/tests.lcov
else
    echo "Install lcov to get coverage percentage summary"
fi
