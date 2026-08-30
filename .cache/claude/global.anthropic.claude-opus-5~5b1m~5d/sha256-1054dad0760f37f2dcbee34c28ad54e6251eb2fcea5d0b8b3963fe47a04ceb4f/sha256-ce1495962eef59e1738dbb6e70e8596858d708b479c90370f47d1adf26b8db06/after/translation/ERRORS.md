# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical grep results

```
$ grep -nE 'return|assert|NULL|errno|-1|if|switch|while|\?|#ifdef|#if|MIN|MAX|exit|abort' c_src/src/driver.c
35:    for (int i = 0; i < len; i++) {
```

- `RETURN_ERROR`-style macros: **0**
- `return <error>` statements: **0** (both functions are `void`; there is not a
  single `return` statement in the file)
- `assert(...)`: **0**
- explicit range checks: **0**
- NULL / pointer validity checks: **0**
- `errno` usage: **0**
- error enums / sentinel values: **0**
- `exit` / `abort`: **0**
- conditionals: exactly ONE — the `i < len` loop condition in `print_hex`, which
  is loop control, not input rejection.
- min/max constants: **0**

## Error-surface table

The public API is `void driver(int x)`. It has **no error surface**: it returns
`void`, takes a single unconstrained `int` by value, dereferences no
caller-supplied pointer, allocates nothing, and performs no validation. Every one
of the 2^32 possible `int` arguments is *valid* and produces output rather than a
rejection. Consequently the table below has no rejection rows, and the rows that
do exist are the mandated generic-boundary rows, whose "expected C result" is
**success (no rejection)** — the Rust must agree, byte for byte, that these are
NOT errors.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `driver` | *(no rejection branch exists anywhere in the library)* | n/a — library has zero error returns |
| E2 | `driver` | `x = 0` (zero-value boundary) | NO error. Prints `00000000030000000000000000000040\n`, returns normally |
| E3 | `driver` | `x = INT_MAX` (`2147483647`, one step below overflow) | NO error. Prints `ffffff7f...`, returns normally |
| E4 | `driver` | `x = INT_MIN` (`-2147483648`, extreme negative boundary) | NO error. Prints `00000080...`, returns normally |
| E5 | `driver` | `x = -1` (all-bits-set / classic error sentinel value) | NO error. Prints `ffffffff...`, returns normally |
| E6 | `driver` | `x = INT_MAX + 1` passed as the *unsigned* bit pattern `0x80000000` across FFI (i.e. one step past the signed range, wrapped by the C ABI) | NO error. Identical to E4 (`00000080...`); C truncates/reinterprets, does not reject |
| E7 | `driver` | `x = 0xFFFFFFFF` passed as unsigned across FFI (one step past `UINT_MAX`-adjacent range) | NO error. Identical to E5 (`ffffffff...`) |
| E8 | `driver` | out-of-range "enum-like" `int` values with no valid variant (`-2`, `4`, `999999`, `0x7FFFFFFE`, `0xDEADBEEF` as int) — the C `int` parameter accepts any `int`, so these are real inputs | NO error / no special-casing. Each is printed as its little-endian 4-byte pattern; there is no enum validation to diverge on |
| E9 | `driver` | value equal to the hard-coded `bedrooms` constant (`x = 3`) — aliasing/overwrite hazard | NO error. `bedrooms` must still be `3`; output `03000000030000000000000000000040\n` |
| E10 | `print_hex` (internal) | `len <= 0` — loop body never executes, only the trailing newline is printed | Unreachable from the public ABI: `driver` always passes `sizeof(house_t)` == 16. Not externally triggerable; `print_hex` is `static` and NOT exported by either `.so` (see SYMBOLS.md), so there is no FFI path to it |
| E11 | `print_hex` (internal) | `p == NULL` — would be UB in C | Unreachable from the public ABI: `driver` always passes `&house` (a valid stack address). Not exported ⇒ no FFI path |
| E12 | `driver` | NULL-pointer argument | Not applicable: `driver` takes no pointer arguments. There is no pointer to pass as NULL |
| E13 | `driver` | zero / oversized length argument | Not applicable: `driver` takes no length argument. The only length in the library is the compile-time constant `sizeof(house_t)` |

### Status

All rows are covered by `translation/tests/error_paths.rs`. Rows E1, E10–E13 are
"structurally impossible" rows: they are documented, and the test asserts the
structural fact (no such symbol / no such parameter) rather than calling a
non-existent entry point. Rows E2–E9 are executed differentially against both
`.so`s and asserted to produce identical, non-error, byte-identical output.
