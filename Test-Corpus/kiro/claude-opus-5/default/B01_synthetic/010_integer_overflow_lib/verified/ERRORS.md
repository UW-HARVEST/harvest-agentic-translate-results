# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Mechanical derivation

Every rejection-shaped construct was grepped for across the entire C source:

```sh
grep -n "return\|assert\|NULL\|errno\|-1\|if\|switch\|#ifdef\|#if\|ERROR\|goto\|exit" \
    c_src/src/driver.c c_src/include/driver.h
```

The only hits are the word "modify" inside the licence comment and the
`#ifndef DRIVER_H_` / `#endif` header guard. Concretely the C source contains:

| construct | count |
|-----------|-------|
| `return` statements | 0 (both functions are `void`) |
| error-return macros (`RETURN_ERROR` etc.) | 0 |
| `assert` | 0 |
| explicit range / bounds checks | 0 |
| null-pointer checks | 0 |
| pointer parameters (so: null is not a representable input) | 0 |
| enum parameters (so: no out-of-range enum input exists) | 0 |
| length / size parameters (so: no zero/oversized length input exists) | 0 |
| `if` / `switch` / conditional branches | 0 |
| min/max constants | 0 |
| error enums or status codes | 0 |
| `goto` / `exit` / `abort` | 0 |

The entire public surface is:

```c
void printHexCharLine (char charHex) { printf("%02x\n", charHex); }
void driver(char data) { char result = data + 1; printHexCharLine(result); }
```

Both take a single by-value `char` and return `void`. **There is therefore no
invalid input and no rejection path in this library**: every one of the 256
bit patterns a `char` can hold is accepted and produces output. The classic
generic boundaries (null pointer, zero length, oversized length, out-of-range
enum) are *not instantiable* here because the API has no pointer, length, or
enum parameter.

## Error-surface rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `printHexCharLine` | *(none exists)* — argument domain is the full 256-value `char` domain; no value is rejected | n/a — total function, always prints a line, returns `void` | `errors_e1_e2_no_rejection_path_exists` | [x] |
| E2 | `driver` | *(none exists)* — same, `data + 1` cannot trap (it is computed in promoted `int`, then truncated) | n/a — total function, always prints a line, returns `void` | `errors_e1_e2_no_rejection_path_exists` | [x] |

Because the table has no positive rows, Phase C is discharged by *proving the
absence* of a rejection path rather than by matching error codes. The tests
below do that empirically instead of trusting the grep.

## Phase C tests (in `tests/differential.rs`)

Each of these builds the stated hostile input, calls **both** `.so`s through
`libloading`, and asserts identical captured stdout — i.e. asserts that both
implementations *agree* on the non-rejection.

| test | what it establishes |
|------|---------------------|
| `errors_e1_e2_no_rejection_path_exists` | Exhaustively over all 256 `char` bit patterns, neither `printHexCharLine` nor `driver` rejects, aborts, or produces empty output; C and Rust agree byte-for-byte. Rows E1/E2. |
| `errors_boundary_values_one_past_range` | The four values one step past each signed/unsigned `char` sub-range boundary — `0x7F`/`0x80` (signed max / one past) and `0xFF`/`0x00` (unsigned max / one past, i.e. wraparound) — for both entry points. Covers "values one step past a documented valid range". |
| `errors_arithmetic_overflow_in_driver` | `driver(0x7F)`: `data + 1` overflows the *signed char* range. C computes it in `int` (no UB) and truncates on assignment; asserts Rust's `wrapping_add`-then-truncate agrees. |
| `errors_out_of_range_int_passed_as_char_arg` | Passes full-width `i32`/`u32` values with non-zero high bytes (`0x1234_5678`, `0xDEAD_BEEF`, `0xFFFF_FF00`, `0x0000_0100`, …) through a deliberately mis-declared `extern "C" fn(c_int)` symbol signature — the FFI analogue of "an out-of-range enum value", since a C `char` parameter, like a C enum, silently accepts any `int` at the ABI level. Asserts C and Rust truncate the register identically. |
| `errors_repeated_and_interleaved_calls_no_state_corruption` | 2000 seeded calls alternating between the two entry points; asserts no hidden state, no drift, and identical stdout stream/buffering behaviour. |

## Outcome

All rows pass. `errors_out_of_range_int_passed_as_char_arg` **found a real
divergence** on its first run — the Rust `printHexCharLine` did not narrow its
argument register the way GCC's does, so `printHexCharLine(0x100)` printed `100`
in Rust and `00` in C. Root cause, disassembly, and fix are written up in the
"Divergence found and fixed" section of `CONFIGS.md`. That row is also the *only*
one of the 20 that catches this class of bug, as `mutation_check.sh` mutant `m4`
demonstrates.

## Note on the "no error surface" conclusion

The claim "this library has no rejection path" is load-bearing, so it is not left
resting on a grep. `errors_e1_e2_no_rejection_path_exists` walks all 256 `char`
values through both entry points on both libraries and asserts each call produced
exactly one non-empty, all-hex-digit line — i.e. no input was refused, dropped,
or handled specially. If a future reader doubts the table, that test is the
evidence.

## Harness caveat

Both functions communicate only through `stdout`, so the tests capture file
descriptor 1 around each call batch. fd-1 redirection is process-wide, which is
incompatible with libtest's multi-threaded runner: its progress output landed
inside capture windows and produced spurious diffs (e.g. a captured line reading
`ffffffdtest errors_… FAILED`). The `differential` target is therefore declared
`harness = false` in `Cargo.toml` and runs every row sequentially from its own
`main`. This was a harness defect, not a translation defect.
