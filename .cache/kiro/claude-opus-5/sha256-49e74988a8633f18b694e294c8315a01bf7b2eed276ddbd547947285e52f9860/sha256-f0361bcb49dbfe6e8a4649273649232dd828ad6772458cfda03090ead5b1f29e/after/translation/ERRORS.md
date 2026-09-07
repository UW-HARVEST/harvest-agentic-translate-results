# ERRORS.md — error-surface table

Derived mechanically from the C source, not from docs or assumptions. The greps
below were run over the **entire** `c_src` tree (`src/` + `include/`), which is
two files totalling 75 lines.

## Mechanical derivation

```sh
$ grep -n "return" -r c_src/src c_src/include
(no matches)

$ grep -nE "NULL|assert|errno|ERROR|error|-1|EXIT|abort|exit" -r c_src/src c_src/include
(no matches)

$ grep -nE "\bif\b|\bswitch\b|#if|#ifdef|\?|&&|\|\|" -r c_src/src c_src/include
c_src/include/driver.h:24:#ifndef DRIVER_H_        <- include guard only

$ grep -nE "enum|#define|MAX|MIN|const" -r c_src/src c_src/include
c_src/include/driver.h:25:#define DRIVER_H_        <- include guard only
```

Findings:

* **0** `return` statements of any kind (both functions are `void` and fall off
  the end).
* **0** error-return macros, error enums, sentinel returns (`return -1`,
  `return NULL`), `assert`s, `errno` uses, `abort`/`exit` calls.
* **0** explicit range checks, null checks, min/max constants.
* **0** runtime `if`/`switch`/ternary/short-circuit branches. The only
  conditional in the whole library is the `for (i = 0; i < len; i++)` loop
  bound inside `static print_hex`.
* The public API is `void driver(int x)` — a single by-value `int`, no
  pointers, no lengths, no enums, no flags.

**Consequence:** the C library has *no rejection surface*. Every value of `int`
is accepted and processed identically; `driver` cannot report failure because it
returns `void`. The "expected C result" for an error-path test is therefore
"no rejection: prints the 16-byte hex image and returns normally", and the
differential assertion is that Rust produces the **byte-identical stdout** and
likewise does not abort, panic, or diverge.

This table is the exhaustive enumeration of every condition that *could* be an
error at the API boundary, including the generic C-API boundaries mandated even
when absent from the source.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `driver` | `floors == 0` (zero / falsy boundary, and the value `{0}` already initialises the field to) | no error; prints `00000000` + `03000000` + `0000000000000040` |
| 2 | `driver` | `floors == -1` (all-bits-set; classic error sentinel passed as data) | no error; prints `ffffffff03000000` `0000000000000040` |
| 3 | `driver` | `floors == INT_MAX` (`2147483647`) — upper boundary of the parameter type | no error; prints `ffffff7f...` |
| 4 | `driver` | `floors == INT_MIN` (`-2147483648`) — lower boundary of the parameter type | no error; prints `00000080...` |
| 5 | `driver` | `floors == INT_MAX` **+1 step past the documented range**, i.e. the bit pattern `0x80000000` reinterpreted as `int` (`INT_MIN`) — there is no representable value one step past `INT_MAX`, so the wrap-around image is the "one past the end" case | no error; identical to row 4 (`00000080...`) |
| 6 | `driver` | `floors == INT_MIN` **−1 step**, i.e. bit pattern `0x7fffffff` (`INT_MAX`) — no representable value below `INT_MIN`; wrap-around image | no error; identical to row 3 (`ffffff7f...`) |
| 7 | `driver` | out-of-range **enum-style** value crossing the FFI boundary: the C prototype is `int`, so an arbitrary 32-bit int with no "valid variant" (e.g. `0x7f7f7f7f`, `0xdeadbeef`, `0xcafebabe`) is a legal input the C accepts. There is no enum in the source, so *every* int is out-of-range-of-nothing and must be handled identically | no error; prints the raw little-endian image of the value |
| 8 | `driver` | **null pointer** boundary — N/A by construction: `driver` takes no pointer parameters, so no null can be passed. Verified by the header signature `void driver(int x);`. Covered as a negative/no-op row so the boundary is not silently skipped | not reachable through the public ABI; nothing to compare |
| 9 | `driver` | **zero / oversized length** boundary — N/A by construction: `driver` takes no length parameter. The only length in the library is `sizeof(house_t)`, a compile-time constant passed to `static print_hex` | not reachable through the public ABI; nothing to compare |
| 10 | `print_hex` (static) | `len <= 0` would skip the loop and print only `"\n"`; `len > sizeof(house_t)` would read out of bounds | **unreachable**: `print_hex` has internal linkage and its only call site passes the constant `sizeof(house_t)` (= 16). Not exported from the C `.so` (see `SYMBOLS.md`), so it cannot be driven from a differential test. Asserted indirectly: every `driver` call must emit exactly 16 hex byte-pairs + `\n` (33 bytes) |
| 11 | `driver` | repeated / interleaved invocation (state corruption, stale buffer reuse across calls) — `house` is a fresh automatic `{0}`-initialised object each call, so there is no persistent state to corrupt | no error; each call independently prints its own 33-byte line |

## Status

All 11 rows have a passing differential test in
`tests/differential.rs` (`phase_c_row_01` … `phase_c_row_11`). Rows 8, 9 and 10
are unreachable through the exported ABI, so they are asserted as the structural
invariants described above (header signature takes no pointer; no length
parameter exists; every call emits exactly 33 bytes) rather than by passing an
impossible argument.

Verified under every feature combination — `default`, `--no-default-features`,
`--all-features` — via `./run_differential.sh`: `26 passed; 0 failed`.

### Harness non-vacuity

Because this table asserts "no error is reported", it would be satisfied
trivially by a test that compares nothing. The harness was mutation-tested to
prove otherwise: five bugs injected into `src/lib.rs` were each caught in all
three feature combinations.

| injected bug | detected |
|--------------|----------|
| `house.bedrooms = 3` → `4` | yes (3/3 combos) |
| `%02x` → `%x` (drops zero padding) | yes (3/3 combos) |
| `unsigned char` → `signed char` promotion | yes (3/3 combos) |
| `house.bathrooms = 2.` → `2.5` | yes (3/3 combos) |
| trailing `'\n'` → `'\r'` | yes (3/3 combos) |
