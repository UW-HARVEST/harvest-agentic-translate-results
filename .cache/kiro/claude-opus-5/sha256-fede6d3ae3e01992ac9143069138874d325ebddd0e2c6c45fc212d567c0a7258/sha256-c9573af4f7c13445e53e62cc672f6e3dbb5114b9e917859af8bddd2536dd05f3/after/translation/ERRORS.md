# ERRORS.md — error / rejection surface table (Phase C gate)

## Mechanical derivation

Every construct that could reject input was grepped for in the *whole* C tree
(`c_src/src/staticalias.c`, `c_src/include/staticalias.h`):

| grep pattern | hits in `.c` | hits in `.h` |
|--------------|--------------|--------------|
| `RETURN_ERROR` | 0 | 0 |
| `assert` | 0 | 0 |
| `NULL` | 0 | 0 |
| `errno` | 0 | 0 |
| `-1` (error sentinel) | 0 | 0 |
| `exit` / `abort` | 0 | 0 |
| `enum` | 0 | 0 |
| `switch` | 0 | 0 |
| `#if` | 0 | 1 (`#ifndef STATICALIAS_H_` include guard only) |
| `if` | 2 (`if(*outer >= inner)` + its `else`) | 3 (all inside the include guard) |
| `return` | 3 (`return &inner;`, `return outer;`, bare `return;`) | 0 |

**Result: the C library has NO explicit error surface.** There is no error enum,
no sentinel return, no null check, no range check, no assert, no `min`/`max`
constant, and neither function can report failure — `static_alias` always
returns a non-null pointer and `driver` returns `void`. The only conditional in
the library (`*outer >= inner`) selects between two *valid* behaviours and is
therefore a Phase B configuration axis, not an error.

Consequently every row below is an *implicit* rejection / boundary condition:
the generic C-API boundaries the task requires (null pointer, zero and
oversized lengths, one-step-past-range values, out-of-range enum values) mapped
onto the two signatures that actually exist. `iterations` is the only
length-like parameter; there is no enum parameter anywhere in the API, so the
out-of-range-enum class degenerates into "arbitrary out-of-domain `int` passed
across FFI", which is covered by rows 4–11.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `static_alias` | `outer == NULL` | no check exists → `*outer` dereferences the null page → process killed by `SIGSEGV` (11). Same fatal signal must be observed for Rust. | `err_01_static_alias_null_pointer` (differential, in a `fork`ed child) | [x] |

> **Divergence found and fixed via row 1.** With Rust's debug assertions on (the
> default for the `dev` profile) the standard library's UB checks turned this
> input into `panicked at src/lib.rs:78: null pointer dereference occurred` →
> non-unwinding panic → `SIGABRT` (6), whereas the C library dies with `SIGSEGV`
> (11). `[profile.dev] debug-assertions = false` / `overflow-checks = false`
> (and the same in `[profile.release]`) was added to `Cargo.toml` so the Rust
> library reproduces the C library's behaviour under *every* profile, not just
> the optimised one.
| 2 | `driver` | `iterations` "length" is zero (`iterations == 0`) | loop body never runs, no `printf`, no state change to `inner`, returns normally | `err_02_driver_zero_iterations` | [x] |
| 3 | `driver` | `iterations` negative — one step past the low end of the valid count range (`-1`, `INT_MIN`) | `i < iterations` false on entry → identical to row 2 (silent no-op, *not* an error, *not* a huge loop) | `err_03_driver_negative_iterations` | [x] |
| 4 | `static_alias` | `*outer == INT_MAX` while `inner >= 1` ⇒ then-branch, `inner += INT_MAX` **overflows** signed `int` (C UB; gcc at `-O0` wraps two's-complement) | `inner` becomes `(inner + INT_MAX)` wrapped, returns `&inner`; Rust must reproduce the same wrapped value | `err_04_static_alias_then_overflow` | [x] |
| 5 | `static_alias` | `*outer == INT_MIN` (one step past the low end of `int`), `inner >= 1` ⇒ else-branch `*outer += inner` cannot overflow, returns `outer` | `*outer == INT_MIN + inner`, return value `== outer` (not `&inner`), `inner` unchanged | `err_05_static_alias_min_probe` | [x] |
| 6 | `static_alias` | else-branch **underflow**: `inner` driven negative (via row 4 wraparound) and `*outer` near `INT_MIN` ⇒ `*outer += inner` wraps below `INT_MIN` | wrapped two's-complement result, pointer identity `== outer` | `err_06_static_alias_else_underflow` | [x] |
| 7 | `static_alias` | aliasing boundary: the returned `&inner` fed straight back in, so `outer == &inner` ⇒ `inner += *outer` reads and writes the same object | `inner` doubles (`2 * inner`, wrapping), returns `&inner` again | `err_07_static_alias_self_alias_doubling` | [x] |
| 8 | `static_alias` | exact `>=` boundary: `*outer == inner` (the `==` half of the comparison, the classic off-by-one) | then-branch, `inner` doubles, returns `&inner`, caller's object untouched | `err_08_static_alias_equal_boundary` | [x] |
| 9 | `static_alias` | one step below the boundary: `*outer == inner - 1` | else-branch, `*outer += inner`, returns `outer` | `err_09_static_alias_below_boundary` | [x] |
| 10 | `driver` | `initial_value == INT_MIN` / `INT_MAX` combined with the largest exercised `iterations` — repeated overflow inside the printed sequence | identical `%d\n` byte stream from both libraries, including every wrapped negative value | `err_10_driver_extreme_values` | [x] |
| 11 | `driver` | oversized length: `iterations` far larger than any sensible count (`INT_MAX` is untestable in finite time — `100_000` is used as the "oversized" proxy, which is already past the point where `inner` overflows) | full identical byte stream, no truncation, no early exit | `err_11_driver_oversized_iterations` | [x] |
| 12 | both | out-of-range "enum"/flag class: the API takes no enum, so arbitrary `int` bit patterns (including `INT_MIN`, `INT_MAX`, `-1`) are passed for every `int` parameter across the FFI boundary | no validation exists; both libraries must accept and process every bit pattern identically | `err_12_no_enum_arbitrary_int_fuzz` | [x] |

All 12 rows are checked off; see `tests/differential.rs` for the corresponding
`err_*` tests.
