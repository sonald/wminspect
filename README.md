# wminspect

A powerful X11 window manager inspector and monitoring tool with a rich DSL for filtering and tracking window events.

## Project Goal

wminspect is designed to provide deep insights into X11 window manager behavior through real-time monitoring and flexible filtering capabilities. It allows developers, system administrators, and X11 enthusiasts to:

- **Monitor window events** in real-time (create, destroy, configure, map/unmap, property changes)
- **Filter windows** using a powerful Domain Specific Language (DSL)
- **Track window stacking order** and geometry changes
- **Serialize and load filtering rules** for reusable monitoring configurations
- **Highlight changes** across events for better visibility
- **Pin specific windows** for persistent monitoring

## Installation

### Prerequisites

- Rust 1.70 or later
- X11 development libraries:
  - `libxcb-dev` (Ubuntu/Debian)
  - `xcb-devel` (RHEL/CentOS/Fedora)
  - `libxcb` (Arch Linux)
- XCB utility libraries:
  - `libxcb-ewmh-dev`, `libxcb-icccm4-dev`, `libxcb-keysyms1-dev` (Ubuntu/Debian)

### Building from Source

```bash
git clone https://github.com/your-username/wminspect.git
cd wminspect
cargo build --release
```

### Installation

```bash
cargo install --path .
# or
cargo install --git https://github.com/your-username/wminspect.git
```

## Quick Start

### Basic Usage

```bash
# List all windows once
wminspect

# Monitor window events in real-time
wminspect --monitor
# or
wminspect monitor

# Monitor with colored output
wminspect -m -c

# Show only mapped windows
wminspect --only-mapped

# Omit hidden windows (outside screen boundaries)
wminspect --omit-hidden

# Show event sequence numbers
wminspect -m -n

# Highlight differences between events
wminspect -m -d
```

### Basic Filtering

```bash
# Filter by window name
wminspect -f "name=*firefox*"

# Filter by geometry
wminspect -f "geom.width>800"

# Filter by attributes
wminspect -f "attrs.map_state=Viewable"

# Complex filtering with logical operators
wminspect -f "any(name=*browser*, all(geom.x>100, geom.width>500))"
```

### Rule Management

```bash
# Show the complete DSL grammar
wminspect --show-grammar

# Load rules from a file
wminspect sheet --load rules.rule

# Compile rules to JSON format
wminspect sheet --compile rules.rule compiled.json

# Compile rules to binary format
wminspect sheet --compile rules.rule compiled.bin
```

## Advanced Examples

### Complex Window Filtering

```bash
# Monitor all Firefox windows above a certain size
wminspect -m -f "all(name=*firefox*, geom.width>=1200, geom.height>=800)"

# Pin specific windows and filter others
wminspect -m -f "name=*terminal*: pin; attrs.map_state=Viewable: filter"

# Monitor only client windows (managed by window manager)
wminspect -m -C

# Complex rule with multiple conditions
wminspect -m -f "
    any(
        all(name=*browser*, geom.x>0, geom.y>0),
        all(attrs.override_redirect=false, geom.width>800)
    ): filter;
    name=*important*: pin
"
```

### Rule Files

Create a `monitoring.rule` file:
```
# Monitor development environment
any(name=*vscode*, name=*terminal*, name=*browser*): filter;

# Pin system dialogs
attrs.override_redirect=true: pin;

# Filter out small windows
all(geom.width<200, geom.height<200): filter;

# Monitor specific applications
any(name=*slack*, name=*discord*, name=*signal*): filter;
```

Load and use:
```bash
wminspect sheet --load monitoring.rule
wminspect -m
```

### Serialization Formats

```bash
# Compile to JSON (human-readable)
wminspect sheet --compile monitoring.rule rules.json

# Compile to binary (faster loading)
wminspect sheet --compile monitoring.rule rules.bin

# Load compiled rules
wminspect sheet --load rules.json
wminspect sheet --load rules.bin
```

## Architecture Overview

wminspect consists of several key components:

- **Context**: Main application context managing X11 connections and window state
- **Filter**: DSL parser and rule engine for window filtering
- **Sheets**: Rule serialization and compilation system
- **WM Core**: X11 event handling and window management integration

For detailed architecture information, see [docs/architecture.md](docs/architecture.md).

## DSL Reference

The filtering DSL supports rich expressions for window matching. For complete grammar and examples, see [docs/dsl.md](docs/dsl.md) or run:

```bash
wminspect --show-grammar
```

## Contributing

We welcome contributions! Please see our [contribution guidelines](CONTRIBUTING.md) for details.

### Development Setup

```bash
git clone https://github.com/your-username/wminspect.git
cd wminspect
cargo build
cargo test
```

### Code Style

- Follow Rust standard formatting: `cargo fmt`
- Ensure code passes linting: `cargo clippy`
- Add tests for new functionality
- Update documentation for API changes

### Testing

```bash
# Run all tests
cargo test

# Run specific test module
cargo test filter

# Run tests with output
cargo test -- --nocapture
```

### Submitting Changes

1. Fork the repository
2. Create a feature branch: `git checkout -b feature/new-feature`
3. Make your changes with tests
4. Ensure all tests pass: `cargo test`
5. Format code: `cargo fmt`
6. Submit a pull request

### Issue Reporting

When reporting issues, please include:

- Operating system and version
- X11 window manager in use
- wminspect version
- Minimal reproduction steps
- Expected vs actual behavior

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Future Ideas

- Better event tracing with dynamic tracepoints
- SQL-like syntax for rules
- SystemTap-like dynamic tracing capabilities
- GUI interface for rule management
- Real-time rule injection/removal
- Support for Wayland compositors
- Event timestamp tracking
- Rule databases with version control

