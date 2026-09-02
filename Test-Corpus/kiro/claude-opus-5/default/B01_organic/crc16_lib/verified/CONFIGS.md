# CONFIGS.md — Configuration-surface table (Phase A / gate for Phase B)

Derived mechanically from the C source: the axes below are exactly the things
`c_src/src/lib.c` branches on or indexes with.

## Public entry points (complete set)

`grep -nE "^[a-zA-Z_].*\(" c_src/include/lib.h` yields one declaration:

```c
tflac_u16 crc16(const tflac_u8 *d, tflac_u32 len, tflac_u16 crc16);
```

There is **no** convenience/one-shot wrapper layered over a lower-level API —
`crc16` *is* the lowest-level entry point and the only one. So "exercise the
low-level entry points, not just the wrappers" is satisfied by driving `crc16`
directly through the `.so`, including **incremental/streaming** use (feeding the
previous return value back in as the seed), which is how a real consumer uses a
CRC and which composes the two internal loops in ways a single one-shot call
cannot reach.

## Axes the C code actually distinguishes

There are no runtime flags, modes, `#ifdef`s, or option setters (0 hits for
`if`, `switch`, `#if`, `enum` outside the loop conditions). All variability is
in the **input shape** and **parameter values**:

| axis | values the C treats differently | why (source evidence) |
|------|--------------------------------|------------------------|
| A. `len` vs the 8-byte block loop | `0`; `1..=7` (tail only); `8` (one block, no tail); `9..=15` (block + tail); exact multiples of 8; large non-multiples | `while (len >= 8)` then `while (len--)` — two distinct code paths selected purely by `len` |
| B. `len % 8` | each residue `0,1,2,3,4,5,6,7` | decides how many single-byte tail iterations run after the block loop |
| C. block count | 0, 1, 2, many (>1000 blocks) | pointer advance `d += 8` and repeated table mixing accumulate errors only over multiple blocks |
| D. seed `crc` | `0x0000`; `0xFFFF`; high-byte-only (`0xAB00`); low-byte-only (`0x00CD`); random; all 65536 | `crc16 >> 8` and `crc16 & 0xFF` are *both* used as table indices in the block loop; `crc16 << 8` truncates in the tail loop |
| E. byte values in `d` | all `0x00`; all `0xFF`; ascending `0..255`; random; single-byte sweep of all 256 values | `d[2]`..`d[7]` and `*d` are raw table indices — value-dependent, so one hand-picked buffer covers only 8 of 256 index values per table |
| F. which of the 8 tables is exercised | tables `[0]` (tail + block) and `[1]`..`[7]` (block only) | table `[0]` is the only one used by the tail loop; `[1]`..`[7]` are reachable **only** via the block loop, so a tail-only test never touches 7/8 of the data |
| G. streaming vs one-shot | one call over N bytes **vs** k calls over the same N bytes split at arbitrary offsets | exercises seed-feedback and non-8-aligned resumption; the split point makes the same total data traverse different block/tail combinations |
| H. buffer alignment / offset | `d` at offset 0..7 inside a larger allocation | C reads byte-at-a-time so results must be alignment-independent; catches any Rust attempt to read wider words |

Byte order, element width, and element type are **not** axes: the API is a flat
`const tflac_u8 *` read one byte at a time, with no multi-byte loads and no
endianness-dependent code.

## Configuration rows (pruned cross-product of the axes above)

