# CONFIGS.md — configuration surface table (valid paths)

Derived mechanically from the public header and every branch the C actually
takes. The public API is a *single* entry point, so "lowest-level entry point"
and "convenience wrapper" coincide — there is nothing above or below `hex2bin`:

```c
/* c_src/include/lib.h — the complete public surface */
int hex2bin(uint8_t *bin, size_t bin_maxlen, const char *hex,
            size_t hex_len, const char *ignore, const char **hex_end_p);
```

## Axes the C branches on

| axis | values the C distinguishes | source of the distinction |
|------|----------------------------|---------------------------|
| A. `ignore` mode | `NULL` / `""` / single-char set / multi-char set / set containing hex digits / set containing bytes ≥ `0x80` | line 23 `ignore != NULL`, line 24 `strchr(ignore, c)` |
| B. `hex_end_p` mode | `NULL` / non-`NULL` | line 50 vs. line 52 — changes the *return value* for partially consumed input |
| C. `bin_maxlen` shape | `0` / `< hex_len/2` / exactly `hex_len/2` / `> hex_len/2` / `SIZE_MAX` | line 31 `bin_pos >= bin_maxlen` |
| D. `hex_len` parity & size | `0` / `1` (odd) / `2` / odd `>1` / even `>2` / long (≥ 512) | loop bound + line 43 `state != 0U` |
| E. digit character class | `'0'..'9'` / `'a'..'f'` / `'A'..'F'` / mixed case | `c_num0` branch vs. `c_alpha0` branch, line 29 `(c_num0 & c_num) | (c_alpha0 & c_alpha)` |
| F. separator placement | none / leading / trailing / between bytes (even `state`) / mid-byte (odd `state`) / runs of separators / whole input separators | line 23 `state == 0U` conjunct |
| G. non-hex byte class | ASCII punctuation / range-adjacent bytes (`/ : @ G \` g`) / `0x00` / `0x80..0xFF` | line 22 `(c_num0 \| c_alpha0) == 0U` |
| H. `bin` pointer | non-`NULL` / `NULL` (only legal when never dereferenced) | no null check in C |
| I. `hex` pointer | non-`NULL` / `NULL` with `hex_len == 0` | no null check in C |

`state` (`0x00` ⇄ `0xFF` via `state = ~state`) and `c_acc` are internal, but
axes D and F exist precisely to drive both of `state`'s values through every
branch.

## Configuration rows

Cross-product of A×B×C×D×E×F×G, pruned to combinations the C treats
differently. Every row is exercised with **many randomized inputs** (fixed seed
`0x5EED_1234_ABCD_0001`, deterministic xorshift PRNG) in
`tests/differential.rs`; a row is checked off only after the whole randomized
sweep matches C byte-for-byte (return value, `bin` buffer *including* the
untouched tail, and `*hex_end_p` offset).

