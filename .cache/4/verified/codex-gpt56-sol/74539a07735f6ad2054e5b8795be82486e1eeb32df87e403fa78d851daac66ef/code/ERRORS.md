# Error Surface

The C source was mechanically searched for error returns, assertions, explicit
range/null checks, enums, min/max constants, conditional branches, and
preprocessor branches:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|\
assert[[:space:]]*\(|#[[:space:]]*(if|ifdef|ifndef)|\
\b(if|switch)[[:space:]]*\(|\b(MIN|MAX)\b|enum' \
  c_src/src c_src/CMakeLists.txt
```

The search has no matches. `main` has one unconditional `return 0`, which is
the normal success result rather than a rejection.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no C rejection branches. Scanner conversion failure and EOF are not
reported as errors: `scanf`'s return is ignored, the initialized `+0.0` remains
in place, `driver(+0.0)` runs, and `main` returns `0`. Those observable paths
are covered as valid behavior in `CONFIGS.md`.

Generic FFI boundary applicability:

| Entry point | Null pointers | Lengths | Enums/ranges | Status |
|-------------|---------------|---------|--------------|--------|
| `driver(double)` | not applicable | not applicable | every IEEE-754 class, signs, boundaries, and randomized payloads | [x] |
| `main(void)` | not applicable | empty/invalid/overflowing stdin forms | not applicable | [x] |
