# CONFIGS.md — configuration / valid-input surface table (Phase B)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`, not from
assumptions about what matters.

## Axes the C actually branches on

**Public entry points** (from `nm -D`, both are tested directly — the low-level
scanner is *not* only reached through the wrapper):

| entry point | header? | role |
|-------------|---------|------|
| `w_utf8_drop(const char *)` | no (non-`static`, still exported) | lowest level: scans, returns first offending byte |
| `w_utf8_filter(const char *, _Bool)` | yes | composed pipeline: `w_utf8_drop` + `strdup` **or** `malloc`/`memcpy`/copy loop/`realloc` |

**Runtime options** — the API has exactly one: `w_utf8_filter`'s `replacement`.
There are no `#ifdef`s, no globals, no init/config struct, no byte-order or
element-width parameters. The state it toggles:

| `replacement` | state toggled | code path |
|---------------|---------------|-----------|
| `0` (false) | invalid bytes are **dropped**; `size`/`repl` never change; `realloc` never called; output shorter than input | `else { valid++; }` only |
| non-zero (true) | invalid bytes are **replaced** by `EF BF BD`; the `repl < 3` accounting + `realloc(size += 4096)` path is live; output can be up to ~3× input | `if (repl < 3) {...}` + 3 stores + `repl -= 3` |

Because the compiled C tests the byte with `cmpb $0x0`, `replacement` has three
distinct *input* classes to exercise: `0`, `1`, and non-canonical non-zero
(`2`, `0x80`, `0xFF`).

**Input shapes the code special-cases:**

* `*valid == '\0'` right after the scan → the `strdup` shortcut (whole input
  valid) vs. the `malloc` + copy-loop path.
* `i = valid - string` → `0` (invalid at offset 0, `memcpy` of length 0) vs.
  `> 0` (prefix `memcpy`).
* Sequence width taken by the copy loop: 1, 2, 3, or 4 bytes per iteration.
* Every accept/reject boundary of `valid_1..valid_4` (lead-byte ranges
  `0xC2..0xDF`, `0xE0..0xEF` with the `0xE0`/`0xED` second-byte splits at `0xA0`,
  `0xF0..0xF4` with the splits at `0x90`/`0x8F`).
* Count of invalid bytes vs. the `REPLACEMENT_INC = 4096` / `repl < 3`
  accounting: `0`, `1`, `2`, `1364`, `1365`, `1366`, `2729..2731`, `≫ 4096`.
  (`repl` goes `0 → 4096 → 4093 → … → 1 → 4097 → 4094 → …`, so the realloc
  cadence is 1365 replacements the first time and 1366 thereafter — a genuinely
  value-dependent code path.)
* Total length vs. 4096: below, exactly, above.
* Empty / one / many bytes; truncated multi-byte sequence at the very end of the
  buffer (the `'\0'` doubles as a non-continuation byte).

## Row table (cross product, pruned to what the C distinguishes)

Each row is driven with **many randomized inputs** from a fixed-seed PRNG
(`SEED = 0x5EED_1234_ABCD_0001`), not one hand-picked value, and asserted
byte-for-byte between the two `.so` files.

### `w_utf8_drop` — lowest-level entry point, no options

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `w_utf8_drop` | empty string `""` | [x] |
| 2 | `w_utf8_drop` | pure ASCII `0x01..0x7F`, random lengths 1..256 | [x] |
| 3 | `w_utf8_drop` | exhaustive single byte, all 255 values `0x01..0xFF` | [x] |
| 4 | `w_utf8_drop` | exhaustive 2-byte strings, all 255×255 pairs | [x] |
| 5 | `w_utf8_drop` | exhaustive 3-byte strings, all 255³ triples | [x] |
| 6 | `w_utf8_drop` | only valid 2-byte sequences (lead `0xC2..0xDF` × cont `0x80..0xBF`), exhaustive then randomly concatenated | [x] |
| 7 | `w_utf8_drop` | only valid 3-byte sequences, exhaustive over (lead, cont1) × random cont2, then concatenated | [x] |
| 8 | `w_utf8_drop` | only valid 4-byte sequences, exhaustive over (lead, cont1) × random cont2/cont3, then concatenated | [x] |
| 9 | `w_utf8_drop` | mixed valid widths (1/2/3/4) in random order, random lengths | [x] |
| 10 | `w_utf8_drop` | valid prefix of random width mix + one invalid byte + random tail (checks the returned offset, not just "some pointer") | [x] |
| 11 | `w_utf8_drop` | truncated multi-byte sequence at end of buffer (lead byte then `'\0'`; 2-, 3-, 4-byte leads, each truncation depth) | [x] |
| 12 | `w_utf8_drop` | uniform random bytes, lengths 0..64, thousands of cases | [x] |
| 13 | `w_utf8_drop` | uniform random bytes, long: lengths 4000..20000 (crosses 4096) | [x] |
| 14 | `w_utf8_drop` | biased random bytes (heavy in `0xC0..0xFF`) so long invalid runs and near-miss sequences dominate | [x] |

