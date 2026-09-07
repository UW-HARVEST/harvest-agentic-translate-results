# CONFIGS.md — Phase B configuration-surface table

## How this table was derived

The public surface is enumerated from the header plus the actual exported
symbols (see `SYMBOLS.md`), not from the convenience wrapper alone:

- **Low-level entry point:** `int foo(const char *in, char c)` —
  `c_src/src/driver.c:29`. Not in `driver.h`, but exported (`nm -D` → `T foo`),
  so it is a public entry point and is driven **directly**.
- **Convenience / one-shot wrapper:** `void driver(const char *in)` —
  `c_src/src/driver.c:37`. Hard-codes two `foo` calls (`'A'`, then `'x'`) and
  formats each result with `printf("%s: %d\n")` to stdout.

There are **no runtime options, modes or flags**: no setter, no config struct,
no global state, no `#ifdef` in the source (grep for `#if`/`switch`/`if (`
returns 0 hits — see `ERRORS.md`). The branch axes the C code actually
distinguishes are therefore purely **input shape**:

1. **needle value `c`** (only variable for `foo`; fixed to `'A'`/`'x'` for
   `driver`): absent from input / present / `'\0'` / high-bit (negative
   `signed char`) / equal to the first byte / equal to the last byte.
2. **occurrence count**: zero, exactly one, many, *all* bytes matching.
3. **occurrence position** — this is what the `s++`-after-match walk branches
   on: at index 0, in the middle, at the final byte before NUL, and consecutive
   runs (adjacent matches, where `s++` lands on another match).
4. **input length**: empty (0), 1 byte, small, large (1 MiB), i.e.
   empty / one / many.
5. **byte content**: printable ASCII, full 0x01–0xFF alphabet including
   high-bit bytes, and inputs whose bytes are *not* valid UTF-8 (the Rust side
   must stay byte-oriented and never assume UTF-8).
6. **stdout formatting path** (`driver` only): the `%d` conversion for counts of
   0, single digits, and multi-digit values, and the ordering/interleaving of
   the two `printf` calls in one stream.

Rows are the pruned cross-product of the axes the code treats differently.
Every row is exercised with **many randomized inputs** from a fixed-seed
xorshift PRNG (seed `0x243F6A8885A308D3`) — 200+ cases per randomized row — and
both `.so`s are called through `libloading`, never by direct Rust linkage.

## Table

| # | entry point(s) | configuration (options set + input shape) | test | [ ] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `foo` | empty input `""`, needle swept over all 256 byte values (needle `0` excluded → row 12) | `cfg_row1_empty_input_all_needles` | [x] |
| 2 | `foo` | 1-byte input, needle swept over all 256 values × payload byte swept over all 255 non-NUL values (full 255×256 matrix) | `cfg_row2_single_byte_matrix` | [x] |
| 3 | `foo` | needle absent from input (zero occurrences), randomized ASCII inputs of random length | `cfg_row3_zero_occurrences_random` | [x] |
| 4 | `foo` | exactly one occurrence, at a randomized position | `cfg_row4_exactly_one_occurrence_random` | [x] |
| 5 | `foo` | many scattered occurrences, randomized inputs over a small alphabet (high match density) | `cfg_row5_many_occurrences_random` | [x] |
| 6 | `foo` | consecutive/adjacent occurrences (runs of the needle), so post-match `s++` lands on another match | `cfg_row6_consecutive_runs_random` | [x] |
| 7 | `foo` | occurrence at index 0 (first byte) | `cfg_row7_match_at_first_byte_random` | [x] |
| 8 | `foo` | occurrence at the last byte before the NUL | `cfg_row8_match_at_last_byte_random` | [x] |
| 9 | `foo` | every byte of the input equals the needle (saturated input, count == length) | `cfg_row9_all_bytes_match_random` | [x] |
| 10 | `foo` | randomized inputs over the **full** 0x01–0xFF byte alphabet (non-UTF-8, high-bit bytes) × randomized non-zero needle | `cfg_row10_full_byte_alphabet_random` | [x] |
| 11 | `foo` | high-bit needle (`0x80`–`0xFF`, negative as `signed char`) against inputs containing both that byte and other high-bit bytes | `cfg_row11_high_bit_needle_random` | [x] |
| 12 | `foo` | needle `c == '\0'`. `strchr(s, 0)` always matches a terminator and so never yields the loop's NULL exit sentinel: the pointer walks out of the object forever. Driven over a deterministic zero-padded arena and out-of-process (`alarm()`-bounded), comparing the termination signal rather than a return value — the call never returns in either library | `cfg_row12_nul_needle_padded_arena` | [x] |
| 13 | `foo` | large input (1 MiB) with randomized needle density | `cfg_row13_large_input_random` | [x] |
| 14 | `driver` | full end-to-end pipeline, stdout captured and compared byte-for-byte: input with neither `'A'` nor `'x'` (both counts `0`) | `cfg_row14_driver_stdout_no_matches` | [x] |
| 15 | `driver` | stdout compared: `'A'` present, `'x'` absent (asymmetric — catches a swapped needle/label) | `cfg_row15_driver_stdout_only_A` | [x] |
| 16 | `driver` | stdout compared: `'x'` present, `'A'` absent (the mirror of row 15) | `cfg_row16_driver_stdout_only_x` | [x] |
| 17 | `driver` | stdout compared: both present with **different** multi-digit counts (exercises `%d` width and the two-line ordering) | `cfg_row17_driver_stdout_multi_digit` | [x] |
| 18 | `driver` | stdout compared: empty input `""` (both counts `0`) | `cfg_row18_driver_stdout_empty` | [x] |
| 19 | `driver` | stdout compared over randomized inputs from the full byte alphabet, including `'A'`/`'x'` at boundary positions and adjacent runs | `cfg_row19_driver_stdout_random` | [x] |
| 20 | `foo` + `driver` composed | the composed pipeline: for the same randomized input, assert `driver`'s two printed numbers equal the directly-called `foo(in,'A')` / `foo(in,'x')` values, cross-checked between the two libraries (catches a wrapper that calls the right function with the wrong argument) | `cfg_row20_composed_consistency_random` | [x] |

No binary executable is built (`CMakeLists.txt` has no `add_executable`), so the
"compare C and Rust binary stdout" clause is satisfied instead by rows 14–19,
which compare the stdout each `.so` writes to a real captured fd.

## Randomization

Rows marked `_random` draw from `Rng` (xorshift64\*) seeded from
`SEED = 0x243F6A8885A308D3` XOR a per-row constant, so every row is
independently reproducible and no two rows draw the same sequence. Iteration
counts: 300–600 per randomized row; rows 1, 2, 11 are *exhaustive* over the
byte/needle space instead (row 2 alone is 255 × 255 = 65 025 differential
calls).

## Result

All 20 rows pass, in the debug profile, the release profile, and release with
`-Cdebug-assertions=on` forced. Run with `./run_all_combos.sh`.
