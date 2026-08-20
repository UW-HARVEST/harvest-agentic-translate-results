# ERRORS.md — Phase A, step 2: ERROR-SURFACE TABLE

Mechanically derived from `c_src/src/lib.c`. The library has **no** error
enums, no `RETURN_ERROR` macros, no `assert`, no `return NULL`, no `errno` use
and no `NULL` checks. Its *entire* rejection surface consists of the
short-circuiting boolean conditions in `hdr_valid` and `hdr_compare`; each one
that can make the call return `0` instead of `1` is one row below.

Full C source under test:

```c
static int hdr_valid(const uint8_t *h) {
    return h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) &&
           ((((h[1]) >> 1) & 3) != 0) && (((h[2]) >> 4) != 15) &&
           ((((h[2]) >> 2) & 3) != 3);
}

int hdr_compare(const uint8_t *h1, const uint8_t *h2) {
    return hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 &&
           ((h1[2] ^ h2[2]) & 0x0C) == 0 &&
           !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0));
}
```

Grep inventory of every rejection site (the anti-blind-spot step):

| grep hit | file:line | kind |
|----------|-----------|------|
| `h[0] == 0xff` | lib.c:4 | equality gate (sync byte 1) |
| `(h[1] & 0xF0) == 0xf0` | lib.c:4 | mask/equality gate (branch A of `\|\|`) |
| `(h[1] & 0xFE) == 0xe2` | lib.c:4 | mask/equality gate (branch B of `\|\|`) |
| `(((h[1]) >> 1) & 3) != 0` | lib.c:5 | reserved-value range check (layer) |
| `((h[2]) >> 4) != 15` | lib.c:5 | reserved-value range check (bitrate index) |
| `(((h[2]) >> 2) & 3) != 3` | lib.c:6 | reserved-value range check (sample-rate index) |
| `((h1[1] ^ h2[1]) & 0xFE) == 0` | lib.c:10 | masked-equality gate |
| `((h1[2] ^ h2[2]) & 0x0C) == 0` | lib.c:11 | masked-equality gate |
| `!(((h1[2] & 0xF0) == 0) ^ ((h2[2] & 0xF0) == 0))` | lib.c:12 | XOR-of-predicates gate |
| *(none)* | — | no `assert`, no `NULL` check, no `return -1`, no error enum |

## The table

`h2v` below = a fully valid `h2` (e.g. `FF FB 90 00`). "expected C result" is
the `int` returned by `hdr_compare`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `hdr_compare` → `hdr_valid(h2)` | `h2[0] != 0xff` (all 255 non-`0xff` values), rest of `h2` valid, `h1 == h2` | `0` |
| 2 | `hdr_compare` → `hdr_valid(h2)` | `h2[1]` satisfies *neither* `(h2[1] & 0xF0) == 0xf0` *nor* `(h2[1] & 0xFE) == 0xe2` (all 238 such byte values — 256 minus the 18 that pass: `0xF0..0xFF`, `0xE2`, `0xE3`), `h2[0]=0xff`, `h2[2]` valid, `h1 == h2` | `0` |
| 3 | `hdr_compare` → `hdr_valid(h2)` | layer field reserved: `((h2[1] >> 1) & 3) == 0`, i.e. `h2[1] & 0xF0 == 0xF0` with `h2[1] ∈ {0xF0,0xF1,0xF8,0xF9}` (the only sync-passing values with layer 0), `h1 == h2` | `0` |
| 4 | `hdr_compare` → `hdr_valid(h2)` | bitrate index reserved: `(h2[2] >> 4) == 15`, i.e. `h2[2] ∈ 0xF0..0xFF` with valid sample-rate bits, valid `h2[0..1]`, `h1 == h2` | `0` |
| 5 | `hdr_compare` → `hdr_valid(h2)` | sample-rate index reserved: `((h2[2] >> 2) & 3) == 3`, i.e. `h2[2] & 0x0C == 0x0C`, valid `h2[0..1]`, bitrate ≠ 15, `h1 == h2` | `0` |
| 6 | `hdr_compare` | version/layer/sync mismatch: `h2` valid, `h1[1] = h2[1] ^ m` for every one of the 254 `m ∈ 0..255` with `m & 0xFE != 0` (i.e. all 127 distinct non-zero masked deltas) | `0` |
| 7 | `hdr_compare` | sample-rate mismatch: `h2` valid, `h1[2] = h2[2] ^ m` for `m ∈ {0x04, 0x08, 0x0C}` (all 3 non-zero deltas inside `0x0C`) | `0` |
| 8 | `hdr_compare` | free-format mismatch A: `h2` valid with bitrate index `!= 0`, `h1[2]` has bitrate index `== 0`, same sample-rate bits → `1 ^ 0` | `0` |
| 9 | `hdr_compare` | free-format mismatch B: `h2` valid with bitrate index `== 0` (free format), `h1[2]` has bitrate index `!= 0`, same sample-rate bits → `0 ^ 1` | `0` |
| 10 | `hdr_compare` | *combined* failures: rows 1–5 each ANDed with a simultaneously mismatching `h1` (verifies the first failing gate short-circuits and the result is still `0`, never `-1`/garbage) | `0` |
| 11 | `hdr_compare` (read-extent contract) | `hdr_valid(h2)` fails at `h2[0] != 0xff` while `h1` is a **null pointer** and `h2[1..]` is **unmapped memory** — C short-circuits and never dereferences them | `0`, no fault |
| 12 | `hdr_compare` (read-extent contract) | `hdr_valid(h2)` fails on `h2[1]` while `h1` is **null** and `h2[2]` is **unmapped** — C never reads `h2[2]` nor any `h1` byte | `0`, no fault |
| 13 | `hdr_compare` (read-extent contract) | `hdr_valid(h2)` fails on `h2[2]` (bitrate 15 / sample-rate 3) while `h1` is **null** — C never reads `h1` | `0`, no fault |
| 14 | `hdr_compare` (read-extent contract) | `h1[1] ^ h2[1]` mismatch while `h1[2]`/`h2[2]` are the last mapped bytes and `h1[3]`/`h2[3]` are **unmapped** — C never reads index ≥ 3 | `0`, no fault |
| 15 | `hdr_compare` (read-extent contract) | success path with only bytes `0..2` mapped for both headers and `h1[0]` never read — C reads at most `h1[1..2]`, `h2[0..2]` | `1`, no fault |
| 16 | `hdr_compare` (out-of-range "enum" values across FFI) | every reserved/undefined field encoding is exercised exhaustively: all 256 values of `h2[0]`, all 256 of `h2[1]`, all 256 of `h2[2]`, all 256 of `h1[1]`, all 256 of `h1[2]` — including all reserved layer (`0`), bitrate (`15`) and sample-rate (`3`) codes, in `h1` (which the C never validates) as well as `h2` | must equal C bit-for-bit for every one of the 2^32 masked-relevant combinations sampled/enumerated |

