# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

## Build-time configuration

Enumerated mechanically from both build files:

* `Cargo.toml` has **no `[features]` section** at all, so the feature power-set is
  a single element: the empty set. `cargo check/test --no-default-features` and a
  plain `cargo build` are the same configuration.
* `c_src/CMakeLists.txt` contains no `option()`, no `target_compile_definitions`,
  no `add_definitions` and no `#ifdef`-driven variants; `driver.c`/`driver.h`
  contain no `#ifdef` other than the `DRIVER_H_` include guard.

```sh
$ grep -c "^\[features\]" Cargo.toml                 # -> 0
$ grep -nE "option\(|target_compile_definitions|add_definitions" \
      c_src/CMakeLists.txt                           # -> no matches
$ grep -n "#if" c_src/src/driver.c c_src/include/driver.h
c_src/include/driver.h:24:#ifndef DRIVER_H_          # include guard only
```

**⇒ Exactly one build configuration exists.** It is nevertheless verified under
both Cargo profiles, because `panic = "abort"` and optimization change codegen:

| profile | flags | verified |
|---------|-------|----------|
| `dev` (debug) | `-C debug-assertions=on`, unwinding | [x] |
| `release` | `-O`, `panic = "abort"` | [x] |

## Runtime configuration axes

Derived from the branches the C actually takes:

* **A1 — `driver`'s `useGood` flag.** The one runtime option in the library
  (`driver.c:50 if (useGood)`). Two states: zero → `bad()`, non-zero → `good()`.
  Because it is a truthiness test, *all* non-zero `int`s collapse to one arm — so
  the interesting shapes are `0`, `1`, `-1`, `INT_MIN`, `INT_MAX`, random
  non-zero, and a 64-bit value whose low 32 bits are zero.
* **A2 — entry point / call depth.** Four public entry points at two depths:
  `printIntPtrLine` (lowest level, takes data), `good` / `bad` (mid, no args),
  `driver` (top, one arg). Depth matters because `bad()` reads a stack slot, so
  `good();bad()` (same depth) behaves differently from `driver(1);bad()` (mixed).
* **A3 — pointed-to `int` value.** `printIntPtrLine` renders it with `%d`:
  `0`, `±1`, `INT_MIN`, `INT_MAX`, single/multi digit, negative (sign path),
  and randomized values.
* **A4 — pointer provenance / shape.** `printIntPtrLine` takes a bare
  `const int *`: stack, heap (`malloc`), `static`/`.bss`, interior of an array,
  misaligned (offset 1/2/3 into a byte buffer), and last-4-bytes-of-a-mapping.
* **A5 — call multiplicity / sequencing.** `printf` to a redirected `stdout` is
  block-buffered, so output accumulates: one call vs. many, and C-then-Rust
  interleaving in the same process (both `.so`s share one libc `stdout`).

Rows are the pruned cross-product — the combinations the C actually distinguishes.
Every row is driven through **both** `.so`s via `libloading` and compared
byte-for-byte; rows marked *randomized* use 256+ seeded inputs (seed `0x5EED_1234`,
`tests/differential.rs`).

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printIntPtrLine` | stack `int`, value `0` | [x] |
| 2 | `printIntPtrLine` | stack `int`, value `5` (the value `good()` uses) | [x] |
| 3 | `printIntPtrLine` | stack `int`, values `1`, `-1`, `9`, `10`, `-9`, `-10` (digit-count / sign boundaries) | [x] |
| 4 | `printIntPtrLine` | stack `int`, `INT_MIN` = `-2147483648` | [x] |
| 5 | `printIntPtrLine` | stack `int`, `INT_MAX` = `2147483647` | [x] |
| 6 | `printIntPtrLine` | stack `int`, all powers of two ±1 (`1<<0 … 1<<31`, 96 values) | [x] |
| 7 | `printIntPtrLine` | stack `int`, **randomized** full-range `i32` (256 values, seeded) | [x] |
| 8 | `printIntPtrLine` | **heap** `int` (`malloc`), randomized values | [x] |
| 9 | `printIntPtrLine` | **static/`.bss`** `int`, randomized values | [x] |
| 10 | `printIntPtrLine` | **interior of an array** — element `k` of a 16-`int` array, all `k`, randomized values | [x] |
| 11 | `printIntPtrLine` | **misaligned** pointer, byte offsets 1/2/3 into a buffer, randomized values | [x] |
| 12 | `printIntPtrLine` | pointer to the final 4 bytes of a heap block (next byte out of range) | [x] |
| 13 | `printIntPtrLine` | many sequential calls (32) accumulating into one buffered stream | [x] |
| 14 | `printIntPtrLine` | C and Rust calls **interleaved** in one process, shared `stdout` buffer | [x] |
| 15 | `good` | single call, no options | [x] |
| 16 | `good` | repeated calls (1, 2, 8 times) — buffering / repeatability | [x] |
| 17 | `driver` | `useGood = 1` (canonical true) | [x] |
| 18 | `driver` | `useGood` = `-1`, `2`, `7`, `INT_MAX`, `INT_MIN` (non-zero, incl. out-of-range enum values) | [x] |
| 19 | `driver` | `useGood` = **randomized** non-zero `i32` (256 values, seeded) | [x] |
| 20 | `driver` | `useGood` = `1` repeated (1, 2, 8 times) | [x] |
| 21 | `driver` + `good` | mixed sequence `driver(1); good(); driver(3)` — top and mid entry points together | [x] |
| 22 | `good` → `bad` | `good()` then `bad()`, **same call depth** → deterministic `5\n5\n` (see `ERRORS.md` note A) | [x] |
| 23 | `good` → `bad` → `bad` | `good()` then `bad()` twice → `5\n5\n5\n` | [x] |
| 24 | `driver` → `driver` | `driver(1); driver(0)` → `5\n5\n` | [x] |
| 25 | `driver` → `driver` → `driver` | `driver(1); driver(0); driver(0)` → `5\n5\n5\n` | [x] |
| 26 | `driver` | `useGood = 0` in isolation — indeterminate UB, outcome **recorded not asserted** | [x] |
| 27 | `bad` | `bad()` in isolation — indeterminate UB, outcome **recorded not asserted** | [x] |
| 28 | mixed depth | `driver(1); bad()` and `good(); driver(0)` — depth-mismatched UB, recorded not asserted | [x] |

Rows 26–28 are the genuinely indeterminate CWE-457 paths: the C's own output is
not reproducible across runs there (`ERRORS.md` note A), so asserting byte
equality would be asserting a fiction. They are still executed against both
`.so`s and their outcomes recorded, and rows 22–25 pin down every part of `bad()`'s
behaviour that *is* deterministic in the C.
