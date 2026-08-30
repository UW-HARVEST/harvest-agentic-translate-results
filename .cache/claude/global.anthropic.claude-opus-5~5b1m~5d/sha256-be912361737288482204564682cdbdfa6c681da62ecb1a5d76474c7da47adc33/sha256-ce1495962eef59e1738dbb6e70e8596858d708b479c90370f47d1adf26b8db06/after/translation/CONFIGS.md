# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from the branches the C in `c_src/src/driver.c` actually
takes, plus the full set of exported entry points from `SYMBOLS.md` (not just the
`driver.h` convenience wrapper).

## Axes the C code actually branches on

1. **Entry point** (4 exported symbols, from `nm -D`):
   * `printIntPtrLine(const int *)` — lowest level; takes the only real data
     input in the library.
   * `good()` — nullary; constructs `int data = 5` and calls level 1.
   * `bad()` — nullary; passes an uninitialised `int *` to level 1 (CWE-457).
   * `driver(int)` — top-level dispatcher; the only header-declared function.
   The convenience wrapper (`driver`) reaches only `5\n` and the UB path, so the
   value-dependent behaviour is reachable ONLY by calling `printIntPtrLine`
   directly. Both levels are driven.
2. **`useGood` truthiness** — the single `if` in the library. `0` → `bad()`,
   everything else → `good()`. Sub-shapes that a buggy translation would treat
   differently: positive small, negative, `INT_MIN`, `INT_MAX`, low-byte-zero
   (`0x100`), low-16-zero (`0x7fff_0000`), 64-bit-dirty (`0x1_0000_0000`).
3. **Pointed-to `int` value shape** for `printIntPtrLine`, because the output is
   `printf("%d\n", ...)` and `%d` formatting is value-dependent: zero, positive,
   negative, `INT_MIN` (no positive counterpart — the `-2147483648` printing
   trap), `INT_MAX`, single-digit vs 10-digit widths, values whose unsigned
   reinterpretation differs (`0xFFFFFFFF` → `-1`), sign-extension traps
   (`0x8000_0000`).
4. **Pointer/storage shape** the pointer refers to — the C takes the address of a
   local (`&data` in `good`), but `printIntPtrLine` accepts any `const int *`, so
   the read must be exactly 4 bytes at exactly that address regardless of
   provenance: stack local, heap allocation, static/const memory, interior
   element of an array (index 0 / middle / last), misaligned address, address at
   the very end of a valid mapping.
5. **Call sequencing / stdout state** — `printf` buffers; repeated and
   interleaved calls must produce identical byte streams and identical ordering
   in one process, including when stdout is a pipe (fully buffered) vs a
   terminal-like fd. Ordering across a `good()`/`printIntPtrLine` mix is
   observable, so a composed pipeline is tested, not only single calls.

There are **no** compile-time `#ifdef`s, no runtime option/flag setters, no
global state, no byte-order handling and no element-type polymorphism in this
library, so those axes are empty by construction (documented here so the absence
is a derived fact rather than an omission).

## Configuration table