### Notes on deliberate non-checks (these are NOT errors in the C)

These are recorded so the Rust is not "fixed" to reject them:

* `h1` is **never validated**. `h1[0]` is **never read at all**; `h1` may be a
  totally malformed header (sync byte `0x00`, reserved bitrate `15`, reserved
  layer `0`, reserved sample-rate `3`) and `hdr_compare` still returns `1` as
  long as `h1[1] & 0xFE == h2[1] & 0xFE`, `h1[2] & 0x0C == h2[2] & 0x0C` and
  the free-format predicates agree.
* Bit `0` of `h[1]` (CRC/protection) is masked out of the comparison (`0xFE`)
  and is not checked by `hdr_valid` → may differ freely.
* Bits `0..1` of `h[2]` (padding, private) are masked out (`0x0C` / `0xF0`
  gates) → may differ freely.
* Byte `h[3]` and beyond are never read.
* `NULL` (and any invalid pointer) passed where C *does* dereference is
  undefined behaviour in the C and is reproduced as the same unchecked
  dereference in Rust; only the *short-circuit* cases (rows 11–15), where the C
  contractually does **not** dereference, are observable and therefore tested.

## Row status

| row | test | status |
|-----|------|--------|
| 1 | `tests/error_paths.rs::row01_h2_sync_byte0_invalid` | [x] |
| 2 | `tests/error_paths.rs::row02_h2_sync_bits_invalid` | [x] |
| 3 | `tests/error_paths.rs::row03_h2_layer_reserved` | [x] |
| 4 | `tests/error_paths.rs::row04_h2_bitrate_index_15` | [x] |
| 5 | `tests/error_paths.rs::row05_h2_samplerate_index_3` | [x] |
| 6 | `tests/error_paths.rs::row06_h1_version_layer_mismatch` | [x] |
| 7 | `tests/error_paths.rs::row07_h1_samplerate_mismatch` | [x] |
| 8 | `tests/error_paths.rs::row08_free_format_mismatch_a` | [x] |
| 9 | `tests/error_paths.rs::row09_free_format_mismatch_b` | [x] |
| 10 | `tests/error_paths.rs::row10_combined_failures` | [x] |
| 11 | `tests/read_extent.rs` (guard-page child process) | [x] |
| 12 | `tests/read_extent.rs` (guard-page child process) | [x] |
| 13 | `tests/read_extent.rs` (guard-page child process) | [x] |
| 14 | `tests/read_extent.rs` (guard-page child process) | [x] |
| 15 | `tests/read_extent.rs` (guard-page child process) | [x] |
| 16 | `tests/error_paths.rs::row16_all_reserved_encodings_exhaustive` + `tests/exhaustive.rs` | [x] |

## Result

All 16 rows have a passing differential test that asserts C and Rust return the
**same** rejection sentinel (`0`) — not merely "both failed somehow". Rows 1–10
and 16 additionally pin the exact expected value so a change in either
direction (wrongly accepting or wrongly rejecting) is caught.

Run: `cargo test --no-default-features` → `tests/error_paths.rs` 12/12 passing,
`tests/read_extent.rs` 2/2 passing (+2 worker tests executed in child
processes). Verified in both the `dev` and `release` profiles.

Every row was also confirmed to be *load-bearing* by mutation testing (see the
mutation table in `CONFIGS.md`): all 14 injected mutants, including all five
short-circuit/read-extent violations, were detected. Mutant survivors: 0.
