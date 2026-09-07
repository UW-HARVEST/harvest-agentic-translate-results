# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` (branch inventory from the same
`grep -n -E '\bif\b|\bswitch\b|#if|continue|break'` run recorded in
`ERRORS.md`).

## Entry points

`c_src/include/lib.h` declares exactly one symbol, and `nm -D` confirms it is
the only exported one:

```c
int memchra2(int a, int b, int c, int d);
```

There are **no convenience wrappers vs. low-level entry points** to distinguish:
`memchra2` *is* the lowest-level public entry point. The eight helpers
(`memchra`, `process_buffer`, `int_to_float_bits`, `process_strings`,
`safe_sum_array`, `interpret_as_int`, `count_occurrences`,
`complex_iteration`) are `static`, so they are only reachable through
`memchra2`, and driving them means choosing `(a,b,c,d)` tuples that steer each
one. Every row below therefore names the helper path(s) it drives.

## Runtime options / modes / flags

**None.** There is no configuration struct, no setter, no global, no mode enum,
no `#ifdef` in the translation unit (`grep -n '#if' c_src/src/lib.c` → no
matches), and no compile-time feature in `translation/Cargo.toml`. The entire
configuration surface is the *shape of the four integer arguments*, so the axes
below are input-shape axes.

## Axes the C actually branches on

| axis | site | distinct states |
|------|------|-----------------|
| A1 — sign of each argument | `snprintf("test%d-%d-%d-%d")` emits a `-` per negative value, which `count_occurrences`/`memchra` then count | 2⁴ = 16 sign patterns → `dash_count ∈ 3..7` → `result += dash_count*10` |
| A2 — `a` reinterpreted as `float` | `if (f > 0.0f && f < 1000.0f)` (line 152) | (i) `a ≤ 0` → not taken; (ii) `1 ≤ a < 0x3F800000` → taken, `0 < f < 1` so `(int)f == 0`; (iii) `0x3F800000 ≤ a < 0x447A0000` → taken, `1 ≤ f < 1000`, `(int)f ∈ 1..999`; (iv) `a ≥ 0x447A0000` → not taken (`f ≥ 1000`, `+inf`, or `NaN`) |
| A3 — decimal width of the formatted text | length of `buffer` drives `strlen`, `memchra`'s `n`, and `process_buffer`'s loop | shortest `"test0-0-0-0"` (11 bytes) … longest `"test-2147483648-…"` (51 bytes); 64-byte `snprintf` cap is never reached |
| A4 — `buf_sum > 0` | line 157 | `buffer` holds only ASCII 45–57 (`-` and digits) plus `"test"`, all positive, and is non-empty → the guard is always **true**; the false branch is unreachable (recorded as row C13 so the Rust is checked for not inverting it) |
| A5 — low bytes of `b`, `c`, `d` | `interpret_as_int` reads `{b&0xFF, c&0xFF, d&0xFF, 0}` as a little-endian `int`, then `result ^= …` | `0x00`, `0xFF`, `0x80` (byte sign bit), mixed; top byte pinned to 0 so the value is always in `0 … 0x00FFFFFF` |
| A6 — low bytes of all four args | `complex_iteration` folds `result ^= (int)((unsigned)*i & 0xFF)` over 4 elements | xor-cancelling patterns (all equal low bytes → 0) vs. non-cancelling |
| A7 — signed wraparound | `safe_sum_array` accumulates `int sum += *i`; `memchra2` accumulates into `int result` | sums that stay in range vs. sums that overflow `int` in both directions |
| A8 — fixed-shape helper inputs | `process_strings(test_strings, 4, "test")`, `safe_sum_array(values, 4)`, `interpret_as_int(bytes, 4)`, `complex_iteration(values, 4)` | single state each (`count`/`len` are literals) — pinned, not variable; covered by every row |
| A9 — FFI argument reinterpretation | the exported ABI itself | called as `(i32,i32,i32,i32)->i32` and as `(u32,u32,u32,u32)->u32` |

