# Domain Specific Language (DSL) Reference

## Overview

The wminspect DSL provides a powerful and flexible way to define window filtering rules. It supports complex logical expressions, wildcard matching, and various window properties for precise window management.

## Formal Grammar (EBNF)

```ebnf
(* Top-level rule definition *)
rules ::= rule (';' rule)*

(* Individual rule with optional action *)
rule ::= condition (':' action)?

(* Condition expressions *)
condition ::= logical_expr | predicate_expr | 'clients'

(* Logical expressions *)
logical_expr ::= 'any' '(' condition (',' condition)* ')'
               | 'all' '(' condition (',' condition)* ')'  
               | 'not' '(' condition ')'

(* Predicate expressions *)
predicate_expr ::= predicate operator value

(* Predicates *)
predicate ::= 'id' | 'name' | attribute_predicate | geometry_predicate

(* Attribute predicates *)
attribute_predicate ::= 'attrs' '.' ('map_state' | 'override_redirect')

(* Geometry predicates *)
geometry_predicate ::= 'geom' '.' ('x' | 'y' | 'width' | 'height')

(* Operators *)
operator ::= '=' | '<>' | '>' | '<' | '>=' | '<='

(* Values *)
value ::= string_literal | number | boolean | map_state_value

(* Map state values *)
map_state_value ::= 'Viewable' | 'Unmapped' | 'Unviewable'

(* Actions *)
action ::= 'filter' | 'pin'

(* Literals *)
string_literal ::= STRING_TOKEN | QUOTED_STRING
number ::= INTEGER
boolean ::= 'true' | 'false' | '0' | '1'
```

## Predicates

### Window Identification

| Predicate | Type | Description | Example |
|-----------|------|-------------|---------|
| `id` | Window ID | Matches window ID (hex or decimal) | `id=0x1000001` |
| `name` | String | Matches window name/title | `name=*firefox*` |

### Window Attributes

| Predicate | Type | Description | Example |
|-----------|------|-------------|---------|
| `attrs.map_state` | MapState | Window visibility state | `attrs.map_state=Viewable` |
| `attrs.override_redirect` | Boolean | Override redirect attribute | `attrs.override_redirect=true` |

### Window Geometry

| Predicate | Type | Description | Example |
|-----------|------|-------------|---------|
| `geom.x` | Integer | X coordinate | `geom.x>100` |
| `geom.y` | Integer | Y coordinate | `geom.y<500` |
| `geom.width` | Integer | Window width | `geom.width>=800` |
| `geom.height` | Integer | Window height | `geom.height<=600` |

### Special Predicates

| Predicate | Description | Example |
|-----------|-------------|---------|
| `clients` | Matches only WM-managed client windows | `clients` |

## Operators

### Comparison Operators

| Operator | Description | Applicable To |
|----------|-------------|---------------|
| `=` | Equals | All types |
| `<>` | Not equals | All types |
| `>` | Greater than | Numeric values |
| `<` | Less than | Numeric values |
| `>=` | Greater than or equal | Numeric values |
| `<=` | Less than or equal | Numeric values |

### Logical Operators

| Operator | Description | Example |
|----------|-------------|---------|
| `any()` | Logical OR | `any(name=*term*, name=*bash*)` |
| `all()` | Logical AND | `all(geom.width>800, geom.height>600)` |
| `not()` | Logical NOT | `not(attrs.override_redirect=true)` |

## Actions

| Action | Description | Example |
|--------|-------------|---------|
| `filter` | Include window in monitoring (default) | `name=*browser*: filter` |
| `pin` | Pin window for persistent highlighting | `name=*important*: pin` |

## Operator Precedence

1. **Parentheses** - Highest precedence
2. **Logical NOT** (`not()`)
3. **Logical AND** (`all()`)
4. **Logical OR** (`any()`) - Lowest precedence

## Examples

### Basic Filtering

```
# Filter by window name
name=*firefox*

# Filter by geometry
geom.width>800

# Filter by attributes
attrs.map_state=Viewable
```

### Wildcard Matching

```
# Simple wildcard
name=*browser*

# Multiple wildcards
name=*fire*fox*

# Question mark for single character
id=0x10000??

# Mixed patterns
name=term*al?
```

