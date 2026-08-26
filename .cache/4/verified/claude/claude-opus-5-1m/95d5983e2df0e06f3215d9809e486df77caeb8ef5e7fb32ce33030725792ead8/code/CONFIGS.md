# CONFIGS.md — configuration-surface table (valid inputs)

Derived mechanically from `c_src/src/main.c` by enumerating every axis the C
code branches on.

## Build-time configuration axes

| source | axes found |
|--------|-----------|
| `Cargo.toml` | **no `[features]` section at all** → exactly one feature combination: the empty/default one. `cargo check --no-default-features` and `cargo check` are the same configuration. |
| `c_src/CMakeLists.txt` | `cmake_minimum_required(3.10)`, `project(driver)`, `add_executable(driver src/main.c)`. No `option()`, no `target_compile_definitions`, no `#ifdef`/`#if` anywhere in `main.c` (verified by grep). → exactly one build configuration. |

So Phases B and C have to be run for **one** combination, but that combination
must cover all of the *runtime* axes below.

## Runtime axes the C code actually branches on

| axis | values the C distinguishes |
|------|---------------------------|
| A. `operation` read from stdin / `op` argument | `0` OP_COPY, `1` OP_REVERSE, `2` OP_MERGE, `3` OP_SPLIT, `4` OP_INTERLEAVE, `5` OP_ROTATE, `6` OP_CHECKSUM, plus "not an enumerator" (`default:`) |
| B. entry point level | low level: `calculate_checksum`, `validate_buffer`, `init_buffer_array`, `free_buffer_array`, `buffer_copy`, `buffer_reverse`, `buffer_merge`, `buffer_split`, `buffer_interleave`, `buffer_rotate`, `buffer_conditional_copy`, `buffer_copy_strided`; mid level: `process_buffer_array`; I/O level: `read_buffer`, `write_buffer`; top level: `main` |
| C. `buffer_t.length` shape | `0`, `1`, `2` (even), `3` (odd), small, `255`, `256` (maximum) |
| D. buffer-pair length relation (merge/interleave) | `l1 == l2`, `l1 < l2`, `l1 > l2`, one side `0`, both `0`, `l1+l2 == 256` (exactly at the limit) |
| E. `split_pos` | `0`, `1`, interior, `length-1`, `length` (== boundary) |
| F. `positions` (rotate) | `0` (early return), `1`, interior, `length` (≡0 after `%`), `> length`, negative, `-length`, `< -length`, `INT_MIN`, `INT_MAX` |
| G. `stride` | `1` (copies everything), `2`, `3`, `length`, `> length`, `INT_MAX` |
| H. `pattern` / `copy_matching` (conditional copy) | pattern present / absent / all bytes match; `copy_matching` `false` (0) and `true` (1) |
| I. `buffer_array_t.count` vs `capacity` | `count == 1`, `2` (even), `3` (odd), `capacity` up to 100; `count < capacity`; negative `count` |
| J. `checksum` field state | consistent with `data` (no warning) vs deliberately corrupted (warning path) |
| K. stdin token formatting (`scanf("%d")`) | spaces / tabs / newlines / `\v` / `\f` / `\r` separators, leading `+`, leading zeros, values that overflow `long` (glibc saturates at `LONG_MAX`/`LONG_MIN` then truncates to `int`), values that overflow `int` but not `long`, byte values outside 0..255 (truncated by `(uint8_t)`) |
| L. `buffer_count` (main) | `1`, `2`, `3`, `100` (maximum) |

`main` never reads a `split_pos`/`positions` token unless `operation` is 3 or 5,
and only reads `buffer_count` buffers — the token stream shape is itself an axis
(axis K) and is fuzzed.

## Combination table

