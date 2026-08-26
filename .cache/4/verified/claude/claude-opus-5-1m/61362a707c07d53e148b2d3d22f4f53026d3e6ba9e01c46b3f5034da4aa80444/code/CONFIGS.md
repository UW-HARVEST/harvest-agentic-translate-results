# CONFIGS.md — CONFIGURATION-SURFACE TABLE (Phase A / Phase B)

Valid-input mirror of `ERRORS.md`. Axes derived mechanically from the branches
`c_src/src/lib.c` actually takes.

## Build-time configuration axes

`c_src/CMakeLists.txt` has **no** `option()`, `add_definitions`, or
`target_compile_definitions`, and `src/lib.c` + `include/lib.h` contain **no**
`#ifdef`/`#ifndef`/`#if`. Verified:

```
$ grep -n '#ifdef\|#ifndef\|#if \|option(\|add_definitions\|compile_definitions' \
      c_src/src/lib.c c_src/include/lib.h c_src/CMakeLists.txt
(no matches)
```

⇒ **exactly one** build configuration. `Cargo.toml` mirrors this with an empty
`default = []` and no other features, so the feature power set is `{ {} }`,
exercised as `--no-default-features`, default, and `--all-features`
(`./check_all_features.sh`).

## Runtime configuration axes (the options/shapes the C branches on)

| axis | values the C distinguishes | source |
|------|---------------------------|--------|
| `apply_bitmask.operation` | `0` (`&0xF0`), `1` (`&0x0F`), `2` (`\|0xAA`), `3` (`^0x55`), else (identity) | `lib.c:57-68` switch |
| `shift_array` guard | `positions<=0`, `positions>=size`, `0<positions<size` | `lib.c:36` |
| `shift_array` shape | `size` ∈ {0,1,2,3,4,…,N}; `positions` ∈ {1..size-1} | `lib.c:37-40` |
| `process_string` | first byte `NUL` vs non-`NUL`; length; **signed** `char` high-bit bytes | `lib.c:45-48` |
| `compare_allocations` | `*ptr1 > 0` (i.e. `val1 > 0`) → `+10`, else `+0`; address order 1/2/3 | `lib.c:102-111` |
| `arity4.param1 % 4` | `0,1,2,3` and **negative** `-1,-2,-3` (C `%` truncates toward zero) → mask selector | `lib.c:142` |
| `arity4.param3` | `== 0` (skip) vs `!= 0` (`result = result*param3/100`), sign of `param3` | `lib.c:152-154` |
| `arity4.param4` | `== 0` (skip) vs `!= 0` (`result += param4`) | `lib.c:156-158` |
| `arity.len` (low byte, **unsigned**) | `<2` → `-1`; `==2` → `arity2`; `==3` → `arity3`; `>=4` → `arity4` | `lib.c:171-181` |
| entry-point level | low-level (`shift_array`, `process_string`, `apply_bitmask`, `init_matrix`, `compare_allocations`), mid (`arity4`), wrappers (`arity2`, `arity3`), dispatcher (`arity`) | all 9 exports |

## Configuration rows

