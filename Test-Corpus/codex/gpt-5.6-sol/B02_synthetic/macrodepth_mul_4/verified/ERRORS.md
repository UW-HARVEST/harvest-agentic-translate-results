# Error Surface

Mechanical scan:

```text
rg 'RETURN_ERROR|return -1|return NULL|assert|if|switch|case|default|MIN|MAX|ERROR' c_src/src
```

The C sources contain no error macros, assertions, null checks, range-error
returns, error enums, or min/max constants. The generated accumulator's
`default` switch arm is ordinary successful behavior and is covered in
`CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `main` | `argc < 3` | write `usage: <argv[0]> A B\n` to `stderr`; return `2` |

## Generic FFI Boundary Probes

These are required even though the C source does not reject them explicitly.

| # | entry point(s) | boundary | expected C behavior | status |
|---|----------------|----------|---------------------|--------|
| G1 | all integer functions and `G_OP` | `0`, `INT_MIN`, `INT_MAX`, and overflow-producing operand pairs | return the C result | [x] |
| G2 | `use_generated` | `INT_MIN`, `-1`, `7`, and `INT_MAX` | take `default`; return the operation's initial accumulator | [x] |
| G3 | `main` | null `argv` with `argc < 3` | process terminates with `SIGSEGV` while evaluating `argv[0]` | [x] |
| G4 | `main` | `argc == 3` and null `argv[1]` | process terminates with `SIGSEGV` in `atoi` | [x] |

There are no length parameters or enum parameters in the public C surface, so
zero/oversized-length and out-of-range-enum probes are not applicable.
