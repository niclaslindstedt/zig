# Conditions

Condition expressions control whether a step runs based on the current
variable state. They are specified via the `condition` field on a step.

## Syntax

Conditions are simple expressions evaluated against workflow variables.

### Comparison Operators

| Operator | Description          | Example                    |
|----------|----------------------|----------------------------|
| `==`     | Equal                | `status == "done"`         |
| `!=`     | Not equal            | `status != "pending"`      |
| `<`      | Less than            | `score < 8`                |
| `>`      | Greater than         | `score > threshold`        |
| `<=`     | Less than or equal   | `retries <= max_retries`   |
| `>=`     | Greater than or equal| `score >= 8`               |

### Truthy Checks

A bare variable name evaluates as a truthy check:

```toml
condition = "approved"
```

A value is **truthy** if it is non-empty, not `"false"`, and not `"0"`.

A value is **falsy** if it is empty, `"false"`, or `"0"`.

## Operand Resolution

Operands in condition expressions are resolved in this order:

1. **String literals** — quoted values (`"done"`, `'pending'`) are used as-is
2. **Variable lookup** — unquoted identifiers are looked up in the variable map
3. **Numeric literals** — if no variable matches, the token is used as-is

## Comparison Behavior

When both operands can be parsed as numbers, numeric comparison is used.
Otherwise, lexicographic (string) comparison is used.

```toml
# Numeric comparison: 5 < 8 → true
condition = "score < 8"

# String comparison: "alpha" == "alpha" → true
condition = "status == \"done\""

# Variable-to-variable: compares their current values
condition = "retries < max_retries"
```

## Examples

### Skip a step when quality is high enough

```toml
[[step]]
name = "refine"
prompt = "Improve the output"
condition = "score < threshold"
```

### Run only when a specific status is set

```toml
[[step]]
name = "deploy"
prompt = "Deploy to production"
condition = "status == \"approved\""
```

### Gate on a boolean flag

```toml
[[step]]
name = "notify"
prompt = "Send notification"
condition = "notifications_enabled"
```

### Compare two variables

```toml
[[step]]
name = "retry-step"
prompt = "Try again"
condition = "attempts < max_attempts"
```

## Using Conditions for Routing

Conditions enable dispatcher patterns where different steps handle different
cases:

```toml
[[step]]
name = "simple-handler"
depends_on = ["classify"]
condition = "complexity == \"simple\""

[[step]]
name = "complex-handler"
depends_on = ["classify"]
condition = "complexity == \"complex\""
```

## See Also

- `zig docs variables` — variable declarations and data flow
- `zig docs patterns` — orchestration patterns using conditions
- `zig docs zwf` — full `.zwf` format reference