One row per combination the C treats differently. Every row is driven with many
randomized inputs (fixed seed) through **both** `.so`s and compared byte for
byte: return value, the full `buffer_t` out-parameters (`data[0..length]`,
`length`, `checksum`), stdout and stderr.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `calculate_checksum` | `length = 0` (NULL and non-NULL data) | [x] |
| 2 | `calculate_checksum` | `length = 1` | [x] |
| 3 | `calculate_checksum` | `length` 2..255, random bytes (checks the `sum << 3` overflow/rotation behaviour) | [x] |
| 4 | `calculate_checksum` | `length = 256` (maximum) | [x] |
| 5 | `calculate_checksum` | data with all-`0x00` / all-`0xFF` / high-bit-set bytes | [x] |
| 6 | `validate_buffer` | `length = 0`, checksum consistent | [x] |
| 7 | `validate_buffer` | `length` 1..256, checksum consistent (no output, returns true) | [x] |
| 8 | `validate_buffer` | `length` 1..256, checksum corrupted → warning to stderr, still true (axis J) | [x] |
| 9 | `init_buffer_array` + `free_buffer_array` | `capacity = 1`, `2`, `3`, `100`, `1000` → check `count = 0`, `capacity`, non-NULL, then free | [x] |
| 10 | `buffer_copy` | `src.length = 0` (dst tail must be left untouched) | [x] |
| 11 | `buffer_copy` | `src.length` 1..255 random, dst pre-filled with a distinct pattern | [x] |
| 12 | `buffer_copy` | `src.length = 256` (maximum) | [x] |
| 13 | `buffer_copy` | `src.checksum` inconsistent → warning + still copies | [x] |
| 14 | `buffer_copy` | `src == dst` (aliased) | [x] |
| 15 | `buffer_reverse` | `length = 0` → early return, checksum NOT recomputed | [x] |
| 16 | `buffer_reverse` | `length = 1` | [x] |
| 17 | `buffer_reverse` | `length` even (2, 4, …, 256) | [x] |
| 18 | `buffer_reverse` | `length` odd (3, 5, …, 255) | [x] |
| 19 | `buffer_reverse` | applied twice (identity) — checks tail bytes are untouched | [x] |
| 20 | `buffer_merge` | `l1 = 0, l2 = 0` | [x] |
| 21 | `buffer_merge` | `l1 = 0, l2 > 0` | [x] |
| 22 | `buffer_merge` | `l1 > 0, l2 = 0` | [x] |
| 23 | `buffer_merge` | `l1 == l2`, random | [x] |
| 24 | `buffer_merge` | `l1 < l2` and `l1 > l2`, random | [x] |
| 25 | `buffer_merge` | `l1 + l2 == 256` exactly (boundary accepted) | [x] |
| 26 | `buffer_merge` | `dst` pre-filled, so the untouched tail is compared too | [x] |
| 27 | `buffer_merge` | `src1 == src2` (aliased sources) | [x] |
| 28 | `buffer_split` | `split_pos = 0` (dst1 empty, dst2 = whole) | [x] |
| 29 | `buffer_split` | `split_pos = length` (dst2 empty) | [x] |
| 30 | `buffer_split` | `split_pos` interior, random `length` 1..256 | [x] |
| 31 | `buffer_split` | `length = 0`, `split_pos = 0` (both empty) | [x] |
| 32 | `buffer_split` | `dst1`/`dst2` pre-filled with distinct patterns (tail preservation) | [x] |
| 33 | `buffer_split` | `dst1 == dst2` (aliased destinations) | [x] |
| 34 | `buffer_interleave` | `l1 == l2` (perfect alternation) | [x] |
| 35 | `buffer_interleave` | `l1 > l2` (tail of src1 appended) | [x] |
| 36 | `buffer_interleave` | `l1 < l2` (tail of src2 appended) | [x] |
| 37 | `buffer_interleave` | one side `0`, other side `> 0` (both orders) | [x] |
| 38 | `buffer_interleave` | both `0` | [x] |
| 39 | `buffer_interleave` | `l1 + l2 == 256` exactly | [x] |
| 40 | `buffer_interleave` | `src1 == src2` (aliased) | [x] |
| 41 | `buffer_rotate` | `positions = 0` → early return (checksum untouched) | [x] |
| 42 | `buffer_rotate` | `length = 0` → early return | [x] |
| 43 | `buffer_rotate` | `0 < positions < length`, random `length` 1..256 | [x] |
| 44 | `buffer_rotate` | `positions == length` (normalises to 0 but does NOT early-return) | [x] |
| 45 | `buffer_rotate` | `positions > length` (needs `%`) | [x] |
| 46 | `buffer_rotate` | `positions` negative, `|positions| < length` | [x] |
| 47 | `buffer_rotate` | `positions == -length` and `positions < -length` | [x] |
| 48 | `buffer_rotate` | `positions = INT_MAX`, `INT_MIN` (the `size_t` round-trip in the C) | [x] |
| 49 | `buffer_rotate` | `length = 1` with every kind of `positions` | [x] |
| 50 | `buffer_conditional_copy` | `copy_matching = 1` (true), pattern present several times | [x] |
| 51 | `buffer_conditional_copy` | `copy_matching = 0` (false), pattern present several times | [x] |
| 52 | `buffer_conditional_copy` | pattern absent, both `copy_matching` values (→ empty / full copy) | [x] |
| 53 | `buffer_conditional_copy` | all bytes equal to pattern, both `copy_matching` values | [x] |
| 54 | `buffer_conditional_copy` | `src.length = 0`; and `src.length = 256` | [x] |
| 55 | `buffer_conditional_copy` | `src == dst` (aliased, in-place filter) | [x] |
| 56 | `buffer_copy_strided` | `stride = 1` (full copy) | [x] |
| 57 | `buffer_copy_strided` | `stride = 2` and `3`, `length` random 1..256 | [x] |
| 58 | `buffer_copy_strided` | `stride == length` and `stride > length` (single byte out) | [x] |
| 59 | `buffer_copy_strided` | `stride = INT_MAX` (one byte, no overflow of `i`) | [x] |
| 60 | `buffer_copy_strided` | `src.length = 0` (loop body never runs) | [x] |
| 61 | `buffer_copy_strided` | `src == dst` (aliased) | [x] |
| 62 | `process_buffer_array` | `op = OP_COPY`, `count = 1` (loop body never runs), `2`, `3`, `10` | [x] |
| 63 | `process_buffer_array` | `op = OP_COPY`, `buffers[0].checksum` corrupted → warning per copy | [x] |
| 64 | `process_buffer_array` | `op = OP_REVERSE`, `count` 1..10, mixed lengths incl. 0 | [x] |
| 65 | `process_buffer_array` | `op = OP_MERGE`, `count` even (pairs consumed exactly) | [x] |
| 66 | `process_buffer_array` | `op = OP_MERGE`, `count` odd (last buffer untouched) | [x] |
| 67 | `process_buffer_array` | `op = OP_ROTATE`, `param` positive / negative / 0 / `INT_MIN`, mixed lengths | [x] |
| 68 | `process_buffer_array` | `op = OP_CHECKSUM`, all consistent (returns 0, no output) | [x] |
| 69 | `process_buffer_array` | `op = OP_CHECKSUM`, some corrupted (warnings, still returns 0) | [x] |
| 70 | `process_buffer_array` | `count` negative with each of OP_COPY/REVERSE/ROTATE/CHECKSUM → loops never run, returns 0 | [x] |
| 71 | `process_buffer_array` | array allocated by the *other* library's `init_buffer_array` (cross-checks the malloc'd layout) | [x] |
| 72 | `read_buffer` | `length = 0` (no byte tokens follow) | [x] |
| 73 | `read_buffer` | `length` 1..256 random values, space separated | [x] |
| 74 | `read_buffer` | `length = 256` (maximum) | [x] |
| 75 | `read_buffer` | byte tokens outside 0..255 (negative, > 255) → `(uint8_t)` truncation | [x] |
| 76 | `read_buffer` | separators: tabs, newlines, `\r`, `\v`, `\f`, runs of whitespace, leading `+`, leading zeros | [x] |
| 77 | `read_buffer` | called repeatedly on one stream (state carried between calls) | [x] |
| 78 | `write_buffer` | `length = 0` → prints just `0\n` | [x] |
| 79 | `write_buffer` | `length` 1..256, byte values 0, 1, 9, 10, 99, 100, 255 (decimal width changes) | [x] |
| 80 | `main` | `operation = 0` OP_COPY, `buffer_count >= 2`, first buffer length 0/1/small/256 | [x] |
| 81 | `main` | `operation = 1` OP_REVERSE, `buffer_count` 1..100, mixed lengths (writes one line per buffer) | [x] |
| 82 | `main` | `operation = 2` OP_MERGE, `buffer_count >= 2`, `l1+l2` 0..256 | [x] |
| 83 | `main` | `operation = 3` OP_SPLIT, `split_pos` 0 / interior / `length` | [x] |
| 84 | `main` | `operation = 4` OP_INTERLEAVE, all length relations | [x] |
| 85 | `main` | `operation = 5` OP_ROTATE, `positions` positive / negative / 0 / huge, `buffer_count` > 1 | [x] |
| 86 | `main` | `operation = 6` OP_CHECKSUM, `buffer_count` 1..100 | [x] |
| 87 | `main` | `buffer_count = 100` (maximum) with `operation` 1 and 6 → large stdout, crosses glibc's 4096-byte stdout buffer | [x] |
| 88 | `main` | extra trailing tokens after the last one consumed (must be ignored) | [x] |
| 89 | `main` | token stream with every whitespace variant and `+`/leading-zero forms (axis K) | [x] |
| 90 | `main` | `operation` given as a value that overflows `long`/`int` in `scanf` | [x] |
| 91 | `main` (both `.so`s, `dlopen`ed and called through the exported `main` symbol) | randomized full-program fuzz, 4000+ generated stdin streams | [x] |

