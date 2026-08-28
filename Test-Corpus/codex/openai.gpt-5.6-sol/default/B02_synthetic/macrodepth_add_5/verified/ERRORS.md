# Error Surface

Mechanical search covered all 218 lines in `../c_src/src/` for return
sentinels, error macros/enums, assertions, null/range checks, conditionals,
switch defaults, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| 1 | `main` | `argc < 3` | print `usage: %s A B\n` to `stderr`; return `2` | [x] |

There are no error-return macros, assertions, error enums, explicit pointer
checks, length parameters, or min/max constants. The `use_generated` switch
default is valid behavior rather than rejection: every `n < 0` or `n >= 7`
returns the selected operation's initial accumulator.

## Generic FFI Boundaries

`op_add`, `op_sub`, `op_mul`, `helper_call`, `helper_ptr`, and
`use_generated` accept scalar `int` values only. There are no enum, length, or
buffer arguments. `main` is the only pointer-taking entry point:

| # | function | boundary | expected C result | verified |
|---|----------|----------|-------------------|----------|
| B1 | `main` | negative and zero `argc`, with a valid `argv[0]` | same rejection as row 1 | [x] |
| B2 | `main` | `argc = INT_MAX`, with valid `argv[0..2]` | arguments beyond index 2 ignored; return `0` | [x] |
| B3 | `main` | `argc >= 3`, `argv = NULL` | process receives the same fatal signal while dereferencing `argv` | [x] |

Passing null element pointers to C `atoi`, or arithmetic inputs that overflow a
signed C `int`, invokes undefined behavior and is not assigned a required
result.