Every row is exercised against BOTH `.so`s through their exported symbols and
compared byte-for-byte on stdout (plus exit code / signal). Rows marked
*randomized* use ≥256 property-style inputs from a fixed-seed
(`0x5EED_1234_ABCD_0001`) SplitMix64 generator.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printIntPtrLine` | stack local, value `0` | [x] |
| 2 | `printIntPtrLine` | stack local, value `5` (the value `good()` uses) | [x] |
| 3 | `printIntPtrLine` | stack local, value `-1` (`0xFFFFFFFF`, unsigned-reinterpretation trap) | [x] |
| 4 | `printIntPtrLine` | stack local, value `INT_MAX` = `2147483647` (10-digit width) | [x] |
| 5 | `printIntPtrLine` | stack local, value `INT_MIN` = `-2147483648` (no positive counterpart; `%d` negation trap) | [x] |
| 6 | `printIntPtrLine` | stack local, value `0x8000_0000` reinterpreted (sign-extension trap) | [x] |
| 7 | `printIntPtrLine` | stack local, all decimal widths: `±1, ±9, ±10, ±99, ±100, ±999 ... ±10^9` boundary sweep | [x] |
| 8 | `printIntPtrLine` | stack local, *randomized* full-range `i32` (256 inputs, fixed seed) | [x] |
| 9 | `printIntPtrLine` | heap allocation (`malloc`ed `int`), *randomized* values | [x] |
| 10 | `printIntPtrLine` | static / global memory (`.data`), *randomized* values | [x] |
| 11 | `printIntPtrLine` | read-only static memory (`.rodata` const), fixed value | [x] |
| 12 | `printIntPtrLine` | array element, index `0` (first) — array of 64 *randomized* ints | [x] |
| 13 | `printIntPtrLine` | array element, middle index — array of 64 *randomized* ints | [x] |
| 14 | `printIntPtrLine` | array element, last index (out-of-range-index / off-by-one trap) — array of 64 *randomized* ints | [x] |
| 15 | `printIntPtrLine` | every element of a 64-element *randomized* array, walked in sequence (exercises pointer arithmetic + repeated buffered writes) | [x] |
| 16 | `printIntPtrLine` | misaligned pointer (`(int*)(bytes.as_ptr()+1)`) over *randomized* byte fill — confirms 4-byte little-endian read at an odd address | [x] |
| 17 | `printIntPtrLine` | pointer to the final 4 valid bytes of a mapping (page-end boundary) | [x] |
| 18 | `printIntPtrLine` | called 512 times back-to-back with *randomized* values in ONE process (stdout full-buffer flush behaviour, >4 KiB of output crossing the BUFSIZ boundary) | [x] |
| 19 | `good` | nullary, single call — must print exactly `5\n` | [x] |
| 20 | `good` | nullary, called 256 times in one process — must print `5\n` 256× with no drift (catches a stale-stack / reused-slot bug) | [x] |
| 21 | `driver` | `useGood = 1` → `good()` path → `5\n` | [x] |
| 22 | `driver` | `useGood = 2` (non-zero, not 1 — "out of range enum" but still truthy) | [x] |
| 23 | `driver` | `useGood = -1` (negative truthy) | [x] |
| 24 | `driver` | `useGood = INT_MAX` | [x] |
| 25 | `driver` | `useGood = INT_MIN` (truthy, sign bit only) | [x] |
| 26 | `driver` | `useGood = 0x100` (low BYTE zero, whole value truthy — bool/`u8` truncation trap) | [x] |
| 27 | `driver` | `useGood = 0x7fff_0000` (low 16 bits zero, truthy — `u16` truncation trap) | [x] |
| 28 | `driver` | `useGood` *randomized* non-zero `i32` (256 inputs, fixed seed) → all must print `5\n` | [x] |
| 29 | `driver` | `useGood = 0` → `bad()` path (UB, compared on exit code + signal + stdout) | [x] |
| 30 | `driver` | called 256 times with `useGood = 1` in one process — buffered output ordering | [x] |
| 31 | `bad` | nullary, direct call — CWE-457 defect must be *preserved*, not fixed | [x] |
| 32 | composed pipeline | in ONE process, interleave `driver(1)`, `good()`, `printIntPtrLine(&v)` over a *randomized* script of 256 steps — asserts ordering + buffering of the composed sequence, which per-wrapper tests cannot see | [x] |
| 33 | stdout shape | all of the above run with stdout redirected to a **pipe** (fully buffered, the default for the harness) | [x] |
| 34 | stdout shape | row 18 + row 32 re-run with stdout redirected to a **regular file** (also fully buffered, different `st_blksize` path in glibc) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the complete set
of feature combinations is the single default (empty) configuration. Phase D
still runs the matrix script, which enumerates features from `Cargo.toml` and
therefore executes exactly one combination: `--no-default-features` plus the
default build. Both are verified.

## Row → test coverage map

Every row above is discharged by a named test in `tests/differential.rs`. Rows
29 and 31 reach the CWE-457 path, whose outcome is indeterminate, so they are
verified by the Phase C undefined-behaviour tests rather than by byte-equality
(see `ERRORS.md` rows 7-8 for why, and for the anti-sanitization gate that makes
them meaningful).

| CONFIGS row(s) | test |
|---|---|
| 1-6   | `cfg_01_06_print_int_ptr_line_value_traps` |
| 7     | `cfg_07_print_decimal_width_sweep` |
| 8     | `cfg_08_print_randomized_stack` |
| 9     | `cfg_09_print_randomized_heap` |
| 10    | `cfg_10_print_randomized_static` |
| 11    | `cfg_11_print_rodata` |
| 12-15 | `cfg_12_15_print_array_indices` |
| 16    | `cfg_16_print_misaligned_pointer` |
| 17    | `cfg_17_print_page_end_boundary` |
| 18    | `cfg_18_print_burst_buffering` |
| 19    | `cfg_19_good_single_call` |
| 20    | `cfg_20_good_repeated` |
| 21-27 | `cfg_21_27_driver_truthy_shapes` |
| 28    | `cfg_28_driver_randomized_nonzero` |
| 29    | `err_08_driver_zero_dispatches_to_bad` (UB path) |
| 30    | `cfg_30_driver_repeated` |
| 31    | `err_07_bad_is_undefined_behaviour` (UB path) |
| 32    | `cfg_32_composed_pipeline` |
| 33-34 | `cfg_33_34_stdout_pipe_vs_regular_file` |

## How to run

```sh
bash verify_all.sh          # builds C + Rust, symbol parity, all feature combos, mutation check
cargo build && cargo build --release && cargo test   # the suite alone
```

Note that `cargo test` does NOT rebuild the `cdylib`, so the two `cargo build`
invocations are required; the suite has a staleness guard that fails loudly
rather than silently testing an outdated `.so`.