Every row is tested against **many randomized inputs with a fixed seed**
(`SEED = 0x0BADC0DE_D15EA5E5`, xorshift64* PRNG defined in the test file), not a
single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `crc16` | `len = 0`, seed random x 256, buffer non-empty (pointer must be untouched) | [x] |
| C2 | `crc16` | `len = 1`, **all 256** byte values x all 256 high-seed bytes (exhaustive tail-loop table-`[0]` sweep) | [x] |
| C3 | `crc16` | `len` in `1..=7` (tail loop only, no block), random bytes, random seed, 200 cases per length | [x] |
| C4 | `crc16` | `len = 8` exactly (one block, empty tail), random bytes, random seed, 2000 cases | [x] |
| C5 | `crc16` | `len = 8`, data = all `0x00` / all `0xFF` / `0..7` ascending, seed `0x0000`/`0xFFFF`/random (block-loop extremes) | [x] |
| C6 | `crc16` | `len` in `9..=15` (one block + each tail residue `1..=7`), random data+seed, 200 cases per length | [x] |
| C7 | `crc16` | `len = 16, 24, 32, ... 128` (exact multiples of 8, multi-block, empty tail), random data+seed | [x] |
| C8 | `crc16` | `len` in `17..=127` non-multiples of 8 (multi-block + tail, every residue), random data+seed | [x] |
| C9 | `crc16` | `len` random in `0..=4096`, fully random data, fully random seed — 4000 property-style cases | [x] |
| C10 | `crc16` | large buffers: `len` = 65536, 65537, 100000, 1000003 (many blocks; exercises pointer advance over >8k blocks), random data, random seed | [x] |
| C11 | `crc16` | seed axis exhaustive: **all 65536** seed values with a fixed 8-byte block (block loop, both `crc>>8` and `crc&0xFF` index paths) | [x] |
| C12 | `crc16` | seed axis exhaustive: **all 65536** seed values with a fixed 1-byte input (tail loop) | [x] |
| C13 | `crc16` | data-value axis: buffer = `0..=255` repeated, `len` swept `0..=300` so every byte value lands in every `d[0..8]` block position and every tail position | [x] |
| C14 | `crc16` | table coverage: inputs constructed so `d[2]`..`d[7]` each take all 256 values (drives tables `[5]`..`[0]` across their full index range) | [x] |
| C15 | `crc16` | streaming/incremental: split a 1024-byte buffer at **every** offset `0..=1024` into two calls, feeding call 1's return as call 2's seed; compare C-chained vs Rust-chained and vs the one-shot value | [x] |
| C16 | `crc16` | streaming with 3+ random chunks (random split points, random chunk count 1..=16) over random buffers, 1000 cases | [x] |
| C17 | `crc16` | alignment: same 64-byte payload read from offsets `0..=7` of a larger allocation, all lengths `0..=56` | [x] |
| C18 | `crc16` | `len` = 0 chained into a non-zero call (identity/composition of the degenerate case) | [x] |
| C19 | `crc16` | known-answer cross-check: `"123456789"` and the FLAC-frame-style all-zero/all-ff payloads, C vs Rust (guards against both sides being wrong in the same trivially-broken way, e.g. always returning the seed) | [x] |
| C20 | `crc16` | adversarial byte patterns: `0x00/0xFF` alternating, `0x80` runs, high-bit-set only, sequences chosen so `crc` hits `0x0000` mid-stream, `len` `0..=64` | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table**, so there is exactly one
feature combination (the default = empty set). `--no-default-features` and the
default build are the same compilation unit; both are run in Phase D anyway to
prove it. All 20 rows above are therefore run under the single existing
configuration, plus separately under `--release` and debug profiles (debug
matters: Rust's debug builds panic on arithmetic overflow and on out-of-range
indexing, which is precisely where a C translation with wrapping/truncating
arithmetic diverges — see `ERRORS.md` E5/E6).

## Harness adequacy — mutation testing (why these rows are trusted)

Passing tests only mean something if the tests can fail. Two things were
verified before trusting the green run.

### 1. A real harness defect was found and fixed: the stale `.so`

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` target — it
compiles `src/lib.rs` as a test harness instead. A harness that globs
`target/<profile>/*.so` therefore loads whatever `.so` an *earlier*
`cargo build` left behind, and the entire differential suite becomes vacuous.
This was observed directly: with the first version of the harness, **every
injected bug survived**. `tests/harness/mod.rs` now builds the cdylib itself
into `target/so-under-test/<profile>` (a dedicated `--target-dir`, because
cargo's build lock is per target dir and reusing the one `cargo test` holds
would deadlock), asserts the artifact is newer than every file in `src/`, and
`phase_c_errors::symbol_export_shape` asserts the loaded path is that
freshly-built, profile-matched artifact.

### 2. Mutants injected into `src/lib.rs` / `src/tables.rs`

| mutant | change | result |
|--------|--------|--------|
| M1 | tail loop `TABLES[0]` -> `TABLES[1]` | CAUGHT |
| M2 | block loop `len >= 8` -> `len >= 9` | survived — **equivalent mutant**, see below |
| M3 | `tables[0][1]`: `0x8005` -> `0x8006` (one of 2048 values) | CAUGHT |
| M4 | block loop `TABLES[5][d[2]]` -> `TABLES[4][d[2]]` | CAUGHT |
| M5 | block loop pointer advance `d.add(8)` -> `d.add(7)` | CAUGHT |
| M6 | block loop `crc >> 8` -> `crc & 0xFF` on `TABLES[7]` | CAUGHT |
| M7 | block loop byte order: `(b0<<8)|b1` -> `(b1<<8)|b0` | CAUGHT |
| M8 | `return 0` instead of the seed when `len == 0` | CAUGHT |
| M9 | tail loop index `crc >> 8` -> `crc & 0xFF` | CAUGHT |
| M10 | tail loop `wrapping_shl(8)` -> `wrapping_shr(8)` | CAUGHT |
| M11 | no-op control (`len = len.wrapping_add(0)`) | survived, as it must |
| M12 | block loop: drop the `TABLES[0][d[7]]` term | CAUGHT |

10 of 10 semantically distinct mutants are caught; the no-op control survives,
so the suite is not trivially failing everything.

**M2 is an equivalent mutant, not a gap.** The 8-byte block loop is the
slice-by-8 form of eight consecutive single-byte steps, so raising the
threshold to `len >= 9` only reroutes work between two loops that compute the
same function. `tests/equivalence_note.rs` proves this *against the C `.so`
itself*: for 500 random buffers, one-shot `crc16(d, n, seed)` equals feeding the
same bytes one at a time. No test can distinguish M2 because no input does.
