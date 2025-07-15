# Baseline Assessment Report

## Environment Setup
- **Date**: July 15, 2025
- **Rust Version**: 1.88.0 (6b00bc388 2025-06-23)
- **Cargo Version**: 1.88.0 (873a06493 2025-05-10)
- **Nightly Version**: 1.90.0-nightly (a00149764 2025-07-14)
- **Project Edition**: 2015 (defaulting, latest available is 2024)

## Compiler Error Log (`cargo check`)

```
warning: `/Users/siancao/work/rust/wminspect/.cargo/config` is deprecated in favor of `config.toml`
note: if you need to support cargo 1.38 or earlier, you can symlink `config` to `config.toml`
warning: no edition set: defaulting to the 2015 edition while the latest is 2024
    Checking memchr v2.7.5
    Checking core-foundation-sys v0.8.7
    Checking regex-syntax v0.8.5
    Checking libc v0.2.174
    Checking log v0.4.27
    Checking unicode-width v0.1.14
    Checking crossbeam-utils v0.8.21
    Checking serde v1.0.219
    Checking num-traits v0.2.19
    Checking unty v0.0.4
    Checking strsim v0.6.0
    Checking ryu v1.0.20
    Checking textwrap v0.9.0
    Checking itoa v1.0.15
    Checking iana-time-zone v0.1.63
    Checking lazy_static v1.5.0
    Checking ansi_term v0.9.0
    Checking vec_map v0.8.2
    Checking bitflags v0.9.1
    Checking bincode v2.0.1
    Checking atty v0.2.14
   Compiling xcb v0.9.0
    Checking aho-corasick v1.1.3
    Checking is-terminal v0.4.16
    Checking crossbeam-epoch v0.9.18
    Checking crossbeam-channel v0.5.15
    Checking crossbeam-queue v0.3.12
    Checking clap v2.27.1
    Checking colored v1.9.4
    Checking crossbeam-deque v0.8.6
    Checking chrono v0.4.41
    Checking crossbeam v0.8.4
    Checking timer v0.2.0
    Checking regex-automata v0.4.9
    Checking erased-serde v0.3.31
    Checking serde_json v1.0.140
    Checking regex v1.11.1
    Checking xcb-util v0.3.0
    Checking wminspect v0.3.0 (/Users/siancao/work/rust/wminspect)
error[E0425]: cannot find function `deserialize_from` in crate `bc`
  --> src/wm/sheets.rs:34:17
   |
34 |             bc::deserialize_from(&mut data.as_bytes()).ok()
   |                 ^^^^^^^^^^^^^^^^ not found in `bc`

error[E0425]: cannot find function `serialize_into` in crate `bc`
   --> src/wm/sheets.rs:141:25
    |
141 |                     bc::serialize_into(&mut dest, &rule)
    |                         ^^^^^^^^^^^^^^ not found in `bc`

warning: unexpected `cfg` condition value: `core_intrinsics`
  --> src/wm/macros.rs:22:7
   |
22 | #[cfg(feature = "core_intrinsics")]
   |       ^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: remove the condition
   |
   = note: no expected values for `feature`
   = help: consider adding `core_intrinsics` as a feature in `Cargo.toml`
   = note: see <https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html> for more information about checking conditional configuration
   = note: `#[warn(unexpected_cfgs)]` on by default

warning: unexpected `cfg` condition value: `core_intrinsics`
  --> src/wm/macros.rs:27:11
   |
27 | #[cfg(not(feature = "core_intrinsics"))]
   |           ^^^^^^^^^^^^^^^^^^^^^^^^^^^ help: remove the condition
   |
   = note: no expected values for `feature`
   = help: consider adding `core_intrinsics` as a feature in `Cargo.toml`
   = note: see <https://doc.rust-lang.org/nightly/rustc/check-cfg/cargo-specifics.html> for more information about checking conditional configuration

