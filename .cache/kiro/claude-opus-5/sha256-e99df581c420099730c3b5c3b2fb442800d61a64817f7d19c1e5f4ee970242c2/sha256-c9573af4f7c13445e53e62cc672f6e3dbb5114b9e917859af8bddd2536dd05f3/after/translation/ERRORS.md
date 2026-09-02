# ERRORS.md — Error / rejection surface table (Phase A, gates Phase C)

## Mechanical derivation

Every way the C source can reject input, found by exhaustively reading both C
files (15 lines of code total):

```
$ grep -nE 'RETURN_ERROR|return|assert|NULL|errno|abort|exit|<|>|==|!=' c_src/src/lib.c c_src/include/lib.h
c_src/include/lib.h:3:  int hdr_compare(const uint8_t *h1, const uint8_t *h2);
c_src/src/lib.c:3:      static int hdr_valid(const uint8_t *h) {
c_src/src/lib.c:4:          return h[0] == 0xff && ((h[1] & 0xF0) == 0xf0 || (h[1] & 0xFE) == 0xe2) &&
c_src/src/lib.c:5:                 ((((h[1]) >> 1) & 3) != 0) && (((h[2]) >> 4) != 15) &&
c_src/src/lib.c:6:                 ((((h[2]) >> 2) & 3) != 3);
c_src/src/lib.c:9:      int hdr_compare(const uint8_t *h1, const uint8_t *h2) {
c_src/src/lib.c:10:         return hdr_valid(h2) && ((h1[1] ^ h2[1]) & 0xFE) == 0 &&
c_src/src/lib.c:11:                ((h1[2] ^ h2[2]) & 0x0C) == 0 &&
c_src/src/lib.c:12:                !((((h1[2]) & 0xF0) == 0) ^ (((h2[2]) & 0xF0) == 0));
c_src/src/lib.c:13:     }
```

Findings that shape this table:

- There are **no** error macros, no `errno`, no error enum, no `assert`, no
  `abort`, no `NULL` check, no allocation, and no `#ifdef` in the C source.
- There are **no enum parameters** anywhere in the public API (`lib.h` declares
  one function taking two `const uint8_t *`), so "out-of-range enum value across
  the FFI boundary" has no instance in this library. The analogous class of
  input — a byte value with no meaningful interpretation, e.g. a reserved
  bitrate/sampling-rate field — *does* exist and is rows 4–6 below.
- The function's only failure signal is its **return value: `0` = rejected,
  `1` = accepted** (`int`, produced by C's `&&`/`!` operators, which yield
  exactly `0` or `1`). "Same error code" in Phase C therefore means "both sides
  return the identical `int`".
- Each term of the two `&&` chains is a distinct rejection branch, and C's `&&`
  **short-circuits**: a rejection at term *n* means terms *n+1…* are never
  evaluated, so `h1` is never dereferenced when `hdr_valid(h2)` is false. That
  short-circuit is itself an observable behaviour (row 11) and is tested.

## Error-surface table