Every row is driven with **many randomized inputs** (fixed seed `0x2545F491`,
xorshift64* PRNG — `tests/common/mod.rs`), not a single hand-picked value.
`[x]` = C and Rust byte-identical across all randomized inputs for that row.
Test file: `tests/valid_paths.rs`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `shift_array` | `0 < positions < size`, `size=4`, `positions=1` (the shape `arity4` itself uses); random `i32` contents incl. `INT_MIN`/`INT_MAX` | [x] |
| C2 | `shift_array` | `0 < positions < size` swept over **all** `size ∈ 1..=16` × **all** `positions ∈ -2..=size+1`; random contents; full buffer compared + 4-int guard region either side | [x] |
| C3 | `shift_array` | large shape: `size=1024`, `positions ∈ {1, 512, 1023}`; random contents | [x] |
| C4 | `process_string` | non-empty ASCII, random lengths `1..=256`, random printable bytes | [x] |
| C5 | `process_string` | high-bit bytes `0x80..=0xFF` (**negative** `c_char`), random lengths; plus embedded-`NUL` buffers (`strlen` stops at first `NUL`) | [x] |
| C6 | `process_string` | long buffer, `4096` bytes | [x] |
| C7 | `apply_bitmask` | `operation` ∈ {0,1,2,3} × random `value` (incl. `INT_MIN`, `INT_MAX`, `0`, `-1`); plus 2 000 random `(value, operation)` pairs covering the `default` branch | [x] |
| C8 | `compare_allocations` | `val1 > 0` (→ `+10`) × random `val2`; parity-neutral 2-call batches | [x] |
| C9 | `compare_allocations` | `val1 <= 0` (→ `+0`), incl. `val1 = 0`, `INT_MIN`, random negatives × random `val2` | [x] |
| C10 | `init_matrix` | 12-int destination, 4-int sentinel guard before/after: verifies the exact 12 values written **and** that nothing outside is touched | [x] |
| C11 | `arity4` | `param1 % 4 == 0` (mask `&0xF0`) × `param3=0`, `param4=0`; random `param2` | [x] |
| C12 | `arity4` | `param1 % 4 ∈ {1,2,3}` (masks `&0x0F`, `\|0xAA`, `^0x55`) × `param3=0`, `param4=0`; random `param2` | [x] |
| C13 | `arity4` | `param1 % 4 ∈ {-1,-2,-3}` (negative `param1` ⇒ `default` identity branch) × random others | [x] |
| C14 | `arity4` | `param3 != 0`, `param4 == 0`: exercises `result*param3/100` incl. negative `param3` (truncation toward zero) | [x] |
| C15 | `arity4` | `param3 == 0`, `param4 != 0`: exercises `result += param4` only | [x] |
| C16 | `arity4` | `param3 != 0` **and** `param4 != 0`: both post-adjustments, composed | [x] |
| C17 | `arity4` | overflow shapes: all 4 params drawn from `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX, 100, -100, 50}` — **full 10⁴ cross-product** (10 000 cases) exercising wrapping `*`, `+`, and `/100` | [x] |
| C18 | `arity4` | fully random 4-tuples (5 000 cases, seeded) | [x] |
| C19 | `arity4` | `param1 > 0` vs `param1 <= 0` crossed with the `param3`/`param4` on/off matrix (the `compare_allocations` `+10` interaction) | [x] |
| C20 | `arity2` | wrapper: random `(p1,p2)`; also asserts equality with `arity4(p1,p2,0,0)` on both libraries | [x] |
| C21 | `arity2` | boundary params from the `{INT_MIN..INT_MAX}` set above, full 10×10 cross-product | [x] |
| C22 | `arity3` | wrapper: random `(p1,p2,p3)` incl. `p3=0` and `p3!=0`; also asserts equality with `arity4(p1,p2,p3,0)` | [x] |
| C23 | `arity3` | boundary params, full 10×10×10 cross-product (1 000 cases) | [x] |
| C24 | `arity` | `len` low byte `== 2` → `arity2` path; 4-element `params` (upper elements must be ignored); random contents | [x] |
| C25 | `arity` | `len` low byte `== 3` → `arity3` path; random contents | [x] |
| C26 | `arity` | `len` low byte `== 4` → `arity4` path; random contents | [x] |
| C27 | `arity` | `len` low byte `> 4` (`5`, `10`, `100`, `255`) → `arity4`, reads only 4 params; `params` array sized `len` with random contents | [x] |
| C28 | `arity` | `len` with high bits set that truncate to a **valid** dispatch value: `258`→`2`, `259`→`3`, `260`→`4`, `65538`→`2` | [x] |
| C29 | `arity` | `len` negative truncating to `>=4`: `-1`→`255`, `-2`→`254`, `-4`→`252` → `arity4` | [x] |
| C30 | `arity` | **all 256** low-byte values × fixed params, plus randomized params per value | [x] |
| C31 | `arity` | `params` pointing into the middle of a larger buffer (offset base pointer) — confirms no over-read past index 3 via guard sentinels | [x] |
| C32 | composed pipeline | `arity` **and** `arity4` interleaved in a long randomized sequence (1 000 iterations = 2 000 calls, mixed lengths) so allocator state, mask selection and post-adjustments compose across calls; each library runs its sequence in its **own fresh process** and the full result sequences are compared (see `ERRORS.md` Note D for why in-process sequence comparison is unsound) | [x] |

## Row C30 addendum

Because `len` is the most ABI-subtle parameter in the library (declared `int` in
`include/lib.h`, defined `unsigned char` in `src/lib.c`), row C30 drives it with
all 256 low-byte values reached through four different high-bit patterns
(`0`, `0x100`, `0x10000`, `-0x10000`) **and** with 2 000 fully random `i32`
values.

## Test-sensitivity check (mutation testing)

Passing tests are only meaningful if they can fail, so the suite was validated
by mutating `src/lib.rs`, rebuilding, and confirming test failures. Every
mutation that is observable through the FFI was caught:

| mutation of the Rust translation | tests failed |
|----------------------------------|--------------|
| `arity4`: `param1.wrapping_rem(4)` → `rem_euclid(4)` (classic negative-`%` bug) | 2 |
| `arity`: remove the 8-bit `len` truncation | 2 |
| `apply_bitmask`: one bit changed in `mask3` | 2 |
| `arity`: `len < 2` → `len < 3` | 2 |
| `shift_array`: `positions < size` → `positions <= size` | 1 |
| `shift_array`: reversed `memmove` direction | 3 |
| `shift_array`: zero-fill one element short | 3 |
| `init_matrix`: last element `12` → `13` | 3 |
| `process_string`: empty string returns `1` instead of `0` | 4 |
| `arity4`: `/100` → `/101` | 2 |
| `arity3`: pads `param4` with `1` instead of `0` | 1 |
| `compare_allocations`: `*uninit_ptr > 0` → `>= 0` | 8 |
| `shift_array`: `positions > 0` → `positions >= 0` | 0 — **equivalent mutant** |

The last row is not a coverage gap: with `positions == 0` the C body would
`memmove(arr, arr, size*4)` (a copy onto itself) and run the zero-fill loop zero
times, so the mutant is behaviourally identical to the original.

`src/lib.rs` was restored to the original translation afterwards (verified with
`diff`).
