# Error Surface

Mechanical scan inputs:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|enum|NULL|MIN|MAX|if[[:space:]]*\(' ../c_src/include ../c_src/src
```

The only explicit `if` checks in the C source select valid conversion
branches. There are no error-return statements, error enums, assertions,
explicit null checks, length parameters, or rejecting range checks.
Consequently, the explicit C rejection table has zero rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

## Mandatory generic FFI boundaries

These are not C rejection branches, but Phase C requires applicable generic
boundaries to be exercised. Because dereferencing a null pointer is undefined
behavior in C, the recorded result below is the observed behavior of the built
x86-64 library in a subprocess, not a portable API guarantee.

| # | function | boundary | observed C result | status |
|---|----------|----------|-------------------|--------|
| G1 | `hsl_to_rgb` | `src == NULL`, valid writable `dest` | subprocess terminates with `SIGSEGV` | [x] |
| G2 | `hsl_to_rgb` | `dest == NULL`, valid readable `src` with `s == 0` | subprocess terminates with `SIGSEGV` | [x] |

Length boundaries and invalid enum discriminants are not applicable: the sole
API has no length or enum parameters. The public header documents no valid
numeric range; out-of-conventional-range float values are accepted and are
covered as valid configurations in `CONFIGS.md`.