Rows 1–5 are the rejection branches inside `hdr_valid`, reached via
`hdr_compare`'s first term with `h = h2`. Rows 6–9 are `hdr_compare`'s own
terms. Rows 10–13 are the generic FFI boundary conditions.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `hdr_valid` (via `hdr_compare`, `h=h2`) | `h2[0] != 0xff` — first sync byte wrong. Any of the 255 non-`0xff` values, with `h2[1]`/`h2[2]`/`h1` otherwise fully valid & matching | returns `0` |
| 2 | `hdr_valid` (via `hdr_compare`) | `(h2[1] & 0xF0) != 0xf0` **and** `(h2[1] & 0xFE) != 0xe2` — second sync byte in neither accepted class. The 238 values outside `{0xe2,0xe3} ∪ [0xf0,0xff]` | returns `0` |
| 3 | `hdr_valid` (via `hdr_compare`) | `((h2[1] >> 1) & 3) == 0` — reserved MPEG layer field, *even though* term 2 passed. Exactly `h2[1] ∈ {0xf0,0xf1,0xf8,0xf9}` | returns `0` |
| 4 | `hdr_valid` (via `hdr_compare`) | `(h2[2] >> 4) == 15` — reserved/bad bitrate index. Exactly `h2[2] ∈ [0xf0,0xff]` | returns `0` |
| 5 | `hdr_valid` (via `hdr_compare`) | `((h2[2] >> 2) & 3) == 3` — reserved sampling-rate field. Exactly `h2[2] & 0x0C == 0x0C` (64 values), with `(h2[2]>>4) != 15` so row 4 did not already fire | returns `0` |
| 6 | `hdr_compare` | `hdr_valid(h2) == 0` (the aggregate first term) — any h2 rejected by rows 1–5, for arbitrary `h1` | returns `0` |
| 7 | `hdr_compare` | `((h1[1] ^ h2[1]) & 0xFE) != 0` — `h1[1]` differs from valid `h2[1]` in any bit except bit 0 (i.e. `h1[1] ∉ {h2[1] & 0xFE, h2[1] \| 0x01}`) | returns `0` |
| 8 | `hdr_compare` | `((h1[2] ^ h2[2]) & 0x0C) != 0` — sampling-rate bits (bits 2–3) of `h1[2]` differ from `h2[2]`, with rows 1–7 passing | returns `0` |
| 9 | `hdr_compare` | `((h1[2] & 0xF0) == 0) != ((h2[2] & 0xF0) == 0)` — exactly one of the two headers has a zero (free-format) bitrate nibble, with rows 1–8 passing | returns `0` |
| 10 | `hdr_compare` | `h1 == NULL` **while** `hdr_valid(h2)` is false — legal in C because `&&` short-circuits before `h1` is read | returns `0`, no crash |
| 11 | `hdr_compare` | `h1` pointing at an unmapped non-null address (`0x1`), and `h1` placed at the last readable byte before a `PROT_NONE` guard page, **while** `hdr_valid(h2)` is false — same short-circuit guarantee: bytes `h1[1]`, `h1[2]` must never be touched | returns `0`, no crash |
| 12 | `hdr_compare` | `h1 == h2` (aliasing the same 3 bytes) with a valid header — the `^`/nibble terms all compare a byte with itself | returns `1` (accept), for every valid `h2` |
| 13 | `hdr_compare` | reading only 3 bytes: `h1[0]` is **never** dereferenced by the C at all (no term mentions `h1[0]`), and neither side may read `h[3]` or beyond | result independent of `h1[0]`; buffers of exactly 3 bytes suffice for both sides |

Notes on conditions deliberately **not** in the table because the C has
undefined behaviour there and so has no "expected result" to match:
`h2 == NULL`, or `h2`/`h1` shorter than 3 bytes when the corresponding bytes are
actually reached. The C dereferences `h2[0]` unconditionally, so a null `h2`
segfaults; the Rust translation reproduces that same unconditional dereference
(deliberately — a silent null check would *diverge* from the C). Row 10/11 cover
the null/short case that the C *does* define, via short-circuiting.

## Phase C check-off

| # | test | status |
|---|------|--------|
| 1 | `err_row01_bad_sync_byte0` | [x] pass |
| 2 | `err_row02_byte1_neither_class` | [x] pass |
| 3 | `err_row03_byte1_reserved_layer` | [x] pass |
| 4 | `err_row04_byte2_bitrate_15` | [x] pass |
| 5 | `err_row05_byte2_samplerate_3` | [x] pass |
| 6 | `err_row06_invalid_h2_any_h1` | [x] pass |
| 7 | `err_row07_byte1_mismatch_above_bit0` | [x] pass |
| 8 | `err_row08_byte2_samplerate_mismatch` | [x] pass |
| 9 | `err_row09_freeformat_nibble_mismatch` | [x] pass |
| 10 | `err_row10_null_h1_with_invalid_h2` | [x] pass |
| 11 | `err_row11_unreadable_h1_with_invalid_h2` | [x] pass |
| 12 | `err_row12_aliased_pointers` | [x] pass |
| 13 | `err_row13_no_overread_past_three_bytes` | [x] pass |
| — | `err_generic_boundaries_and_out_of_range_field_values` (generic boundaries: every byte position swept 0..=255, one step past every field boundary, all-`0x00`/all-`0xff`) | [x] pass |

All 13 rows live in `tests/phase_c_errors.rs`. Row 13's guard page is
**self-validating**: a forked child reads the first guard byte and the test
asserts it dies with `SIGSEGV`/`SIGBUS`, so the over-read check cannot pass
vacuously.
