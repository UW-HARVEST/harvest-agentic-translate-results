# CONFIGS.md — Phase B configuration-surface table

## How the axes were derived

Mechanically, from the C source and public header — not from guesses:

```
grep -n "enum\|typedef\|struct\|#ifdef\|#if \|switch" include/driver.h src/driver.c
    -> NONE
grep -n "if *(" src/driver.c
    -> 31 (line != NULL), 46 (data >= 0), 66 (data >= 0), 85 (data >= 0 && data < (10))
nm -D --defined-only build/libdriver.so
    -> printLine, printIntLine, bad, good, driver
```

**Runtime option/mode/flag axis: EMPTY.** The library has no setters, no global
state, no context/handle struct, no environment variables, no `#ifdef`, and no
`switch`. `driver.h` exposes a single function and no types. There is nothing to
configure, so the configuration surface is entirely the **input-shape** axis
crossed with the **entry-point** axis. Every branch the C takes is decided
solely by the argument values listed below.

**Entry-point axis — the FULL set of 5 exported symbols, lowest level first.**
The tests drive the low-level primitives (`printLine`, `printIntLine`) directly,
then the mid-level `bad`/`good`, then the composed one-shot wrapper `driver` —
`driver` alone would hide per-primitive divergences, and the primitives alone
would hide pipeline/ordering divergences.

Call hierarchy (from the source):

```
driver(goodData, badData)
  ├─ printLine("Calling good()...")
  ├─ good(goodData)
  │    ├─ goodG2B()            [static; data hardcoded 7]  -> printIntLine ×10
  │    └─ goodB2G(goodData)    [static; guard 0 <= d < 10] -> printIntLine ×10 | printLine(err)
  ├─ printLine("Finished good()")
  ├─ printLine("Calling bad()...")
  ├─ bad(badData)              [guard d >= 0 ONLY]         -> printIntLine ×10 | printLine(err)
  └─ printLine("Finished bad()")
```

**Input-shape axis, per parameter type:**

* `const char *line` (`printLine`): NULL · empty · 1 byte · plain ASCII ·
  embedded `%` format specifiers · embedded newline/tab · embedded high /
  non-UTF-8 bytes · very long (100 000 bytes) · randomized byte strings.
* `int intNumber` (`printIntLine`): `0` · `1` · `-1` · `INT_MAX` · `INT_MIN` ·
  every ±power-of-two bit position · randomized `i32`.
* `int data` (`bad`, `good`, and both `driver` params): the guards partition the
  `int` line into `[INT_MIN, 0)` · `0` · `(0, 9]` (each of the 10 in-bounds
  indices is a distinct printed pattern, so all 10 are enumerated) · `10`
  (first out-of-bounds) · `(10, INT_MAX]`.

