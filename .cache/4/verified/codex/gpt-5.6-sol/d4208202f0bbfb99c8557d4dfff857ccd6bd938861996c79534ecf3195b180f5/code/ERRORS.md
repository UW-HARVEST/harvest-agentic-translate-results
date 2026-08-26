# Error Surface

Mechanical source scan:

```text
rg 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|if[[:space:]]*\(|switch[[:space:]]*\(|#[[:space:]]*(if|ifdef|ifndef)|\b(MIN|MAX)[A-Za-z0-9_]*\b|enum' c_src/src/main.c
```

The scan returns no matches. The C implementation has no error-return macro,
error enum, assertion, explicit range check, null check, min/max constant, or
input-rejection branch. Its only return statement is the unconditional
successful `return 0` from `main`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

## Generic FFI Boundaries

These are mandatory boundary cases, not explicit C rejection branches. They
must run in child processes because the C implementation dereferences the null
pointer through `strchr`.

| # | function | boundary input | expected C result | verified |
|---|----------|----------------|-------------------|----------|
| G1 | `foo` | `in == NULL`, nonzero `c` | process terminates with `SIGSEGV` | [x] |
| G2 | `driver` | `in == NULL` | process terminates with `SIGSEGV` | [x] |

There are no pointer-plus-length APIs, enums, or documented numeric ranges, so
zero/oversized lengths and out-of-range enum values do not apply.