### `w_utf8_filter` — composed pipeline, `replacement = 0` (drop)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 15 | `w_utf8_filter` | `replacement=0`, `""` → `strdup` shortcut | [x] |
| 16 | `w_utf8_filter` | `replacement=0`, fully valid ASCII → `strdup` shortcut | [x] |
| 17 | `w_utf8_filter` | `replacement=0`, fully valid mixed-width → `strdup` shortcut | [x] |
| 18 | `w_utf8_filter` | `replacement=0`, invalid byte at offset 0 (`i == 0`, zero-length `memcpy`) | [x] |
| 19 | `w_utf8_filter` | `replacement=0`, invalid byte in the middle (`i > 0`, prefix `memcpy`) | [x] |
| 20 | `w_utf8_filter` | `replacement=0`, invalid byte as the last byte | [x] |
| 21 | `w_utf8_filter` | `replacement=0`, every byte invalid (all `0x80..0xBF` run), lengths 1..300 | [x] |
| 22 | `w_utf8_filter` | `replacement=0`, copy loop forced through each width: runs of 1-, 2-, 3-, 4-byte sequences after the first invalid byte | [x] |
| 23 | `w_utf8_filter` | `replacement=0`, uniform random bytes, lengths 0..64, thousands of cases | [x] |
| 24 | `w_utf8_filter` | `replacement=0`, biased random (heavy `0xC0..0xFF`), lengths 0..512 | [x] |
| 25 | `w_utf8_filter` | `replacement=0`, long random, lengths 4000..20000 | [x] |
| 26 | `w_utf8_filter` | `replacement=0`, truncated sequence at end of buffer, all widths/depths | [x] |

### `w_utf8_filter` — `replacement = 1` (U+FFFD substitution; realloc accounting live)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 27 | `w_utf8_filter` | `replacement=1`, `""` → `strdup` shortcut (option irrelevant, must still match) | [x] |
| 28 | `w_utf8_filter` | `replacement=1`, fully valid input → `strdup` shortcut (no realloc) | [x] |
| 29 | `w_utf8_filter` | `replacement=1`, exactly 1 invalid byte → first `realloc`, `repl: 0→4096→4093` | [x] |
| 30 | `w_utf8_filter` | `replacement=1`, exactly 2 invalid bytes | [x] |
| 31 | `w_utf8_filter` | `replacement=1`, invalid byte at offset 0 (`i == 0`) | [x] |
| 32 | `w_utf8_filter` | `replacement=1`, invalid count swept over the realloc cadence: 1363,1364,1365,1366,1367 (2nd `realloc` boundary) | [x] |
| 33 | `w_utf8_filter` | `replacement=1`, invalid count swept over the 3rd cadence: 2729,2730,2731,2732,2733 | [x] |
| 34 | `w_utf8_filter` | `replacement=1`, invalid count 4000..8000 (many reallocs; output ≈ 3× input) | [x] |
| 35 | `w_utf8_filter` | `replacement=1`, all-invalid run, lengths 1..300 | [x] |
| 36 | `w_utf8_filter` | `replacement=1`, invalid bytes interleaved with each valid width (1/2/3/4) | [x] |
| 37 | `w_utf8_filter` | `replacement=1`, uniform random bytes, lengths 0..64, thousands of cases | [x] |
| 38 | `w_utf8_filter` | `replacement=1`, biased random (heavy `0xC0..0xFF`), lengths 0..512 | [x] |
| 39 | `w_utf8_filter` | `replacement=1`, long random, lengths 4000..20000 (crosses 4096 and many reallocs) | [x] |
| 40 | `w_utf8_filter` | `replacement=1`, input length exactly 4095/4096/4097 with one invalid byte | [x] |
| 41 | `w_utf8_filter` | `replacement=1`, truncated sequence at end of buffer, all widths/depths | [x] |