### Logical Expressions

```
# OR condition
any(name=*firefox*, name=*chrome*, name=*browser*)

# AND condition
all(geom.width>800, geom.height>600, attrs.map_state=Viewable)

# NOT condition
not(attrs.override_redirect=true)

# Nested conditions
any(
    all(name=*browser*, geom.width>1000),
    all(name=*terminal*, geom.height>400)
)
```

### Actions

```
# Pin specific windows
name=*important*: pin;

# Filter and pin in one rule
attrs.map_state=Viewable: filter;
name=*dialog*: pin;

# Complex rule with actions
any(
    all(name=*dev*, geom.width>1200),
    name=*terminal*
): filter;
```

### Complete Rule Examples

```
# Development environment monitoring
any(name=*vscode*, name=*terminal*, name=*browser*): filter;
attrs.override_redirect=true: pin;

# Large window filtering
all(geom.width>1000, geom.height>800): filter;

# Exclude small or hidden windows
not(any(
    all(geom.width<100, geom.height<100),
    geom.x<0,
    geom.y<0
)): filter;
```

## Serialization Formats

### 1. Plain Text Rules (`.rule`)

Human-readable format for writing and editing rules:

```
# comments are supported
name=*browser*: filter;
attrs.map_state=Viewable: pin;
```

### 2. JSON Format (`.json`)

Structured format for programmatic access:

```json
[
  {
    "action": "FilterOut",
    "rule": {
      "Single": {
        "pred": { "Name": null },
        "op": "Eq",
        "matcher": { "Wildcard": "*browser*" }
      }
    }
  }
]
```

### 3. Binary Format (`.bin`)

Optimized binary format for fast loading:

```
Binary serialized using bincode crate
```

## CLI Usage

### Rule Compilation

```bash
# Compile plain text rules to JSON
wminspect sheet --compile rules.rule compiled.json

# Compile plain text rules to binary
wminspect sheet --compile rules.rule compiled.bin

# Compile with error handling
wminspect sheet --compile complex.rule output.json 2>errors.log
```

### Rule Loading

```bash
# Load plain text rules
wminspect sheet --load rules.rule

# Load JSON rules
wminspect sheet --load compiled.json

# Load binary rules
wminspect sheet --load compiled.bin

# Load rules and start monitoring
wminspect sheet --load rules.rule monitor
```

### Rule Validation

```bash
# Show grammar for validation
wminspect --show-grammar

# Test rule syntax (compile without output)
wminspect sheet --compile test.rule /dev/null

# Validate and apply inline
wminspect -f "name=*test*" --monitor
```

## Advanced Features

### String Literals

```
# Simple strings
name=firefox

# Quoted strings with spaces
name='Firefox Browser'
name="Terminal Emulator"

# Escaped characters in quotes
name="Name with \"quotes\""
```

### Numeric Values

```
# Decimal integers
geom.width>800

# Hexadecimal (for IDs)
id=0x1000001

# Negative values
geom.x>-100
```

### Case Sensitivity

- String comparisons are case-sensitive
- Keywords (`any`, `all`, `not`, `filter`, `pin`) are case-insensitive
- Attribute values (`Viewable`, `true`, `false`) are case-insensitive

### Error Handling

Common parsing errors and solutions:

```
# Invalid syntax
name=*browser  # Missing closing quote or wildcard
# Solution: name=*browser*

# Invalid operator
name~=browser  # Invalid operator
# Solution: name=*browser*

# Missing action separator
name=browser filter  # Missing colon
# Solution: name=browser: filter
```

## Performance Considerations

- **Compilation**: Binary format loads ~10x faster than JSON
- **Wildcards**: Simple wildcards (`*text*`) are faster than complex patterns
- **Nesting**: Deeply nested logical expressions may impact performance
- **Caching**: Compiled rules are cached for subsequent uses

## Migration Guide

### From 0.2.x to 0.3.x

```
# Old format
window.name=*browser*

# New format
name=*browser*
```

### Adding New Features

When extending the DSL:

1. Update the grammar in `src/wm/filter.rs`
2. Add new predicates to the `Predicate` enum
3. Implement parsing logic in `parse_cond()`
4. Update serialization support
5. Add tests and documentation
