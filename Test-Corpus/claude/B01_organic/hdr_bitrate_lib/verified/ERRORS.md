# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Derived mechanically from the C source. Grep for every rejection construct:

```
$ grep -nE 'return|assert|NULL|errno|-1|RETURN_ERROR|if|switch|#if|enum|goto' \
      c_src/src/lib.c c_src/include/lib.h
c_src/src/lib.c:12:    return 2 *
```

**The C function contains exactly one `return` statement and NO other listed
construct**: no `if`, no `switch`, no `assert`, no `goto`, no error enum, no
`RETURN_ERROR` macro, no `return -1`, no `return NULL`, no null check, no
explicit range check, no min/max constant, no `#ifdef`.

`hdr_bitrate` is therefore **totally branch-free and has an EMPTY explicit
error surface**: every input, valid or invalid, flows through the same single
expression and produces an `unsigned` return value. There is no in-band error
code and no sentinel.

Consequently the rows below are the *implicit* rejection/degenerate paths — the
ways the C handles input it has no valid table entry for. These are real inputs
an external caller can pass, so each still gets a differential test asserting C
and Rust return the **same** value.

## The single expression

```c
return 2 * halfrate[!!((h[1]) & 0x8)][(((h[1]) >> 1) & 3) - 1][((h[2]) >> 4)];
```

with `static const uint8_t halfrate[2][3][15]`. Index derivation:

| index | expression | range the expression can produce | valid range for the declared array |
|-------|-----------|----------------------------------|-----------------------------------|
| `i` | `!!(h[1] & 0x8)` | `0..=1` | `0..=1` — always in range |
| `j` | `((h[1] >> 1) & 3) - 1` | `-1..=2` | `0..=2` — **`-1` is out of range** |
| `k` | `h[2] >> 4` | `0..=15` | `0..=14` — **`15` is out of range** |

Because C multidimensional array indexing is flat, the read lands at byte
offset `flat = i*45 + j*15 + k` from the base of the 90-byte table. Only
`flat` outside `0..=89` is a genuine out-of-bounds read; an out-of-range `j` or
`k` alone frequently still lands *inside* the table (on a neighbouring row).

## ERROR-SURFACE TABLE

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `hdr_bitrate` | `h[2] >> 4 == 15` (invalid/`bad` bitrate index, one past the last row entry) **and** `(i,j) != (1,2)` — i.e. `flat` lands on the first byte of the next row, which is `0` in every row | `0` (reads a neighbouring row's leading `0`) |
| E2 | `hdr_bitrate` | `h[2] >> 4 == 15` **and** `i==1, j==2` (`h[1] & 0x0E == 0x0E`) → `flat == 90`, one byte **past the end of the whole table** | `0` in this build (reads `.rodata` padding after the table). **True UB.** |
| E3 | `hdr_bitrate` | `(h[1] >> 1) & 3 == 0` (reserved/invalid layer field → `j == -1`) **and** `i == 0` (`h[1] & 0x0E == 0`) → `flat == k-15`, i.e. `-15..=-1` for `k<15`: reads **before the start of the table** | `0` in this build (reads `.rodata`/segment padding in front of the table). **True UB.** |
| E4 | `hdr_bitrate` | `(h[1] >> 1) & 3 == 0` (`j == -1`) **and** `i == 1` (`h[1] & 0x0E == 0x08`) → `flat == 30+k`, which is **still inside** the table: aliases `halfrate[0][2][k]` | `2 * halfrate[0][2][k]`, i.e. `{0,32,48,56,64,80,96,112,128,144,160,176,192,224,256}` — *not* an error value |
| E5 | `hdr_bitrate` | `(h[1] >> 1) & 3 == 0` and `i == 0` and `k == 15` → `flat == 0` exactly: aliases `halfrate[0][0][0]` | `0` |
| E6 | `hdr_bitrate` | `h[2] >> 4 == 0` ("free"/`0` bitrate index — a *valid* index whose table entry is `0`) | `0` for every `(i,j)` (all 6 rows begin with `0`) |
| E7 | `hdr_bitrate` | `h == NULL` (no null check exists in the C) | **segfault (SIGSEGV)** — the C unconditionally dereferences `h[1]`/`h[2]`. Not a returnable error. Verified to fault identically in Rust. |
| E8 | `hdr_bitrate` | `h` points to a buffer shorter than 3 bytes (the C reads `h[1]` and `h[2]` with no length parameter and no bound) | reads out of the caller's buffer; **no rejection** — returns whatever the indices select. Verified with a 3-byte-exact allocation at a page boundary. |
| E9 | `hdr_bitrate` | out-of-range "enum-like" field values crossing the FFI boundary: the layer field `(h[1]>>1)&3` and the bitrate index `h[2]>>4` have no valid-variant validation; every one of the `2 * 4 * 16 = 128` index triples is accepted | never rejected — all 128 covered exhaustively; C and Rust must agree on all |
| E10 | `hdr_bitrate` | `h[0]`, and `h[3]`+ set to arbitrary/garbage values (never read by the C) | must **not** influence the result |

### Notes on E2 / E3 (the genuine UB rows)

E2 and E3 are the only inputs where the compiled C actually reads memory
outside the `halfrate` object. Their result is *not* guaranteed by the C
standard; it is a property of the build's `.rodata` layout. Both were probed
empirically and return `0` for all 7680 (E3) + 512 (E2) input pairs. The Rust
translation reproduces this by storing the flat table with 15 bytes of zero
padding on each side, so it returns `0` for the same inputs **without**
performing an out-of-bounds access in Rust.

Because this is the one genuinely build-dependent part of the translation, the
stability of the behaviour was measured rather than assumed. The **entire**
suite (including the exhaustive 65536-input test) was re-run against seven
independently built C shared objects via the `HDR_C_SO` override:

| C build | flat `-15` | flat `90` | full suite |
|---------|-----------|----------|------------|
| `cmake` default (the prescribed build) | `0` | `0` | 28/28 pass |
| `gcc -O0` | `0` | `0` | 28/28 pass |
| `gcc -O1` | `0` | `0` | 28/28 pass |
| `gcc -O2` | `0` | `0` | 28/28 pass |
| `gcc -O3` | `0` | `0` | 28/28 pass |
| `gcc -Os` | `0` | `0` | 28/28 pass |
| `clang -O0` | `0` | `0` | 28/28 pass |
| `clang -O2` | `0` | `0` | 28/28 pass |

So the `0` observation is stable across optimisation level and compiler, not an
artifact of one build. It remains, formally, UB in the C: a toolchain that
placed non-zero bytes immediately around the table would make E2/E3 diverge.
That would be a property of the C's undefined behaviour, not a defect in the
Rust translation, and is recorded here deliberately.

## Checklist

- [x] E1 — covered by `err_e1_bitrate_index_15_neighbour_row`
- [x] E2 — covered by `err_e2_bitrate_index_15_past_end_of_table`
- [x] E3 — covered by `err_e3_reserved_layer_reads_before_table`
- [x] E4 — covered by `err_e4_reserved_layer_aliases_row_0_2`
- [x] E5 — covered by `err_e5_reserved_layer_k15_aliases_first_entry`
- [x] E6 — covered by `err_e6_free_bitrate_index_zero`
- [x] E7 — covered by `err_e7_null_pointer_faults_in_both` (subprocess, both SIGSEGV)
- [x] E8 — covered by `err_e8_short_buffer_no_bounds_check`
- [x] E9 — covered by `err_e9_all_128_index_triples` + exhaustive test
- [x] E10 — covered by `err_e10_unread_bytes_do_not_matter`