Randomization: fixed seed (`0xD1234567`, SplitMix64) so runs are reproducible;
each row asserts across many generated inputs, not one hand-picked value.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `printLine` (lowest level) | valid non-empty ASCII string | `cfg_01_print_line_ascii` | [x] |
| 2 | `printLine` | empty string `""` (zero length, valid) — emits just `\n` | `cfg_02_print_line_empty` | [x] |
| 3 | `printLine` | single-byte string; every byte value `0x01..0xFF` | `cfg_03_print_line_every_single_byte` | [x] |
| 4 | `printLine` | string containing `%s %d %n %%` — format-specifier bytes as *data* | `cfg_04_print_line_percent_data` | [x] |
| 5 | `printLine` | embedded `\n`, `\t`, `\r` (multi-line payload) | `cfg_05_print_line_embedded_ws` | [x] |
| 6 | `printLine` | non-UTF-8 / high bytes (`0x80..0xFF` mixed) | `cfg_06_print_line_non_utf8` | [x] |
| 7 | `printLine` | very long string (100 000 bytes) — crosses stdout buffer boundaries | `cfg_07_print_line_huge` | [x] |
| 8 | `printLine` | 512 randomized byte strings, random lengths `0..=256` | `cfg_08_print_line_random` | [x] |
| 9 | `printIntLine` (lowest level) | `0` | `cfg_09_print_int_line_zero` | [x] |
| 10 | `printIntLine` | `INT_MAX`, `INT_MIN`, `-1`, `1` (extremes + signs) | `cfg_10_print_int_line_extremes` | [x] |
| 11 | `printIntLine` | every ±power of two (all 32 bit positions) | `cfg_11_print_int_line_bit_positions` | [x] |
| 12 | `printIntLine` | 4096 randomized `i32` values | `cfg_12_print_int_line_random` | [x] |
| 13 | `printIntLine` | many calls in sequence — accumulated stream / buffering | `cfg_13_print_int_line_sequence` | [x] |
| 14 | `bad` (mid level) | `data == 0` — writes the **first** in-bounds slot | `cfg_14_bad_index_zero` | [x] |
| 15 | `bad` | `data` = each of `1..=8` — a different printed pattern per index | `cfg_15_bad_interior_indices` | [x] |
| 16 | `bad` | `data == 9` — the **last** in-bounds slot (upper boundary) | `cfg_16_bad_index_nine` | [x] |
| 17 | `bad` | all in-bounds `0..=9` in one loop, checking the `1` moves position | `cfg_17_bad_all_in_bounds_sweep` | [x] |
| 18 | `bad` | repeated calls with the same in-bounds `data` — buffer must be re-zeroed each call (`int buffer[10] = {0}` is a fresh local, not `static`) | `cfg_18_bad_repeated_calls_reinitialize` | [x] |
| 19 | `good` (mid level) | `data == 0` — `goodG2B` (fixed 7) then `goodB2G(0)`; both sections print | `cfg_19_good_index_zero` | [x] |
| 20 | `good` | `data == 7` — `goodB2G`'s index coincides with `goodG2B`'s hardcoded 7, so both halves print the identical 10-line pattern | `cfg_20_good_index_seven_same_as_g2b` | [x] |
| 21 | `good` | `data == 9` — last valid index for `goodB2G` | `cfg_21_good_index_nine` | [x] |
| 22 | `good` | all valid `0..=9` swept; verifies `goodG2B`'s fixed-7 half never varies while the `goodB2G` half does | `cfg_22_good_all_in_bounds_sweep` | [x] |
| 23 | `good` | repeated calls, same `data` — both static helpers re-zero their locals | `cfg_23_good_repeated_calls` | [x] |
| 24 | `driver` (composed wrapper) | `goodData` and `badData` both valid and **equal** | `cfg_24_driver_both_valid_equal` | [x] |
| 25 | `driver` | `goodData != badData`, both valid — proves the two args are not swapped or aliased in the pipeline | `cfg_25_driver_valid_distinct_args` | [x] |
| 26 | `driver` | **full cross product** of `goodData × badData` over `0..=9` (100 combinations) — the option-interaction row | `cfg_26_driver_full_cross_product_valid` | [x] |
| 27 | `driver` | cross product of `goodData × badData` over `{-1, 0, 5, 9, 10, INT_MIN, INT_MAX}` — mixes valid and invalid on both axes (49 combinations); interleaves the four fixed banner lines with both variable sections | `cfg_27_driver_cross_product_mixed` | [x] |
| 28 | `driver` | 1024 randomized `(goodData, badData)` pairs drawn from a mixture of in-range, near-boundary, and full-`i32` distributions | `cfg_28_driver_random_pairs` | [x] |
| 29 | all 5 entry points | **interleaved** in one randomized script (512 steps) against a single shared `stdout` — catches ordering, hidden state, and buffer-flush divergences invisible to per-function tests | `cfg_29_interleaved_all_entry_points` | [x] |
| 30 | `bad` | `data >= 10` (`10`, `11`, `16`, `31`, `64`) — the out-of-bounds-write shape. Documented as a *shape* the C special-cases by *omission*; behaviour is C-undefined, so this row asserts on the observable stdout the C compiler actually produces. See the UB note below. | `cfg_30_bad_oob_write_shape` | [x] |