## Traceability: row → test

Row *N* in the table above is verified by the test function named `rowNN_...`.
The tests live in:

| rows | test file | driven through |
|------|-----------|----------------|
| 1–61 (+ `alias_*`) | `tests/diff_lowlevel.rs` | the 12 low-level exported symbols, called directly via `libloading` |
| 62–71 | `tests/diff_process.rs` | `init_buffer_array` → `process_buffer_array` → `free_buffer_array`, i.e. the composed pipeline |
| 72–79 | `tests/diff_io.rs` | `read_buffer` / `write_buffer` with fd 0/1 redirected |
| 80–91 | `tests/so_main_diff.rs` | the exported `main` symbol of each `.so` **and** the two executables |

`tests/diff_lowlevel.rs` additionally contains five `alias_*` tests that cover
the fully-aliased argument forms (`buffer_split(p,pos,p,p)`,
`buffer_interleave(p,p,p)`, `buffer_interleave` with `dst` equal to either
source, `buffer_merge` with `dst == src1`, and `buffer_copy(p,p)` with an
inconsistent checksum) — everything that is still well defined in C.

Each row is driven with many randomized inputs from a fixed-seed SplitMix64
generator (`tests/common/mod.rs::Rng`), plus hand-picked boundary values, and
compares:

* the return value,
* every byte written to stdout,
* every byte written to stderr, and
* the full 256-byte `data` array, `length` and `checksum` of every `buffer_t`
  in the working set (so an implementation that touched bytes past `length`
  would be caught).

The only place where the comparison is narrowed to the *defined* bytes is
`process_buffer_array` with `OP_MERGE` (rows 65, 66): the C original copies a
whole `buffer_t` out of an **uninitialized** stack object, so every byte past
`merged.length` is indeterminate in C by construction.

## How to run every configuration

```sh
./run_all_configs.sh          # enumerates the feature power set and runs all of it
cargo build && cargo test     # single (only) configuration
```

`run_all_configs.sh` reads the feature list out of `Cargo.toml`, so it stays
correct if features are ever introduced. With the current `Cargo.toml` it
reports `1 configuration(s) to verify`.

Note: the suite redirects the process's own stdout/stderr to capture what each
shared object writes, so it must run serially. `.cargo/config.toml` sets
`RUST_TEST_THREADS = 1` for that, and `tests/common/mod.rs::assert_serial()`
fails loudly with an explanation rather than producing bogus diffs if it is
ever run in parallel.
