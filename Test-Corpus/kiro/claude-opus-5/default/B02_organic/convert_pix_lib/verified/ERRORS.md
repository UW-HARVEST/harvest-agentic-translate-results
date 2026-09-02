# ERRORS.md — error-surface table

Derived mechanically from
`grep -n "return\|assert\|cp_error_reason\|goto\|default:" c_src/src/lib.c`.
Every distinct rejection / error branch in `c_src/src/lib.c` gets one row.

Only `cp_inflate` and `convert_pix` are reachable across the ABI, so the
"expected C result" column states the value seen by the caller of the exported
function plus the resulting `cp_error_reason` string.

## Build configuration matters

`c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and never defines `NDEBUG`, so
the reference `.so` built by the command in the task has **`assert()` live** —
confirmed by `__assert_fail@GLIBC_2.2.5` in its undefined symbols. A failing
assertion calls `abort()`, which is observable behaviour on malformed input.

The Rust translation therefore reproduces the assertions under the default
`c_asserts` feature; with `panic = "abort"` it dies with `SIGABRT` on exactly the
same inputs. `--no-default-features` drops them, matching a C build with
`-DNDEBUG` (`c_ndebug_build`, produced by `run_all.sh`). Every row below is
verified against **both** pairings.

Assert rows cannot be compared in-process (the first abort would kill the test
runner), so they are compared in child processes: `tests/phase_c_subproc.rs`
runs each case once per implementation and requires the same sequence of result
lines *and* the same exit status / signal.

## Reachable error returns

| # | function (C line) | trigger (exact invalid input/condition) | expected C result | test |
|---|-------------------|------------------------------------------|-------------------|------|
| E1 | `cp_stored` L169-176 | stored block (`btype==0`) whose `LEN != (uint16_t)~NLEN` | `cp_inflate` → `0`; `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `phase_c_errors::e1_stored_len_nlen_mismatch` (8 LENs × 7 ways of breaking the complement × 4 input alignments) |
| E2 | `cp_stored` L178-184 | after reading LEN/NLEN, `s->bits_left / 8 > (int)LEN` — i.e. **more** input bytes remain than the stored length. Any stored block that is not the last thing in the stream trips this. | `cp_inflate` → `0`; `cp_error_reason` = `"Stored block extends beyond end of input stream."` | `phase_c_errors::e2_stored_extends_beyond_input`, and rows C15/C16 in `phase_b_valid` |
| E3 | `cp_block` L252-260 | literal symbol (`<256`) decoded when `s->out + 1 > s->out_end` | `cp_inflate` → `0`; `"Attempted to overwrite out buffer while outputting a symbol."` | `phase_c_errors::e3_literal_overruns_out` (fixed **and** dynamic blocks, every `out_bytes` from 0 to n-1, 4 alignments) |
| E4 | `cp_block` L272-279 | length/distance pair with `s->out - backwards_distance < s->begin` | `cp_inflate` → `0`; `"Attempted to write before out buffer (invalid backwards distance)."` | `phase_c_errors::e4_distance_before_out_begin` (match as the first symbol for all 30 distance symbols, plus `dist == emitted + 1`) |
| E5 | `cp_block` L281-288 | length/distance pair with `s->out + length > s->out_end` | `cp_inflate` → `0`; `"Attempted to overwrite out buffer while outputting a string."` | `phase_c_errors::e5_string_overruns_out`, and row C40's one-byte-short cases |
| E6 | `cp_inflate` L355-360 | block header with `btype == 3` (bits `11`) | `cp_inflate` → `0`; `"Detected unknown block type within input stream."` | `phase_c_errors::e6_unknown_block_type` (both `bfinal` values, 12 input lengths, 4 alignments, 3 out sizes, plus btype 3 as a second block) |
| E7 | `cp_inflate`, input exhausted | truncated stream. With asserts live `cp_read_bits`' `assert(s->bits_left > 0)` / `assert(!cp_would_overflow(...))` aborts (rows A5/A7); with `-DNDEBUG` the reads return zero bits, the next header decodes as `bfinal=0, btype=0`, and `cp_stored` sees `LEN=0, NLEN=0` → E1. Either way it terminates. | abort (asserts) / `0` + E1's string (NDEBUG) | `phase_c_subproc::truncated_streams` (every truncation of fixed and dynamic streams, plus shrunken `in_bytes`), `stored_block_truncated_and_oversized_len` |
| E8 | `cp_inflate` L308 | `in_bytes == 0`: `bits_left = 0`, `word_count = 0`, `final_word_available = 0` → first `cp_read_bits` has no input | abort via A5 (asserts) / `0` + E1's string (NDEBUG) | `phase_c_subproc::a5_zero_and_negative_in_bytes` |