For more information about this error, try `rustc --explain E0425`.
warning: `wminspect` (bin "wminspect") generated 2 warnings
error: could not compile `wminspect` (bin "wminspect") due to 2 previous errors; 2 warnings emitted
```

## Summary of Issues
1. **Major Errors**: 2 compilation errors in `src/wm/sheets.rs` related to missing bincode functions
2. **Warnings**: 2 warnings about unexpected cfg conditions in `src/wm/macros.rs`
3. **Deprecation**: `.cargo/config` should be renamed to `config.toml`
4. **Edition**: Project using 2015 edition, should be updated to 2024

## Critical Issues
- The bincode crate API appears to have changed, causing function name mismatches
- The `bc` alias likely refers to bincode, but the functions `deserialize_from` and `serialize_into` are not found

## Unit Tests Assessment

### Existing Tests
**Location**: `src/wm/macros.rs` and `src/wm/filter.rs`

#### Macro Tests (src/wm/macros.rs)
- `test_hashset1()` - Tests hashset macro with trailing comma
- `test_hashset2()` - Tests hashset macro without trailing comma
- `test_hashset3()` - Tests hashset macro with mixed string types
- `test_hashset4()` - Tests hashset count macro

#### Filter Tests (src/wm/filter.rs)
- `test_parse_rule()` - Tests basic rule parsing
- `test_parse_rule2()` - Tests rule parsing with actions
- `test_parse_rule3()` - Tests attribute-based rule parsing
- `test_parse_rule4()` - Tests complex ALL rule parsing
- `test_scan_tokens1()` through `test_scan_tokens5()` - Tests token scanning
- `test_parse_flow()` - Tests end-to-end parsing flow
- `test_wild_match()` - Tests wildcard matching functionality
- `test_whole()` - Tests filter integration
- `test_store1()`, `test_store2()`, `test_store3()` - Tests serialization

**Total Test Count**: 17 unit tests

### Test Status
❌ **All tests fail to compile** due to the bincode API incompatibility

## Runtime Behaviors Analysis

### CLI Modes
1. **Default Mode**: Lists all windows with their properties
2. **Monitor Mode** (`-m` or `monitor` subcommand): Continuously monitors window events
3. **Sheet Management** (`sheet` subcommand): Manages filter rule sheets
   - Load sheets from file (`--load`)
   - Compile rules (`--compile`)
4. **Grammar Display** (`--show-grammar`): Shows filter rule grammar

### Key Features
- **Window Filtering**: Complex rule-based filtering with wildcards
- **Sheet Formats**: Supports .rule (plain text), .json, and .bin formats
- **Window Properties**: Tracks geometry, attributes, names, IDs
- **Actions**: Pin windows or filter them out
- **Colorized Output**: Optional colored terminal output

### CLI Options
- `-v, --only-mapped`: Show only mapped windows
- `-c, --colored`: Output info with color
- `-m, --monitor`: Run in monitor mode
- `-f, --filter`: Apply filter rule
- `-o, --omit-hidden`: Omit hidden windows
- `-O, --no-override-redirect`: Ignore override-redirect windows
- `-s, --no-special`: Ignore special windows
- `-n, --num`: Show event sequence count
- `-d, --diff`: Highlight diffs between events
- `-C, --clients-only`: Trace clients of window manager only
- `--show-grammar`: Show detailed grammar for filter rule

## Sample Outputs / CLI UX

### Grammar Display
```
grammar:
    top -> ( item ( ';' item )* )?
    item -> cond ( ':' action)? 
        | 'clients'
    cond -> pred op VAL
        | ANY '(' cond (',' cond )* ')'
        | ALL '(' cond (',' cond )* ')'
        | NOT '(' cond ')'
        | 'clients'
    pred -> ID ('.' ID)*
    op -> '=' | '>' | '<' | '>=' | '<=' | '<>'
    action -> 'filter' | 'pin'
    ID -> STRING_LIT
    VAL -> STRING_LIT
    
pred could be:
    attrs.(map_state|override_redirect)
    geom.(x|y|width|height)
    id
    name
```

### Sample Rule Sheet
**File**: `sheets/mapped_only.rule`
```
attrs.map_state = Viewable: filter
```

### Build Status
❌ **Project fails to build** due to compilation errors
❌ **Tests fail to run** due to compilation errors
❌ **CLI cannot be executed** due to build failure

## Dependencies Analysis
- **XCB**: X11 protocol bindings for Linux window management
- **Clap 2.27**: Command-line argument parsing (outdated)
- **Bincode 2.0**: Binary serialization (API changed)
- **Serde**: Serialization framework
- **Regex**: Pattern matching
- **Colored**: Terminal color output
- **Crossbeam**: Concurrency primitives
- **Timer**: Timing utilities

## Regression Testing Baseline

### Current State
- ❌ Build: FAIL (2 compilation errors)
- ❌ Tests: FAIL (cannot compile)
- ❌ CLI Help: FAIL (binary not executable)
- ❌ Grammar Display: FAIL (binary not executable)
- ❌ Sheet Loading: FAIL (binary not executable)
- ❌ Monitor Mode: FAIL (binary not executable)

### Expected Functionality (Based on Code Analysis)
- Window inspection and monitoring
- Rule-based filtering with complex grammar
- Multiple output formats (JSON, binary, plain text)
- Real-time event monitoring
- Sheet management system
- Colorized terminal output