### `w_utf8_filter` — non-canonical `_Bool` bytes

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 42 | `w_utf8_filter` | `replacement ∈ {2, 3, 0x7F, 0x80, 0xFF}` × fully valid input | [x] |
| 43 | `w_utf8_filter` | `replacement ∈ {2, 3, 0x7F, 0x80, 0xFF}` × invalid at offset 0 | [x] |
| 44 | `w_utf8_filter` | `replacement ∈ {2, 3, 0x7F, 0x80, 0xFF}` × random bytes, lengths 0..64 | [x] |
| 45 | `w_utf8_filter` | `replacement ∈ {2, 3, 0x7F, 0x80, 0xFF}` × ≥ 1400 invalid bytes (realloc path via a non-canonical bool) | [x] |

### Composed pipeline (both entry points against the same buffer)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 46 | `w_utf8_drop` **then** `w_utf8_filter` | same buffer fed to the scanner and then to both filter modes; the drop offset must agree AND the filtered output must agree, for random and biased inputs (catches divergence only visible in the composition) | [x] |
| 47 | `w_utf8_filter` idempotence cross-check | `w_utf8_filter(x, r)` output re-fed to `w_utf8_drop`; C and Rust must agree on the second pass too | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one; `cargo test --no-default-features` and
`--all-features` are the same build. This is verified mechanically by
`check_features.sh` rather than assumed.

## Row → test mapping

Rows 1–47 map one-to-one onto tests named `rowNN_…`:

* rows 1, 2, 6–14 and 15–47 → `tests/phase_b_configs.rs` (44 tests)
* rows 3, 4, 5 (the exhaustive sweeps) → `tests/phase_b_exhaustive.rs` (6 tests)

Every row is driven with many randomized inputs from `SEED`
(`0x5EED_1234_ABCD_0001`, per-row sub-seeded) — several thousand inputs for the
random rows, and *complete* enumeration for rows 3–5:

| sweep | inputs | both entry points? |
|-------|--------|--------------------|
| all 1-byte strings | 255 | drop + filter × 7 mode bytes |
| all 2-byte strings | 65 025 | drop + filter × 2 modes (+ 2 non-canonical modes over the boundary leads) |
| all 3-byte strings | 16 581 375 | drop + filter × 2 modes |
| all 4-byte strings over 18 boundary lead bytes | 18 × 255³ ≈ 298 M | drop |

## Feature combinations

`check_features.sh` extracts the `[features]` table from `Cargo.toml` and loops
over its power set. Verified output:

```
features declared in Cargo.toml: 0 (none)
PASS  [default] profile=dev (7 test binaries ok)
PASS  [default] profile=release (7 test binaries ok)
PASS  [no-default-features] profile=dev (7 test binaries ok)
PASS  [no-default-features] profile=release (7 test binaries ok)
ALL FEATURE COMBINATIONS PASSED
```

There is no `[features]` table, so `--no-default-features`, `--all-features` and
the default build are the same code. The script therefore also runs the whole
suite against **both** builds of the Rust `cdylib`, which *is* a real behavioural
axis for this crate:

* `target/debug/libdriver.so` — `debug_assert!` active, integer overflow checks
  on (this is what would catch a `repl -= 3` underflow), `panic = unwind`;
* `target/release/libdriver.so` — optimised, `panic = "abort"`.

Both pass identically, and both abort (SIGABRT) on a NULL argument like the C.