## `convert_pix` silent-rejection rows (no error channel)

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| E10 | `convert_pix` L477 `switch (bpp)` — no `default:` | `bpp` not in `{1,2,3,4}`. A C `int` accepts any value, so this is the out-of-range-enum case an FFI caller can produce: 0, 5..1000, 65536, negatives, `INT_MAX`, `INT_MIN`. | nothing written to `dst`, `dst` never advanced; `src` still advanced by `1 + w*bpp` per row. `void` | `phase_c_errors::e10_convert_pix_out_of_range_bpp` (19 values × 3 sizes, plus the four extremes) |
| E11 | `convert_pix` L474 outer `for` | `h <= 0` (incl. `INT_MIN`) | no iteration at all; `dst`/`src` untouched, NULL pointers are safe | `phase_c_errors::e11_e12_convert_pix_empty_dims`, row C12 |
| E12 | `convert_pix` L476 inner `for` | `w <= 0` (incl. `INT_MIN`) with `h > 0` | per row only `src++`; nothing written to `dst` | `phase_c_errors::e11_e12_convert_pix_empty_dims`, row C12 |

## Assert rows — the C `abort()`s (compared via exit status)

| # | function (C line) | trigger | expected C result | test |
|---|-------------------|---------|-------------------|------|
| A1 | `cp_ptr` L89 | `cp_stored` reaches `cp_ptr` while `s->bits_left & 7` (not byte aligned) | `abort()` | `phase_c_subproc::stored_block_truncated_and_oversized_len` |
| A2 | `cp_peak_bits` L98 | `s->word_index > s->word_count` | `abort()` — unreachable, guarded by the enclosing `if` | translated; unreachable by construction |
| A3 | `cp_consume_bits` L109 | `s->count < num_bits_to_read`, e.g. `cp_decode` consuming `key & 0xF` bits from a truncated stream | `abort()` | `phase_c_subproc::truncated_streams`, `fuzz_*` |
| A4 | `cp_read_bits` L117/118 | `num_bits_to_read > 32` or `< 0` | `abort()` — unreachable, all call sites pass 0..16 | translated; unreachable by construction |
| A5 | `cp_read_bits` L119 | `s->bits_left <= 0` (input exhausted, `in_bytes <= 0`) | `abort()` | `phase_c_subproc::a5_zero_and_negative_in_bytes`, `a5_null_input_pointer` |
| A6 | `cp_read_bits` L120 | `s->count > 64` | `abort()` | `phase_c_subproc::fuzz_*` |
| A7 | `cp_read_bits` L121 | `cp_would_overflow(s, n)`: `(bits_left + count) - n < 0` | `abort()` | `phase_c_subproc::truncated_streams`, `fuzz_*` |
| A8 | `cp_build` L148 | a code length `>= 16` reaches `cp_build` (mutated `cp_fixed_table`, or a malformed dynamic block whose garbage tree yields a symbol `>= 16`) | `abort()` | `phase_c_subproc::a8_fixed_table_with_oversized_code_length` (6 bad lengths × 10 table positions). See **U6** for the `-DNDEBUG` case. |
| A9 | `cp_decode` L211 | the decoded prefix does not match the tree entry: incomplete or over-subscribed Huffman code, or `hi == 0` so `tree[-1]` is read | `abort()` | `phase_c_subproc::a9_incomplete_and_oversubscribed_literal_codes`, `a9_empty_distance_tree_with_match`, `a9_empty_literal_tree` |

`tree[-1]` reads are *deterministic*: the Rust `cp_state_t` is `#[repr(C)]` with
the same field order as the C struct and is likewise zero-initialised, so
`lit[-1]` is the tail of `lookup`, `dst[-1]` is `lit[287]` and `len[-1]` is
`dst[31]` in both. The `a9_empty_*` tests confirm this.

## Undefined-behaviour rows