## Table (one row per combination the C treats differently)

Every row is exercised with **many randomized tuples** (`SplitMix64`, fixed
seed `0x5EED_1234_ABCD_9876`) drawn from that row's constrained domain, not a
single hand-picked value. Both `.so`s are called through `libloading` and the
returned `int`s are compared bit-for-bit.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `memchra2` | A1 = all 16 sign patterns × A3 mixed widths; magnitudes random in `±1..±999_999_999` (drives `count_occurrences`/`memchra` dash counting, `process_buffer`, `safe_sum_array`, `complex_iteration`) | [x] |
| C2 | `memchra2` | A2 state (i): `a ≤ 0`, float term omitted; `b,c,d` random | [x] |
| C3 | `memchra2` | A2 state (ii): `a ∈ [1, 0x3F800000)`, float term taken but `(int)f == 0` (positive subnormal / `f < 1`) | [x] |
| C4 | `memchra2` | A2 state (iii): `a ∈ [0x3F800000, 0x447A0000)`, float term taken with `(int)f ∈ 1..999` — the only rows where the truncating `(int)f` cast is observable | [x] |
| C5 | `memchra2` | A2 state (iv): `a ≥ 0x447A0000`, float term omitted because `f ≥ 1000` / non-finite | [x] |
| C6 | `memchra2` | A5: `b,c,d` low bytes all `0x00` (`interpret_as_int == 0`, the xor is a no-op) with random high bytes | [x] |
| C7 | `memchra2` | A5: `b,c,d` low bytes all `0xFF` (`interpret_as_int == 0x00FFFFFF`) with random high bytes | [x] |
| C8 | `memchra2` | A5: `b,c,d` low bytes all `0x80` (byte sign bit set; assembled value still positive because the top byte is pinned to 0) | [x] |
| C9 | `memchra2` | A6: `a,b,c,d` share one low byte → `complex_iteration` xor-cancels to `0`; high bytes and signs random | [x] |
| C10 | `memchra2` | A6: pairwise-cancelling low bytes (`a,b` equal and `c,d` equal, but the two pairs differ) → xor `0` via a different path | [x] |
| C11 | `memchra2` | A7: tuples chosen so `a+b+c+d` overflows `INT_MAX` (positive wraparound in `safe_sum_array`) | [x] |
| C12 | `memchra2` | A7: tuples chosen so `a+b+c+d` underflows `INT_MIN` (negative wraparound) | [x] |
| C13 | `memchra2` | A3/A4 longest text: all four args in `[-2147483648, -1000000000]` → 51-byte buffer, `dash_count == 7`, largest `buf_sum` (checks `buf_sum > 0` is not inverted and `snprintf` truncation is not triggered) | [x] |
| C14 | `memchra2` | A3 shortest text: all four args in `-9..9` → 11–15-byte buffer, smallest `buf_sum` | [x] |
| C15 | `memchra2` | A3 uniform 10-digit positive magnitudes `[1000000000, 2147483647]` → `dash_count == 3`, `buf_sum` near its maximum for the no-minus case | [x] |
| C16 | `memchra2` | A1×A2×A5×A6×A7 catch-all: fully uniform random over the whole `i32⁴` domain (10 000 tuples) — the cross-product no constrained row reaches | [x] |
| C17 | `memchra2` | A2 × A1: `a` swept over every float-window boundary (`0, 1, 0x3F7FFFFF, 0x3F800000, 0x4479FFFF, 0x447A0000`) crossed with all 16 sign patterns of `b,c,d` (and `a`'s own sign implied) | [x] |
| C18 | `memchra2` | A9: identical 4-word inputs sent through the `(u32,u32,u32,u32)->u32` FFI signature; raw result words compared (also re-checks the same tuples via the signed signature) | [x] |
| C19 | `memchra2` | A2 state (iii) × A6 cancelling: `a` in the `1 ≤ f < 1000` window *and* low bytes arranged to xor-cancel — interaction row, since the float term and the xor term are the two that a per-helper test would check in isolation | [x] |
| C20 | `memchra2` | A7 overflow × A1 all-negative × A3 longest: interaction of wraparound, 7 dashes, and maximum buffer length simultaneously | [x] |
| C21 | `memchra2` | A3/A4 unreachable-path proof: over 50 004 inputs (4 extremes + 50 000 random), assert the formatted text never exceeds 51 bytes and never contains a byte ≥ 0x80 — this makes the two surviving mutants (`snprintf` cap off-by-one, `char` signedness in `process_buffer`) *provably* equivalent rather than untested | [x] |

## Notes on unreachable states (recorded so the Rust is still checked)

- A4's false branch (`buf_sum <= 0`) is unreachable; C13/C14/C15 confirm the
  Rust also always takes the true branch.
- The 64-byte `snprintf` truncation path is unreachable (max output 51 bytes);
  C13 pins the worst case.
- `process_strings` always returns `3` (`+15`); every row depends on that
  constant, so a divergence there fails all rows.

## Additional coverage beyond the row table

| test | what it adds |
|------|--------------|
| `phase_b_soak::soak_uniform_random` | 1 000 000 uniform random tuples over the whole `i32⁴` domain |
| `phase_b_soak::soak_stratified_cells` | every cell of (16 sign patterns) × (4 float-window states) × (4 low-byte classes) = 256 cells × 40 tuples — guarantees no cell is left to chance by uniform sampling |
| `phase_b_soak::soak_exhaustive_dense_block` | exhaustive `(a,b) ∈ [-60,60]²` × 2 `(c,d)` shapes, plus a dense sweep of `a = bits(k as f32)` for `k ∈ 0..1000` (every integral `(int)f` step in the window) |
| `phase_a_harness::*` | negative control — proves the two `.so`s resolve to distinct addresses in distinct `/proc/self/maps` mappings (so no global-symbol interposition makes the suite vacuous), that the byte comparison can fail, and that both implementations are deterministic |

## Mutation-testing result (`./mutation_check.sh`)

10 deliberate mutations were injected into `src/lib.rs` and the suite re-run.
**8 KILLED, 2 SURVIVED**, and both survivors are provably *equivalent* mutants:

| mutation | outcome | why |
|----------|---------|-----|
| `memchra` target byte `^ 1` | KILLED | |
| `interpret_as_int` LE → BE | KILLED | |
| `safe_sum_array` init `0` → `1` | KILLED | |
| float window `< 1000.0` → `< 100.0` | KILLED | |
| `complex_iteration` mask `0xFF` → `0xFE` | KILLED | |
| `matches * 5` → `* 6` | KILLED | |
| `buf_sum % 256` → `% 255` | KILLED | |
| `dash_count * 10` → `* 11` | KILLED | |
| `snprintf` cap `len-1` → `len-2` | SURVIVED | the formatted text is at most 51 bytes, so no cap ≥ 52 ever binds — proven by `c21_unreachable_paths_are_provably_unreachable` |
| `process_buffer` `as i8` → `as u8` | SURVIVED | every byte of the formatted text is < 0x80 (`'-'`, digits, `"test"`), so signed and unsigned widening coincide — proven by the same test |

Row **C21** exists specifically to turn those two "it can't be reached"
arguments into checked invariants over 50 004 inputs, so the survivors are
documented equivalences rather than untested code.

## Build/profile matrix also verified

The full suite was additionally run with the harness pointed at the **debug**
profile cdylib (`MEMCHRA2_RUST_SO=target/debug/libmemchra2_lib.so`), where
Rust's integer-overflow checks are enabled. All tests pass, confirming no
accumulation in the translation relies on release-mode wrapping to avoid a
panic — every one uses explicit `wrapping_*`.

The C side was cross-checked at `-O0`, `-O1`, `-O2`, `-O3` and `-Os` (plus the
default CMake build) against each other over 200 007 inputs: **all six agree**,
so the C's signed-overflow and type-punning UB is stable and the reference
behaviour is unambiguous.
