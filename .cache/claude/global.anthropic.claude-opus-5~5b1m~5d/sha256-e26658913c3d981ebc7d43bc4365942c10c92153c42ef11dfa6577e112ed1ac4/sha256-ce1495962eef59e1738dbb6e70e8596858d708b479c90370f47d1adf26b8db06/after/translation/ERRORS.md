# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/driver.c`. The exhaustive grep for every
rejection construct in the whole library is:

```
$ grep -n 'return\|assert\|NULL\|if\|else\|switch\|#if\|exit\|abort\|-1' c_src/src/driver.c
31:    if (line != NULL)
```

(the only other hits are in the licence comment block)

That is the **complete** rejection surface. Concretely, the library contains:

- error-return macros (`RETURN_ERROR` &c.): **none**
- `return -1` / `return NULL` / error enums / status codes: **none** — all five
  functions are `void` and return no value at all
- `assert` / `abort` / `exit`: **none**
- explicit range checks, min/max constants: **none** — no numeric input exists
- null checks: **one** (`driver.c:31`)
- `switch` / `#ifdef` branches: **none**

So the table has exactly one row.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `printLine` | `line == NULL` — the `if (line != NULL)` guard at `driver.c:31` is false | Guard falls through, `puts` is **not** called. Function returns normally (`void`, no error code). Net effect: **zero bytes** written to `stdout`, no crash. | `err_01_print_line_null` | [x] |

## Generic FFI boundary conditions

The task also mandates covering the boundaries every C API has, even when not in
the table above. Several are **not instantiable** for this library, and it is
worth recording *why*, so their absence is a derived fact rather than an
oversight:

| boundary | applicable? | covered by |
|----------|-------------|------------|
| Null pointer argument | yes — `printLine(NULL)` | row 1 / `err_01_print_line_null` |
| Zero length | yes — the empty string `""` is the zero-length input | `err_02_print_line_empty` |
| Oversized length | yes — no length argument exists, but the NUL-terminated string may be arbitrarily long, incl. past libc's `BUFSIZ` stdout buffer | `err_03_print_line_oversized` |
| Value one step past a valid range | **n/a** — no function takes a numeric/bounded argument | — (documented, not testable) |
| Out-of-range enum value across FFI | **n/a** — the library declares no `enum` and no function takes an integer parameter, so there is no int-with-no-valid-variant to pass | — (documented, not testable) |
| Unterminated / non-UTF-8 bytes | yes — `puts` is byte-oriented, so arbitrary non-UTF-8 bytes are valid input that Rust must not mangle or reject | `err_04_print_line_non_utf8`, `cfg_*` randomized rows |
| Argument-less functions given no state | yes — `bad`/`good`/`driver` take no arguments and read no state, so they have no invalid input; called for parity anyway | `cfg_05`–`cfg_07` |
| Misaligned / interior-NUL pointer | yes — a pointer into the middle of a buffer, and a buffer whose first byte is NUL | `err_02_print_line_empty`, `err_05_print_line_interior_nul` |

**Every row is checked off with a passing differential test** that asserts the
two implementations produce the *same* observable result (identical captured
`stdout` bytes and identical non-crashing return), not merely that "both did
something".