| # | function | trigger | status |
|---|----------|---------|--------|
| U1 | `cp_dynamic` L219 | code-length symbol `16` decoded at `n == 0` → reads `lens[-1]`, uninitialised C stack | **Tested.** `phase_c_subproc::u1_code_length_symbol_16_first`: the resulting all-zero literal code makes `cp_decode` run with `hi == 0`, whose assert fails, so both implementations abort identically for every variant tried (4 repeat counts × 3 filler bytes × 2 padding lengths). |
| U2 | `cp_dynamic` L222 | a repeat code overshoots `nlit + ndst` → writes past `lens[319]`, smashing the C's own stack frame | **Not comparable.** Observed: the C corrupts its loop bounds and spins until killed, while the Rust (whose backing array is padded) aborts later on an assert. No defined C result exists. Reached only by corrupting a dynamic block's *header*; `fuzz_bitflipped_dynamic_payloads` therefore corrupts only the payload, and `fuzz_bitflipped_dynamic_headers` is `#[ignore]`d with this explanation. |
| U3 | `cp_stored` L188 | `memcpy(s->out, p, LEN)` with no check that `s->out + LEN <= s->out_end`, and `p` (from `cp_ptr`) can point outside the declared input | **Tested.** `tests/common`'s `AlignedBuf` surrounds every input with 64 bytes of zeros in front and 64 KiB behind, so the C's over-reads are deterministic in both processes; `out_bytes` is always ≥ `LEN`. Covered by `stored_block_truncated_and_oversized_len` (LEN up to 0x0FFF larger than the payload). |
| U4 | `cp_inflate` L308-323 | `in_bytes < 0` → negative `bits_left` / `word_count` | **Tested.** Trips A5 in both; `a5_zero_and_negative_in_bytes` covers `-1 .. INT_MIN+1`. |
| U5 | `cp_inflate` / `convert_pix` | NULL `in` / `out` / `src` / `dst` | **Partially tested.** NULL `in` (rows A5) and NULL `out` with `out_bytes == 0` or an empty block (`null_output_pointer_without_writes`) are compared. NULL `out` with a literal to store would have the C write to address 0; that dereference is excluded. NULL `src`/`dst` *are* compared for `convert_pix` whenever no dereference happens (E10-E12). |
| U6 | `cp_build` L136 | `counts[lens[n]]++` (and later `codes[len]`, `first[len]`) with `lens[n] >= 16` — out-of-bounds access on three `int[16]` stack arrays | **Split by configuration.** With asserts live the C reaches `assert(len < 16)` and aborts, and so does the Rust (row A8, tested). With `-DNDEBUG` the C keeps running on a corrupted frame and eventually segfaults; no defined result exists, so `a8_*` is `#[ignore]`d in that configuration and `fuzz_bitflipped_fixed_streams` keeps byte 0 intact so the stream cannot wander into `cp_dynamic` and produce a length `>= 16`. The Rust widens the three arrays to 256 entries so it absorbs the same index rather than faulting on a bounds check — otherwise the Rust would abort on inputs the C survives. |

## Dead-code rows (`static`, never called — unreachable through the ABI)

`cp_paeth`, `cp_make32`, `cp_chunk`, `cp_find` and `cp_unfilter` are `static` and
nothing in `lib.c` calls them, so no differential test can exist. They are
translated for completeness.

| # | function (C line) | trigger | expected C result |
|---|-------------------|---------|-------------------|
| D1 | `cp_unfilter` L433-434 | first row's filter byte `> 4` | `return 0` |
| D2 | `cp_unfilter` L467-468 | any later row's filter byte `> 4` | `return 0` |
| D3 | `cp_chunk` L397 | `memcmp(start+4, chunk, 4) != 0`, or `len < minlen`, or `png->p + len + 12 > png->end` | `return NULL` |
| D4 | `cp_find` L409 | chunk not found before `png->end` | `return NULL` |

## Checklist

Every row has a passing differential test, or an explicit reason why none can
exist. Verified under **both** feature combinations (see `run_all.sh`).

- [x] E1 [x] E2 [x] E3 [x] E4 [x] E5 [x] E6 [x] E7 [x] E8
- [x] E10 [x] E11 [x] E12
- [x] A1 [x] A3 [x] A5 [x] A6 [x] A7 [x] A8 (asserts config) [x] A9
- [x] A2, A4 — unreachable from any call site; translated verbatim
- [x] U1 [x] U3 [x] U4 [x] U6 (asserts config)
- [x] U2 — documented as C stack corruption, no defined result (`#[ignore]`d test kept)
- [x] U5 — compared wherever no NULL dereference occurs
- [x] D1-D4 — unreachable dead `static` code
- [x] Generic boundaries: NULL pointers, `in_bytes` 0 / negative / shrunken,
      `out_bytes` 0 and one-byte-short, oversized `LEN`, out-of-range "enum"
      values (`bpp`, and distance symbols 30/31 whose `cp_dist_base` entry is 0)
