# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c` (13 lines) and
`c_src/include/lib.h`.

## Inventory of rejection mechanisms found in the C source

The C library has **no** `RETURN_ERROR` macros, **no** `assert`, **no** error
enums, **no** `return NULL`, **no** `-1` sentinels, and **no** explicit
null-pointer checks. Its *entire* rejection surface is expressed as
short-circuit `&&` / `||` conditions inside two boolean expressions whose
falsity makes `hdr_compare` return the sentinel **`0`** ("headers do not
match"); success is **`1`**.

Every distinct sub-condition that can independently drive the result to `0` is
one row below. `hdr_valid` (`lib.c:3-7`) contributes rows 1-5; `hdr_compare`
(`lib.c:9-13`) contributes rows 6-8. Rows 9-11 are the generic
FFI-boundary boundaries mandated by Phase C.

There are no enum parameters in this API, so "out-of-range enum value" degrades
to "arbitrary out-of-range `uint8_t` payload bytes", covered exhaustively /
randomly by rows 1-8 and by the full-byte sweeps in Phase B.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `hdr_valid` via `hdr_compare` (`lib.c:4`, 1st conjunct) | `h2[0] != 0xff` — sync byte wrong (any of the 255 non-`0xff` values) | `hdr_compare` returns `0` |
| 2 | `hdr_valid` via `hdr_compare` (`lib.c:4`, 2nd conjunct) | `(h2[1] & 0xF0) != 0xf0` **and** `(h2[1] & 0xFE) != 0xe2` — second sync nibble is neither `0xFx` nor `0xE2/0xE3` | `0` |
| 3 | `hdr_valid` via `hdr_compare` (`lib.c:5`, 3rd conjunct) | `((h2[1] >> 1) & 3) == 0` — reserved "layer" field is 0 (e.g. `h2[1] == 0xf0`, `0xf1`, `0xf8`, `0xf9`) | `0` |
| 4 | `hdr_valid` via `hdr_compare` (`lib.c:5`, 4th conjunct) | `(h2[2] >> 4) == 15` — high nibble of `h2[2]` is `0xF` (bitrate index 15) | `0` |
| 5 | `hdr_valid` via `hdr_compare` (`lib.c:6`, 5th conjunct) | `((h2[2] >> 2) & 3) == 3` — reserved sample-rate index 3 | `0` |
| 6 | `hdr_compare` (`lib.c:10`, 2nd conjunct) | `h2` is valid but `((h1[1] ^ h2[1]) & 0xFE) != 0` — version/layer/protection bits differ between the two headers | `0` |
| 7 | `hdr_compare` (`lib.c:11`, 3rd conjunct) | `h2` valid, `h1[1]`/`h2[1]` compatible, but `((h1[2] ^ h2[2]) & 0x0C) != 0` — sample-rate index differs | `0` |
| 8 | `hdr_compare` (`lib.c:12`, 4th conjunct) | `h2` valid and previous conjuncts hold, but exactly one of `(h1[2] & 0xF0) == 0` / `(h2[2] & 0xF0) == 0` is true — one header is "free bitrate", the other is not | `0` |
| 9 | `hdr_compare` — generic FFI boundary | `h2` points at a buffer whose 3rd byte is the last readable byte (exactly 3 bytes) — must not read `h2[3..]`; and `h1` read must not exceed `h1[2]` | same value as for a longer buffer; no over-read (verified with 3-byte-exact allocations) |
| 10 | `hdr_compare` — generic FFI boundary, null `h1` | `h1 == NULL` while `h2` is **invalid** — C `&&` short-circuits on `hdr_valid(h2)` and never dereferences `h1` | `0`, no crash (Rust must short-circuit identically) |
| 11 | `hdr_compare` — generic FFI boundary, unread tail bytes | trailing bytes `h1[3]`, `h2[3]`, … set to arbitrary/`0xFF` garbage, and `h1[0]` set to arbitrary garbage (`h1[0]` is never read by the C) | result independent of those bytes; C and Rust agree |

## Checklist

- [x] 1 — `h2[0] != 0xff`
- [x] 2 — `h2[1]` fails both sync-nibble alternatives
- [x] 3 — `((h2[1] >> 1) & 3) == 0`
- [x] 4 — `(h2[2] >> 4) == 15`
- [x] 5 — `((h2[2] >> 2) & 3) == 3`
- [x] 6 — `((h1[1] ^ h2[1]) & 0xFE) != 0`
- [x] 7 — `((h1[2] ^ h2[2]) & 0x0C) != 0`
- [x] 8 — free-bitrate mismatch on `& 0xF0`
- [x] 9 — 3-byte-exact buffers, no over-read
- [x] 10 — null `h1` short-circuit
- [x] 11 — garbage in never-read bytes (`h1[0]`, `h1[3+]`, `h2[3+]`)

All rows are covered by `translation/tests/error_paths.rs`; every test compares
the C `.so` and the Rust `.so` return values for **exact equality of the
returned `int`**, not merely "both non-success".