## Note on row 30 / `ERRORS.md` #5 (the CWE-787 out-of-bounds write)

`bad()` guards only `data >= 0`, so `buffer[data] = 1` for `data >= 10` writes
past a 10-element stack array — undefined behaviour in C. This is the injected
defect the test case exists to demonstrate, and per the task rules it is
**reproduced, not fixed**.

How far the overrun can be *compared* is bounded by the C's own frame layout,
which `objdump -d c_src/build/libdriver.so` gives exactly:

```text
bad:  push %rbp ; mov %rsp,%rbp ; sub $0x40,%rsp
      buffer -> -0x30(%rbp) .. -0x08(%rbp)     (10 ints)
      i      -> -0x04(%rbp)
      data   -> -0x34(%rbp)
      movl $0x1,-0x30(%rbp,%rax,4)             <- buffer[data] = 1, unchecked
      movl $0x0,-0x04(%rbp)                    <- i = 0, immediately after
```

* `buffer[10]` → `-0x08(%rbp)`: frame padding, never read. Harmless.
* `buffer[11]` → `-0x04(%rbp)`: the loop counter `i` — but the *next* instruction
  is `i = 0`, so the write is overwritten before use. Harmless.
* `buffer[12]`, `buffer[13]` → the saved `%rbp` at `0x0(%rbp)`.
* `buffer[14]`, `buffer[15]` → the return address at `0x8(%rbp)`.
* `buffer[16]`+ → the **caller's** frame.

Measured, calling the C `.so` with each `data` in a forked child so a crash is
contained: the C takes **SIGSEGV for `data` in `12..=15`**, and again for
`20..=23`; the values in between only "survive" by accident of what the caller
happens to store at that offset — i.e. the outcome depends on the *caller*, not
on the library.

Row 30 therefore compares `data ∈ {10, 11}` (the constant `BAD_OOB_INFRAME` in
`tests/common/mod.rs`), where the C's damage stays inside its own frame and its
stdout is well defined: ten `0` lines, because the `1` landed outside the ten
elements that get printed. The Rust translation backs its 10-element view with a
1024-element zeroed array (`BUFFER_SLACK`), so the same two overruns likewise
land in slack and print ten `0`s — byte-identical.

`data >= 12` is deliberately **excluded from differential comparison and stated
here rather than silently skipped**: no translation can reproduce corruption of
an arbitrary caller's stack frame, and the divergence would be a property of the
C's UB and the calling convention, not of the translation. Row 30 additionally
interleaves each overrun with an in-bounds call and asserts the in-bounds result
is still correct, so a *lingering* effect of the overrun would still be caught.

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so `default`,
`--no-default-features`, and any `--features <combo>` all denote the same single
build configuration. Every row above is therefore verified under *every*
feature combination that exists. Confirmed mechanically by
`scripts/verify_all.sh`, which enumerates the features declared in `Cargo.toml`,
forms their power set (falling back to `default` + `--no-default-features` when
none are declared), and for each combination rebuilds, diffs `nm -D` against the
C `.so`, and re-runs the Phase B / C / D suites. It also asserts that all
combinations export an identical symbol set.

## Not applicable

* **Binary / driver executable comparison.** `c_src/CMakeLists.txt` contains only
  `add_library(driver SHARED src/driver.c)` — no `add_executable` — and
  `translation/Cargo.toml` declares only `[lib]` with `crate-type = ["cdylib"]`,
  with no `[[bin]]` and no `src/main.rs`. There is no executable on either side,
  so the "compare C and Rust binary stdout" gate is vacuous. Asserted
  mechanically by `sym_05_project_defines_no_binary_target`, which fails if a
  binary target is ever added.
* **Runtime options / modes / flags.** None exist (see "How the axes were
  derived").