| #  | entry point | configuration (options set + input shape)                                                                                        | [x] |
|----|-------------|----------------------------------------------------------------------------------------------------------------------------------|-----|
| 1  | `hex2bin`   | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen == hex_len/2`, even `hex_len` 2..64, lowercase digits only                            | [x] |
| 2  | `hex2bin`   | `ignore=NULL`, `hex_end_p=NULL`, exact `bin_maxlen`, even `hex_len`, uppercase digits only                                        | [x] |
| 3  | `hex2bin`   | `ignore=NULL`, `hex_end_p=NULL`, exact `bin_maxlen`, even `hex_len`, decimal digits only                                          | [x] |
| 4  | `hex2bin`   | `ignore=NULL`, `hex_end_p=NULL`, exact `bin_maxlen`, even `hex_len`, mixed-case digits (all of A/a/0 classes interleaved)          | [x] |
| 5  | `hex2bin`   | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen > hex_len/2` (slack), even `hex_len`, mixed case — asserts tail of `bin` untouched     | [x] |
| 6  | `hex2bin`   | `ignore=NULL`, `hex_end_p=NULL`, `bin_maxlen == SIZE_MAX`, even `hex_len`, mixed case                                             | [x] |
| 7  | `hex2bin`   | `ignore=NULL`, `hex_end_p=non-NULL`, exact `bin_maxlen`, even `hex_len`, mixed case — checks `*hex_end_p == &hex[hex_len]`          | [x] |
| 8  | `hex2bin`   | `ignore=NULL`, `hex_end_p=non-NULL`, `hex_len == 0`, `bin_maxlen` random                                                          | [x] |
| 9  | `hex2bin`   | `ignore=NULL`, `hex_end_p=non-NULL`, long even input (512..2048 nibbles), exact `bin_maxlen`, mixed case                           | [x] |
| 10 | `hex2bin`   | `ignore=NULL`, `hex_end_p=non-NULL`, `bin_maxlen` random in `0..=hex_len` (straddles the buffer-full boundary), even `hex_len`      | [x] |
| 11 | `hex2bin`   | `ignore=NULL`, `hex_end_p=non-NULL`, odd `hex_len` (1,3,5,…), all-hex input                                                       | [x] |
| 12 | `hex2bin`   | `ignore=""`, `hex_end_p=non-NULL`, exact `bin_maxlen`, even `hex_len`, mixed case (empty ignore ≠ `NULL` ignore only for `0x00`)     | [x] |
| 13 | `hex2bin`   | `ignore=":"`, `hex_end_p=non-NULL`, separators between complete bytes (even `state`), exact `bin_maxlen`                            | [x] |
| 14 | `hex2bin`   | `ignore=": -"` (multi-char), `hex_end_p=non-NULL`, random separator runs between bytes, exact `bin_maxlen`                          | [x] |
| 15 | `hex2bin`   | `ignore=": -"`, `hex_end_p=non-NULL`, **leading** separator run                                                                    | [x] |
| 16 | `hex2bin`   | `ignore=": -"`, `hex_end_p=non-NULL`, **trailing** separator run (`hex_pos` must reach `hex_len`)                                   | [x] |
| 17 | `hex2bin`   | `ignore=": -"`, `hex_end_p=non-NULL`, separator placed **mid-byte** (odd `state`) → parsing stops there                             | [x] |
| 18 | `hex2bin`   | `ignore=": -"`, `hex_end_p=NULL`, separators between bytes — return value differs from the `hex_end_p != NULL` case                 | [x] |
| 19 | `hex2bin`   | `ignore` = whole input's alphabet (input is *only* separators), `hex_end_p` both modes                                             | [x] |
| 20 | `hex2bin`   | `ignore="abc0"` (contains valid hex digits — provably inert), `hex_end_p=non-NULL`, mixed-case even input                           | [x] |
| 21 | `hex2bin`   | `ignore` containing bytes ≥ `0x80` (e.g. `"\x80\xFF\xA5"`), `hex` sprinkled with those same high bytes, `hex_end_p=non-NULL`         | [x] |
| 22 | `hex2bin`   | `ignore` = all 255 non-NUL bytes, `hex` = fully random bytes, `hex_end_p=non-NULL`, random `bin_maxlen`                             | [x] |
| 23 | `hex2bin`   | `ignore=NULL`, `hex` = fully random bytes (`0x00..=0xFF`), random `bin_maxlen`, random `hex_len`, both `hex_end_p` modes             | [x] |
| 24 | `hex2bin`   | `ignore` = random 1..8-byte set, `hex` = random bytes drawn from hex-digits ∪ separators ∪ junk, random `bin_maxlen`, both `hex_end_p` modes | [x] |
| 25 | `hex2bin`   | `bin=NULL` + `bin_maxlen=0`, `hex_len` 0 and >0, both `hex_end_p` modes, `ignore` both modes                                        | [x] |
| 26 | `hex2bin`   | `hex=NULL` + `hex_len=0`, all four (`ignore`,`hex_end_p`) mode combinations                                                        | [x] |
| 27 | `hex2bin`   | exhaustive single-byte sweep: `hex_len=1`, `hex[0]` over all `0x00..=0xFF`, × `ignore ∈ {NULL, "", byte itself, other}` × `hex_end_p ∈ {NULL, set}` × `bin_maxlen ∈ {0,1}` | [x] |
| 28 | `hex2bin`   | exhaustive two-byte sweep: `hex_len=2`, both bytes over all `0x00..=0xFF` (65 536 pairs), `ignore=NULL`, `hex_end_p` set, `bin_maxlen=1` | [x] |
| 29 | `hex2bin`   | `hex_len` shorter than the backing buffer (parser must not read past `hex_len`) — guard bytes after `hex_len` are non-hex vs. hex    | [x] |
| 30 | `hex2bin`   | `bin_maxlen` random in `0..=2` with long inputs (early buffer-full on a long stream), both `hex_end_p` modes                        | [x] |
| 31 | `hex2bin`   | **whole-surface fuzz**: axes A–I randomized *simultaneously* and independently (200 000 iterations) — the unconstrained cross-product where interaction bugs live | [x] |

All 31 rows pass across their randomized sweeps — see `tests/differential.rs`
(`configs_md_row_XX_*` and `fuzz_all_axes_simultaneously`).

## Evidence

Every row test records how many C-vs-Rust comparisons it actually performed and
asserts a lower bound, so a loop that silently iterated zero times fails instead
of passing (`assert_did_work` in `tests/common/mod.rs`). Measured totals:

```
$ cargo test --release -- --nocapture --test-threads=1
tests reporting counts: 60
total C-vs-Rust differential comparisons: 396435
```

Each comparison checks three observables byte-for-byte: the `int` return value,
the **entire** `bin` buffer (pre-filled with `0xA5` so stray writes past the
reported length are caught), and the `*hex_end_p` offset.

## Feature combinations

`translation/Cargo.toml` has no `[features]` section, so the feature
cross-product is the single default configuration. `scripts/check_features.sh`
enumerates the features from `Cargo.toml` and re-runs the full suite for each
combination (default and `--no-default-features`), rather than assuming.
