# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep result

```sh
grep -nE 'return|assert|NULL|errno|RETURN_ERROR|if|switch|<|>|\?|enum|#define|#if' \
     src/lib.c include/lib.h
```

Matches: only `src/lib.c:12 return 2 *` and the index expression on line 13,
plus `#include <stdint.h>` in the header.

Consequences, stated precisely:

* There are **zero** error-return macros (`RETURN_ERROR`, `goto fail`, ...).
* There are **zero** `return -1` / `return NULL` / error-enum returns — the
  function has exactly **one** `return` statement and it is unconditional.
* There are **zero** `assert`s (`assert.h` is not included).
* There are **zero** explicit range checks, null checks, `if`, `switch`,
  ternary, or `#ifdef` branches.
* There are **zero** `#define`d MIN/MAX constants.
* The return type is `unsigned`, which carries no reserved sentinel value; the
  full range of the return is legitimate output.

Therefore the C API **cannot reject any input**. Every 3-byte input produces a
value. The "error surface" of this library is entirely *implicit*: the two index
expressions can leave the declared bounds of `halfrate[2][3][15]`, and a null /
under-length pointer is dereferenced without a check.

Rows below are the distinct implicit rejection/failure conditions that actually
exist in the C, each with the result the C build actually produces (measured
against the built `.so`, not guessed). The Rust must reproduce each one
identically.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `hdr_bitrate` | `h == NULL` — the only unchecked pointer deref (`h[1]`, `h[2]`); there is no null guard | Not an error return: the process faults. `SIGSEGV` (signal 11), no value returned. Rust must fault identically. |
| E2 | `hdr_bitrate` | layer field `(h[1] >> 1) & 3 == 0` (the reserved MPEG layer value) makes the middle index `-1`, i.e. `j = -1`, out of the declared range `0..2`. Combined with `!!(h[1] & 0x8) == 0` this yields flat offsets `-15 .. 0`, i.e. reads *before* the table. | No error: reads the 15 bytes preceding `halfrate`. In this build `.rodata` begins at a page boundary (`0x2000`) so offsets `-15..-1` are zero page-padding ⇒ returns `0`. Offset `0` is `halfrate[0][0][0] == 0` ⇒ also `0`. |
| E3 | `hdr_bitrate` | layer field `== 0` (`j = -1`) with version bit set (`h[1] & 0x8 != 0`, `i = 1`) ⇒ flat offsets `30 .. 45`. These are out of the *declared* subarray bounds but still inside the 90-byte table: they alias row `halfrate[0][2][*]` (and offset 45 aliases `halfrate[1][0][0]`). | No error: returns `2 * halfrate_flat[30 + k]`, i.e. the `halfrate[0][2]` row: `0,32,48,56,64,80,96,112,128,144,160,176,192,208,224,256` for `k = 0..15`. Must **not** be "fixed" to an error. |
| E4 | `hdr_bitrate` | bitrate nibble `h[2] >> 4 == 15` (the MPEG "bad" bitrate index) makes the last index `15`, one past the declared `0..14`. | No error: reads the byte after the 15-byte row. For `(i,j)` other than `(1,2)` this aliases the first byte of the next row; every such byte is `0` ⇒ returns `0`. |
| E5 | `hdr_bitrate` | maximal index combination: `i = 1`, `j = 2`, `k = 15` ⇒ flat offset `90`, one byte *past the end of the whole table* (table occupies `.rodata` `0x2000..0x205A`). | No error: reads `0x205A`, which is section alignment padding before `.eh_frame_hdr` ⇒ `0`. Returns `0`. |
| E6 | `hdr_bitrate` | buffer shorter than 3 bytes (length 0, 1, or 2) — there is no length parameter and no length check, so `h[1]`/`h[2]` read past the end. | No error and no validation. The C reads exactly bytes `h[1]` and `h[2]` and nothing else; if those bytes are unmapped the process faults (`SIGSEGV`). With a 3-byte buffer the C never touches `h[0]` or `h[3]`. Rust must read exactly the same two bytes. |
| E7 | `hdr_bitrate` | out-of-range "enum-like" field values crossing the FFI boundary: the version/layer/bitrate fields are bit-fields with no valid-variant validation, so *every* one of the 256 values of `h[1]` and 256 values of `h[2]` is a legal input, including the reserved/`bad`/`free` encodings (`layer = 0b00`, `bitrate = 0b1111`, `bitrate = 0b0000`). C performs no variant check. | No error for any of the 65 536 `(h[1], h[2])` combinations; each produces a defined `unsigned`. Rust must match all 65 536 byte-for-byte. |

## Verification gate

Each row has a differential test in `translation/tests/differential.rs`:

| row | test |
|-----|------|
| E1 | `e1_null_pointer_same_fault` (subprocess, compares termination signal) |
| E2 | `e2_reserved_layer_negative_index_low_version` |
| E3 | `e3_reserved_layer_negative_index_high_version` |
| E4 | `e4_bad_bitrate_nibble_15` |
| E5 | `e5_max_index_past_end_of_table` |
| E6 | `e6_reads_exactly_three_bytes` (guard-page mmap: byte 3 onward unmapped) |
| E7 | `e7_exhaustive_all_header_bytes` (all 65 536 `(h[1], h[2])`) |

- [x] E1 checked
- [x] E2 checked
- [x] E3 checked
- [x] E4 checked
- [x] E5 checked
- [x] E6 checked
- [x] E7 checked
