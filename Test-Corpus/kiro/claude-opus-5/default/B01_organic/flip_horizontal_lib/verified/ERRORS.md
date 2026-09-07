# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every rejection path in the C library was searched for mechanically:

```
$ cd c_src && grep -nE 'return|assert|NULL|errno|if *\(|switch|#if|-1|else|ERROR|enum' \
      src/lib.c include/lib.h
(no matches — exit status 1)
```

Findings from that grep, over the ENTIRE C source (`src/lib.c`, 19 lines;
`include/lib.h`, 16 lines):

* 0 `return` statements (the function is `void`; there is no error channel at
  all — no error code, no sentinel, no out-param status).
* 0 `assert` / `NULL` checks / `errno` writes.
* 0 `if` / `switch` / `#if` — the only branches in the whole library are the two
  loop conditions `i < flips` and `j < w`.
* 0 error enums or named error constants; 0 range checks; 0 min/max constants.

So the C library **rejects nothing**. It validates no input, and every input is
"accepted" in the sense that control always falls off the end of the function.
The observable result for any input is therefore only one of two things:

* **`OK/NOP`** — returns normally, buffer left in some defined state; or
* **`FAULT`** — the C code dereferences memory it was handed and the process
  traps (SIGSEGV), because the C performs no check that would prevent it.

Rows below are the distinct *conditions* the C code reaches without a check,
which is what an error table must record for a library with no error returns.
"expected C result" is the ground truth the Rust must reproduce EXACTLY.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `flip_horizontal` | `img == NULL` | FAULT: unconditional `img->pix` load at entry → SIGSEGV. No null check exists. Rust must also SIGSEGV (same signal), not panic/abort with a Rust message. | [x] |
| 2 | `flip_horizontal` | `img->pix == NULL`, `h >= 2`, `w >= 1` (so the swap loop actually runs) | FAULT: `*a` deref of NULL+offset → SIGSEGV. | [x] |
| 3 | `flip_horizontal` | `img->pix == NULL`, `h < 2` (`flips == 0`) | OK/NOP: `pix` is loaded but never dereferenced; returns cleanly. NOT a fault. | [x] |
| 4 | `flip_horizontal` | `img->pix == NULL`, `w == 0`, any `h` | OK/NOP: outer loop runs but inner `j < w` is false immediately; no deref. Returns cleanly. | [x] |
| 5 | `flip_horizontal` | `h == 0` | OK/NOP: `flips = 0`, no iterations, buffer untouched. | [x] |
| 6 | `flip_horizontal` | `h == 1` (one step below the smallest `h` that does any work) | OK/NOP: `flips = 1/2 = 0`, buffer untouched. | [x] |
| 7 | `flip_horizontal` | `h < 0` (e.g. `-1`, `-2`, `-7`, `INT_MIN`) | OK/NOP: C `/` truncates toward zero, so `flips = h/2 <= 0` and `i < flips` is false at `i = 0`. Buffer untouched. Must NOT be treated as unsigned / must not loop. | [x] |
| 8 | `flip_horizontal` | `w < 0` (e.g. `-1`, `-5`), `h >= 2` | OK/NOP: inner `j < w` false immediately for every row. Buffer untouched. Pointers `pix + w*i` are formed but never dereferenced. | [x] |
| 9 | `flip_horizontal` | `w == 0`, `h >= 2` | OK/NOP: zero-length rows, buffer untouched. | [x] |
| 10 | `flip_horizontal` | `w == 0 && h == 0` (both zero / empty image) | OK/NOP. | [x] |
| 11 | `flip_horizontal` | oversized `w`/`h` product: `w * (h - i - 1)` overflows `int` (`w = 65536, h = 65536`) | FAULT: the wrapped (negative) offset is dereferenced. `w*(h-0-1) = 65536*65535` overflows to `-65536`, so the second row pointer is `pix - 262144` bytes → SIGSEGV. Verified in child processes: C and Rust both SIGSEGV. | [x] |
| 12 | `flip_horizontal` | `h == INT_MIN` (`h/2` at the extreme of the range) | OK/NOP: `INT_MIN/2` is negative → `flips < 0` → no iterations. | [x] |
| 13 | `flip_horizontal` | `w == INT_MIN` | OK/NOP: `j < w` false immediately. | [x] |
| 14 | `flip_horizontal` | `h == INT_MAX` | OK/NOP in principle: `flips = 1_073_741_823` and (with `w <= 0`) the inner loop is empty, so no memory is touched — but it is a ~1e9-iteration no-op loop in BOTH implementations, i.e. a test-runtime hazard rather than a behavioral axis. Covered as: `w == INT_MAX` with short-circuiting `h`, plus `w == 0` with `h` up to 1_000_001 as a bounded stand-in. Documented deliberately rather than silently dropped. | [x] |
| 15 | `flip_horizontal` | out-of-range "enum" value across FFI | N/A — the API has **no** enum, flag, or mode parameter anywhere (`grep enum` → 0 hits). The only parameter is a struct pointer, so there is no invalid-discriminant class of input for this library. Recorded for completeness; the adjacent class (arbitrary/garbage `int` values in `w`/`h`, which C accepts for any `int`) is covered by rows 5–14. | [x] |

