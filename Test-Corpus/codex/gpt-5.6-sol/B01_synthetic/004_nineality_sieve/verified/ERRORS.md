# Error Surface

Mechanically inspected constructs in `c_src/src/main.c`: two `if` statements,
two `return 1` statements, and no assertions, error enums, null checks, range
checks, or min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `main` | `argc != 2` | Writes `Error: should only be a single (integer) argument!\n` to stdout and returns `1`; `argv` is not dereferenced. | [x] |
| 2 | `main` | After `strtol(argv[1], &end, 10)`, `end == argv[1]` because no characters were converted. This includes empty, whitespace-only, sign-only, and leading nonnumeric strings. | Writes `Error: first argument must be an integer!\n` to stdout and returns `1`. | [x] |

## Generic FFI boundaries

The tests must additionally cover these inputs through the branches above:

| boundary | C-defined case | tested |
|----------|----------------|--------|
| Null pointer | `argc == 0`, `argv == NULL`; rejected before dereference | [x] |
| Null unused element | `argc == 2`, `argv[0] == NULL`, valid `argv[1]`; accepted because `argv[0]` is never read | [x] |
| Zero count | `argc == 0` | [x] |
| Oversized count | `argc == INT_MAX` | [x] |
| One past required count | `argc == 3` | [x] |
| Out-of-range enum | Not applicable; the API has no enum parameter | [x] |

`argc == 2` with `argv == NULL` or `argv[1] == NULL` has undefined behavior in
the C source (`argv[1]`/`strtol(NULL, ...)`), so it is not a C rejection result
and is not included as a differential error case.
