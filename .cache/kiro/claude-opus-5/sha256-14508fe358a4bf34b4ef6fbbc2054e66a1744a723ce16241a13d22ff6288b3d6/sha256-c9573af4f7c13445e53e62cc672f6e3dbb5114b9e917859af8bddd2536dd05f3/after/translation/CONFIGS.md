# CONFIGS.md — Phase A configuration-surface table

The mirror of `ERRORS.md`: every **valid** input configuration the C actually
branches on. Derived mechanically from `c_src/src/lib.c` and
`c_src/include/lib.h`.

## Axes the C code actually distinguishes

**A1 — `charinbuf` `mode` (`switch (mode)`, lib.c:100)**: `0`, `1`, `2`, `3`,
`4` are five distinct code paths; everything else is `default`
(that's an error row, see `ERRORS.md` #15).

**A2 — `value` shape, mode 0 only** (feeds `validate_uint16_range`): in range
`[0, 65535]` vs out of range. In-range sub-shapes the `printf`s make visible:
`0`, `1`, `65535` (`%d` of the boundary), mid-range.

**A3 — `value`/`opt1`/`opt2` shape, mode 3 only**: three independent `int`s fed
through `reset → increment → multiply → decrement(5)`. Sub-shapes the arithmetic
distinguishes: zero, positive, negative, `INT_MIN`/`INT_MAX` (wrapping
overflow), `opt2 == 0` (annihilates the counter), `opt2 == -1` (sign flip,
`INT_MIN * -1` overflows).

**A4 — `opt1`/`opt2` are ignored by modes 0, 1, 2, 4**, and `value` is ignored
by modes 1, 2, 4. Passing junk in the ignored slots must not change the output —
a real configuration to check, not an assumption.

**A5 — counter state on entry**: `charinbuf` unconditionally does `counter = 0`
(lib.c:98) *before* the `switch`, so pre-existing counter state must be
discarded; and mode 3 leaves the counter at a computed value that the
standalone mutators must then observe. Two directions to test.

**A6 — entry-point level.** `charinbuf` is the only header-declared
("convenience") entry point; the other nine exports are the low-level API and
must be driven directly:
`increment_counter`, `decrement_counter`, `multiply_counter`, `reset_counter`
(stateful `int → int`), `is_string_empty`, `find_char_in_buffer`,
`create_buffer`, `validate_uint16_range`, `apply_operation`.

**A7 — `is_string_empty` input shape**: non-NULL with `*str != 0`. Sub-shapes:
ASCII, byte `0x01`, high-bit byte `0x80..0xFF` (signed `char`), embedded NUL
after byte 0 (only byte 0 is read).

**A8 — `find_char_in_buffer` input shape**: match at offset 0 / interior /
`size-1`; multiple occurrences (first wins); `target == '\0'` matching an
embedded NUL; non-ASCII/negative `target`; buffer length 1; large buffer;
`size` smaller than / equal to / larger than the first match offset.

**A9 — `create_buffer` input shape**: empty string (`malloc(1)`), 1 byte,
long string, string containing all byte values `0x01..0xFF`. Result must be
`free()`-able and byte-identical, and the returned pointer must be distinct
from the input.

**A10 — `apply_operation` callback identity**: `NULL` (error row) or each of the
four exported mutators. Each library must be driven with **its own** function
pointers, since each `.so` owns a separate `static counter`.

**A11 — stdout vs return value**: every `charinbuf` mode writes to `stdout`
through `printf`/`puts`. Both the return value **and** the exact stdout bytes
are part of the observable contract, so every `charinbuf` row compares both.

There are **no** compile-time options: `c_src/src/lib.c` contains no `#ifdef`
outside its `#include`s, `CMakeLists.txt` sets no `target_compile_definitions`,
and `translation/Cargo.toml` declares no `[features]`. Therefore the only
feature combination is the default one (`cargo test`, plus the explicitly-empty
`--no-default-features` run, which is byte-identical) — see Phase D.

## Rows

Every row is exercised with many randomized inputs (fixed-seed PRNG in
`tests/common/mod.rs`, `ITERS` per row) unless the row is a fixed literal
configuration, in which case the randomization goes into the *ignored* argument
slots (axis A4).

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|--------------------------------------------|------|-----|
| 1  | `charinbuf` | mode 0, `value` random in `[0, 65535]`, `opt1`/`opt2` random junk (ignored) | `cfg_01_mode0_in_range` | [x] |
| 2  | `charinbuf` | mode 0, `value` ∈ {`0`, `1`, `65534`, `65535`} boundary in-range | `cfg_02_mode0_boundaries` | [x] |
| 3  | `charinbuf` | mode 0, `value` random over the whole `int32` range (mixes valid and invalid; checks the `%u` of `UINT16_MAX` too) | `cfg_03_mode0_full_int_range` | [x] |
| 4  | `charinbuf` | mode 1, `value`/`opt1`/`opt2` random junk (all ignored); exercises both `is_string_empty` calls and `result = 0 + 10` | `cfg_04_mode1` | [x] |
| 5  | `charinbuf` | mode 2, random junk args; `create_buffer` + `strlen` + `free`, `%s` and `%zu` output | `cfg_05_mode2` | [x] |
| 6  | `charinbuf` | mode 3, `value`/`opt1`/`opt2` random over full `int32` (wrapping `reset/inc/mul/dec` chain) | `cfg_06_mode3_full_range` | [x] |
| 7  | `charinbuf` | mode 3, structured shapes: `opt2 == 0`; `opt2 == -1`; `value == INT_MIN`; `value == INT_MAX`; `opt1 == INT_MAX`; all-zero; all combinations of {`INT_MIN`,`-1`,`0`,`1`,`INT_MAX`}³ | `cfg_07_mode3_overflow_grid` | [x] |
| 8  | `charinbuf` | mode 4, random junk args; `create_buffer` + `memchr` hit at a known offset, `%c`/`%s`/`%d` output | `cfg_08_mode4` | [x] |
| 9  | `charinbuf` | every mode `0..4` in a random *sequence*, so each call sees leftover counter/heap state from the previous one (axis A5, forward direction) | `cfg_09_mode_sequence` | [x] |
| 10 | `reset_counter` + `charinbuf(3,…)` + `increment_counter` | counter pre-seeded to a random value, then mode 3, then read back via the low-level mutators (axis A5, both directions) | `cfg_10_counter_state_across_charinbuf` | [x] |
| 11 | `increment_counter` | standalone, random `int32` values applied repeatedly to accumulating state | `cfg_11_increment` | [x] |
| 12 | `decrement_counter` | standalone, random `int32` values applied repeatedly | `cfg_12_decrement` | [x] |
| 13 | `multiply_counter` | standalone, random `int32` values incl. `0`, `-1`, `INT_MIN` | `cfg_13_multiply` | [x] |
| 14 | `reset_counter` | standalone, random `int32` values incl. boundaries | `cfg_14_reset` | [x] |
| 15 | all four mutators | random *interleaving* of the four ops (random op, random operand) over a long chain — the composed low-level pipeline, not one wrapper at a time | `cfg_15_mutator_random_pipeline` | [x] |
| 16 | `validate_uint16_range` | random `int32` over the full range plus the exhaustive boundary set `{INT_MIN, -2, -1, 0, 1, 2, 65534, 65535, 65536, 65537, INT_MAX}` | `cfg_16_validate_full_range` | [x] |
| 17 | `is_string_empty` | non-empty, first byte random `0x01..0xFF` (incl. high-bit/signed) | `cfg_17_is_string_empty_first_byte` | [x] |
| 18 | `is_string_empty` | random-length random-content strings; empty; embedded NUL at byte 0 vs later | `cfg_18_is_string_empty_shapes` | [x] |
| 19 | `find_char_in_buffer` | random buffer, `target` present, match at offset 0 / interior / `size-1`, `size` == buffer length | `cfg_19_find_char_hit_positions` | [x] |
| 20 | `find_char_in_buffer` | random buffer + random `target` (may or may not be present) + random `size ≤ len`; compares the *offset* of the returned pointer, and that it points into the caller's buffer | `cfg_20_find_char_random` | [x] |
| 21 | `find_char_in_buffer` | multiple occurrences of `target` (first-match semantics); `target == '\0'` with an embedded NUL; buffer of length 1 | `cfg_21_find_char_multi_and_nul` | [x] |
| 22 | `create_buffer` | empty string, 1-byte, random-length random-content strings, and a string of every byte `0x01..0xFF`; compares returned bytes, `strlen`, non-NULL, `!= input`, and that `free()` succeeds | `cfg_22_create_buffer_shapes` | [x] |
| 23 | `apply_operation` | each of the four mutators (that library's own pointer) with random `int32` operands, from a random starting counter | `cfg_23_apply_operation_each_op` | [x] |
| 24 | `apply_operation` | random *sequence* of (random mutator, random operand) pairs — composed pipeline through the indirect-call entry point | `cfg_24_apply_operation_pipeline` | [x] |
| 25 | `create_buffer` → `find_char_in_buffer` | output of one low-level entry point fed straight into the next (the pipeline `charinbuf` mode 4 builds internally, driven manually with random content and random targets) | `cfg_25_create_then_find_pipeline` | [x] |
| 26 | `apply_operation` + foreign callback | `apply_operation` is a pure indirect call, so a caller may hand it *any* `int (*)(int)`: C's `apply_operation` is driven with the Rust `.so`'s mutator pointers and vice versa, for all four ops with random operands and random start values | `cfg_26_apply_operation_foreign_callback` | [x] |
| 27 | `charinbuf` + counter mutators | the counter is perturbed to a random value *between* two identical `charinbuf` calls; because `charinbuf` zeroes it on entry, return value and stdout must be identical both times (argument-determinism of the convenience entry point across all modes incl. `default`) | `cfg_27_charinbuf_is_argument_determined` | [x] |