Rows 1, 2 and 11 are *deliberate* faults: the correct Rust behavior is to fault
the same way, so they are tested by forking child processes and comparing the
termination signal from the C `.so` against the Rust `.so`. Measured, for every
fault case, on the shipped release artifact:

| case | C | Rust |
|------|---|------|
| `null_img` (row 1) | SIGSEGV (11) | SIGSEGV (11) |
| `null_pix_active` w=1,h=2 (row 2) | SIGSEGV (11) | SIGSEGV (11) |
| `null_pix_active_wide` w=16,h=4 (row 2) | SIGSEGV (11) | SIGSEGV (11) |
| `offset_overflow` w=h=65536 (row 11) | SIGSEGV (11) | SIGSEGV (11) |

All rows checked: **15/15**.

## Which Rust artifact the error paths are asserted against

`tests/common/mod.rs` loads `target/release/libflip_horizontal_lib.so` — the
artifact the project's documented `cargo build --release` produces and the exact
file an external `dlopen` consumer would load. This matters for rows 1 and 2:

A **debug** Rust build compiles in Rust's null-pointer / "unsafe precondition"
UB checks, which convert a null dereference into a controlled `abort()`
(SIGABRT, 6) instead of a hardware fault (SIGSEGV, 11). That is a Rust
debugging feature, not translated behavior — the C has no such check and cannot
have one. Rows 1 and 2 therefore diverge (11 vs 6) under a debug artifact and
match exactly under the release artifact. `assert_same_fault` accepts that one
named difference *only* when the harness has been explicitly pointed at another
build via `HARVEST_RUST_SO`; the default path demands exact signal equality.
Every other row, and all of Phase B, passes under both artifacts.

## Divergence found and fixed during Phase C

Running Phase C against a debug artifact exposed a **real** translation bug, not
just the UB-check artifact above: `src/lib.rs` used `ptr::offset` / `ptr::add`
and `core::ptr::swap`. Those carry debug-only preconditions that abort the
process when the *address calculation* overflows or leaves the allocation —
conditions the C reaches routinely and never traps on (negative `w` forms
`pix + w*i` without dereferencing it; row 11 overflows the offset). The
translation now uses `wrapping_offset` / `wrapping_add` and a literal
`let t = *a; *a = *b; *b = t;` swap, so its behavior no longer depends on
whether `debug_assertions` is enabled. Row 11 went from aborting in debug to
matching C's SIGSEGV in both profiles.

## Non-vacuity

`scripts/mutation_check.sh` builds six deliberately wrong variants of the
translation and requires the suite to reject each one. All six are caught:

| mutant | deviation | caught by |
|--------|-----------|-----------|
| `unsigned_h_div` | `h / 2` computed on `u32` | negative-`h` no-op rows (7, 12) |
| `off_by_one_rows` | pairs row `i` with `h-i-2` | 19 assertion failures across Phase B |
| `drop_alpha` | preserves the destination alpha channel | 19 assertion failures (row 20 patterns) |
| `skip_last_column` | inner loop stops one pixel early | 19 assertion failures |
| `one_extra_flip` | `flips = h/2 + 1` | `h == 0` / `h == 1` rows (5, 6) |
| `inner_ge_zero` | `j != w` instead of `j < w` | negative-`w` rows (8, 13) |
