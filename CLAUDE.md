# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

wminspect is a powerful X11 window manager inspector and monitoring tool with a rich DSL for filtering and tracking window events. It provides real-time monitoring, window filtering, rule serialization, and flexible highlighting capabilities.

## Development Commands

### Building and Testing
```bash
# Build the project (requires X11 development libraries)
cargo build

# Build release version
cargo build --release

# Run all tests
cargo test

# Run specific test module
cargo test filter
cargo test wildcard

# Run tests with output
cargo test -- --nocapture

# Run library tests only (faster, no X11 required)
cargo test --lib
```

### Code Quality
```bash
# Format code
cargo fmt

# Run clippy linter
cargo clippy

# Run benchmarks
cargo bench
```

### macOS Development Notes
- The project supports macOS with X11 through XQuartz or Homebrew XCB libraries
- See `.cargo/config.toml` for cross-architecture build configuration (Apple Silicon + Intel)
- XQuartz (/opt/X11/lib) and Homebrew (/opt/homebrew/lib) library paths are pre-configured

### Running the Application
```bash
# Basic window inspection
./target/debug/wminspect

# Monitor mode with colored output
./target/debug/wminspect -m -c

# Test filtering syntax
./target/debug/wminspect -f "name=*firefox*"

# Show DSL grammar
./target/debug/wminspect --show-grammar

# Rule management
./target/debug/wminspect sheet --compile rules.rule compiled.json
./target/debug/wminspect sheet --load rules.rule
```

## Architecture Overview

### Core Module Structure
```
src/
├── core/           # Core utilities and shared components
│   ├── error.rs    # Error types and results (WmError, WmResult, CoreResult)
│   ├── state.rs    # Global state management (GlobalState, StateRef)
│   ├── types.rs    # Shared type definitions
│   ├── wildcard.rs # Pattern matching with caching (OptimizedWildcardMatcher)
│   ├── stack_diff.rs # Window stacking change detection
│   └── colorized_output.rs # Terminal output formatting
├── dsl/            # Domain Specific Language implementation
│   ├── filter.rs   # DSL parser, tokenizer, and rule engine
│   └── sheets.rs   # Rule serialization (plain text, JSON, binary)
├── platform/       # Platform-specific X11 integration
│   └── x11.rs      # X11 event handling and window management
└── ui/             # Future GUI components (placeholder)
```

### Key Architectural Patterns

**Event Flow**: X11 events → Context dispatch → Filter application → Rule processing → Output formatting

**Threading Model**: 
- Main thread: CLI and initialization
- Event loop: Asynchronous X11 event processing
- Worker threads: Parallel rule evaluation with crossbeam channels

**State Management**: Centralized through `GlobalState` with thread-safe `StateRef` for shared access

### DSL Architecture
The filtering DSL is implemented as a multi-stage pipeline:
1. **Tokenization**: Input string → Token stream (`scan_tokens`)
2. **Parsing**: Tokens → Abstract Syntax Tree (`parse_rule`, `parse_cond`)
3. **Compilation**: AST → Executable filter predicates
4. **Serialization**: Rules ↔ Multiple formats (plain text, JSON, binary)

Key types:
- `Token`: Lexical tokens (operators, literals, keywords)
- `Predicate`: Window properties (name, geometry, attributes)
- `FilterItem`: Complete rule with condition and action
- `Op`: Comparison operators (=, <>, >, <, etc.)

### Pattern Matching System
`OptimizedWildcardMatcher` provides:
- Wildcard pattern caching with `GLOB_CACHE`
- Direct string comparison for non-wildcard patterns
- GlobSet integration for complex patterns
- Performance optimization for repeated pattern matching

## Feature System

The codebase uses Rust features for conditional compilation:
- `x11`: Core X11 functionality (default)
- `gui`: Future GUI components
- `platform-*`: Platform-specific code paths
- Default features: `["x11", "gui"]`

## Testing Architecture

### Test Organization
- Unit tests: Embedded in each module with `#[cfg(test)]`
- Integration tests: `tests/` directory with CLI and X11 interaction tests
- Benchmarks: `benches/filter_bench.rs` for performance testing

### Key Test Patterns
- DSL parsing: Comprehensive token scanning and rule parsing tests
- Pattern matching: Wildcard and glob pattern validation
- Serialization: Round-trip testing for all supported formats
- Error handling: Validation of parsing errors and recovery

## Error Handling Strategy

The project uses a layered error handling approach:
- `WmError`: Core window manager errors
- `CoreResult<T>`: Result type for core operations
- `WmResult<T>`: Window management specific results
- Error propagation through `?` operator with context preservation

## Configuration and Rules

### Rule File Formats
1. **Plain text** (`.rule`): Human-readable with comments
2. **JSON** (`.json`): Structured for programmatic access
3. **Binary** (`.bin`): Optimized for fast loading (~10x faster than JSON)

### DSL Grammar Key Points
- Logical operators: `any()`, `all()`, `not()`
- Predicates: `name`, `geom.width`, `attrs.map_state`, `clients`
- Actions: `filter` (include) or `pin` (highlight)
- Wildcards: `*` (multi-char), `?` (single char)

## Development Workflow Notes

### Common Development Tasks
- DSL modifications require updating both parser (`filter.rs`) and grammar documentation
- New predicates need enum updates and parsing logic in `parse_cond()`
- Pattern matching changes should update cached glob handling
- Cross-platform testing important due to X11 dependencies

### Performance Considerations
- Rule compilation is cached for repeated use
- Binary serialization preferred for production rule loading
- Wildcard patterns are pre-compiled and cached via `GLOB_CACHE`
- Event processing uses async patterns to avoid blocking main thread