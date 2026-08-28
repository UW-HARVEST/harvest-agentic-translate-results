# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c` by grepping for **every** place the
code rejects input, asserts, or indexes/writes without a bound check:

```
grep -n 'cp_error_reason =\|return 0;\|return NULL\|assert(\|\[\(symbol\|distance_symbol\|n\)\]' c_src/src/lib.c
```

The C library has no error enum.  It signals failure four ways:

1. `cp_inflate` returns `0` (success `1`) and leaves a diagnostic in the global
   `const char *cp_error_reason`;
2. internal `static` helpers return `0`;
3. a failed `assert()` — the CMake build defines no `NDEBUG`, so asserts are
   **live** — prints
   `"<prog>: <abs-path>/lib.c:<line>: <func>: Assertion `<expr>' failed."` and
   raises `SIGABRT`;
4. it does something undefined (an unchecked index or an overflowing local
   array).  Those are rows 30–36; they are *reproduced*, not fixed.

`convert_pix` contains **no** validation whatsoever (rows 22–23).

How each row is tested:

* rows that **return** → `tests/errors.rs` (in-process, both `.so`s called back
  to back, return value + whole output buffer + `cp_error_reason` string
  compared);
* rows that **abort / hang** → `tests/aborts.rs` and
  `tests/dynamic_overshoot.rs`, which run each scenario in a *child process*
  (once per library) and compare `(exit code, signal, stderr assertion text,
  output-buffer hash)`;
* rows that are **unchecked indexing** → `tests/oob_tables.rs` and
  `tests/dynamic_overshoot.rs`.

## Table

| #  | function | trigger (exact invalid input / condition) | expected C result | [x] | test |
|----|----------|-------------------------------------------|-------------------|-----|------|
| 1  | `cp_stored` (line 170) | stored block whose `LEN != (uint16_t)~NLEN` | `cp_inflate` → `0`, `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | [x] | `errors::err01_stored_len_nlen_mismatch` |
| 2  | `cp_stored` (line 179) | stored block where `s->bits_left / 8 > LEN`, i.e. **more** input bytes remain than `LEN` (the C's test is `<=`, the inverse of the intuitive one, so a stored block followed by *anything* is rejected) | `0`, `"Stored block extends beyond end of input stream."` | [x] | `errors::err02_stored_extends_beyond`, `errors::err30_stored_len_off_by_one` |
| 3  | `cp_block` (line 254) | literal decoded while `s->out + 1 > s->out_end` (output full, `out_bytes == 0`, or `out_bytes < 0`) | `0`, `"Attempted to overwrite out buffer while outputting a symbol."` | [x] | `errors::err03_out_full_on_literal`, `errors::err24_out_bytes_zero_and_negative`, `errors::err31_literal_at_exact_boundary` |
| 4  | `cp_block` (line 273) | length/distance pair with `s->out - backwards_distance < s->begin` | `0`, `"Attempted to write before out buffer (invalid backwards distance)."` | [x] | `errors::err04_bad_backwards_distance` |
| 5  | `cp_block` (line 282) | length/distance pair with `s->out + length > s->out_end` | `0`, `"Attempted to overwrite out buffer while outputting a string."` | [x] | `errors::err05_out_full_on_string`, `oob_tables::oob03_empty_distance_tree_out_too_small` |
| 6  | `cp_inflate` (line 356) | `btype == 3` (bits `11` after BFINAL) | `0`, `"Detected unknown block type within input stream."` | [x] | `errors::err06_btype_3_unknown_block`, `inflate::i27_stored_not_final` |
| 7  | `cp_unfilter` (line 434) | row-0 filter byte `> 4` | `return 0` | [x] | unreachable: `cp_unfilter` is `static` and never called (absent from `nm -D` of *both* objects). Documented; no test possible without changing the C. |
| 8  | `cp_unfilter` (line 468) | row-`y` (`y >= 1`) filter byte `> 4` | `return 0` | [x] | unreachable, as row 7 |
| 9  | `cp_chunk` (line 397) | `memcmp(start+4, chunk, 4) != 0`, or `len < minlen`, or `png->p + len + 12 > png->end` | `return NULL` | [x] | unreachable: `static`, never called |
| 10 | `cp_find` (line 409) | no matching chunk before `png->end` | `return NULL` | [x] | unreachable: `static`, never called |
| 11 | `cp_ptr` (line 89) | `assert(!(s->bits_left & 7))`.  **Reachable.**  `cp_stored` byte-aligns using `s->count & 7`, which only tracks the real bit position while every refill added a multiple of 8 bits.  The `final_word` branch of `cp_peak_bits` adds `s->bits_left` instead, so if it fires at a bit position that is not byte aligned, `count` and `bits_left` drift apart.  Concrete input: bytes `02 E4 FF 1F 00` at a pointer `≡ 2 (mod 4)` — an empty fixed block followed by a stored block; `bits_left` ends up `-5`, and `-5 & 7 == 3`. | `SIGABRT` + `Assertion '!(s->bits_left & 7)' failed.` | [x] | `aborts::abort11_cp_ptr_unaligned` |
| 12 | `cp_peak_bits` (line 98) | `assert(s->word_index <= s->word_count)` | unreachable — the increment happens inside `if (s->word_index < s->word_count)`, so the post-condition always holds | [x] | proven unreachable by inspection; no test |
| 13 | `cp_consume_bits` (line 109) | `assert(s->count >= num_bits_to_read)` — consume more bits than are buffered.  Concrete input: `01 00` (a 2-byte stored block header): after `LEN` is read, `count == 8`, no words remain and `final_word` is spent, so the 16-bit `NLEN` read cannot be satisfied. | `SIGABRT` + `Assertion 's->count >= num_bits_to_read' failed.` | [x] | `aborts::abort13_consume_more_than_buffered`, hit by all three fuzz sweeps |
| 14 | `cp_read_bits` (line 117) | `assert(num_bits_to_read <= 32)` — reachable by writing `> 32` into the exported, writable `cp_len_extra_bits` / `cp_dist_extra_bits` | `SIGABRT` + `Assertion 'num_bits_to_read <= 32' failed.` | [x] | `aborts::abort14_read_bits_gt_32` |
| 15 | `cp_read_bits` (line 118) | `assert(num_bits_to_read >= 0)` | unreachable — every argument is a literal `1,2,3,4,5,7,16`, `s->count & 7`, or a `uint8_t` table entry; all `>= 0` | [x] | proven unreachable by inspection; no test |
| 16 | `cp_read_bits` (line 119) | `assert(s->bits_left > 0)` — input exhausted.  `cp_inflate(in, 0, …)` trips it before the first bit is read. | `SIGABRT` + `Assertion 's->bits_left > 0' failed.` | [x] | `aborts::abort16_in_bytes_zero`, `aborts::abort26_null_in`, `aborts::abort25_in_bytes_negative`, all fuzz sweeps |
| 17 | `cp_read_bits` (line 120) | `assert(s->count <= 64)` | unreachable — `cp_peak_bits` only refills when `count < num_bits_to_read <= 32`, so `count <= 32 + 32 = 64` afterwards | [x] | proven unreachable for the arguments the code uses; no test |
| 18 | `cp_read_bits` (line 121) | `assert(!cp_would_overflow(s, n))`, i.e. `(bits_left + count) - n < 0`.  Reachable both from truncated streams and deterministically by setting `cp_len_extra_bits` to `30` and offering a 4-byte fixed block with one match (`bits_left == count == 14`, `28 - 30 < 0`). | `SIGABRT` + `Assertion '!cp_would_overflow(s, num_bits_to_read)' failed.` | [x] | `aborts::abort18_would_overflow`, hit by the unstructured + mutated fuzz sweeps |
| 19 | `cp_build` (line 148) | `assert(len < 16)` — a code-length entry `>= 16`.  Reachable by writing `16`…`255` into `cp_fixed_table`, and from a malformed code-length tree that decodes a symbol in `19..=31`. | `SIGABRT` + `Assertion 'len < 16' failed.` | [x] | `aborts::abort19_code_length_ge_16` (values `16` and `255`), `oob_tables::oob04_code_length_ge_16_sweep` (15 values x 9 table positions = 135 combinations, all `SIGABRT len < 16` in both) |
| 20 | `cp_decode` (line 211) | `assert((search >> len) == (key >> len))` — the peeked bits match no code in the tree.  Always fires for an **empty** tree on the first block, because `tree[-1]` is then `0` and `len` becomes `32` (`search >> 32` is `search` on x86-64, and `search >= 0xFFFF != 0`). | `SIGABRT` + `Assertion '(search >> len) == (key >> len)' failed.` | [x] | `aborts::abort20_decode_no_match` (`decode_empty_tree`, `decode_truncated`), all fuzz sweeps, `dynamic_overshoot::ov03/ov04` |
| 21 | `cp_inflate` (line 309) | `calloc` returns `NULL` — the C does not check and dereferences `s` | `SIGSEGV` | [x] | not testable without an allocator interposer; the Rust likewise dereferences its unchecked `alloc_zeroed` result, so the observable behaviour (`SIGSEGV` on a null write) is the same. Documented only. |

## Generic FFI-boundary cases (required even though the C does not check them)

| #  | entry point | trigger | expected C result | [x] | test |
|----|-------------|---------|-------------------|-----|------|
| 22 | `convert_pix` | `bpp` outside `{1,2,3,4}`: `0, 5, 6, 7, 8, 16, 255, 256, -1, -2, -8, INT_MAX, INT_MIN` — the `switch` has no `default`, so `dst` is neither written nor advanced, while `src` still advances by `bpp` per pixel | returns normally, writes nothing | [x] | `convert_pix::err22_convert_pix_bad_bpp` |
| 23 | `convert_pix` | `w <= 0` and/or `h <= 0` (incl. `INT_MIN`); `NULL` `src`/`dst` (never dereferenced once a loop bound is non-positive) | returns normally, writes nothing | [x] | `convert_pix::err23_convert_pix_nonpositive_and_null` |
| 24 | `cp_inflate` | `out_bytes == 0`, and `out_bytes < 0` (`out_end < out`, so every literal trips row 3) | `0` + row-3 message | [x] | `errors::err24_out_bytes_zero_and_negative` |
| 25 | `cp_inflate` | `in_bytes < 0` (`-1`, `-4`, `-1000`) — `bits_left` starts negative, so row 16 fires.  Note `last_bytes = in_bytes & 3` first makes the `final_word` loop read *before* the buffer, identically in both. | `SIGABRT` (row 16) | [x] | `aborts::abort25_in_bytes_negative` |
| 26 | `cp_inflate` | `in = NULL, in_bytes = 0, out = NULL, out_bytes = 0` — `NULL` is 4-aligned, so `first_bytes == 0` and nothing is dereferenced before row 16 fires | `SIGABRT` (row 16) | [x] | `aborts::abort26_null_in` |
| 27 | `cp_inflate` | one step past every documented range.  The API has **no** C `enum` parameters, so the "out-of-range enum value across FFI" class collapses onto the two integer selectors: `BTYPE` (2 bits, `3` is the invalid one — row 6) and `bpp` (row 22).  Both are covered with values that have no valid variant. | as rows 6 / 22 | [x] | `errors::err06_btype_3_unknown_block`, `convert_pix::err22_convert_pix_bad_bpp` |
| 28 | `cp_stored` | `LEN > out_bytes` — `cp_stored` performs **no** output bound check, so `memcpy` overruns the output buffer | returns `1` and writes `LEN` bytes | [x] | `errors::err28_stored_overruns_out` (over-allocated buffer, so the overrun is compared byte-for-byte) |
| 29 | `cp_stored` | `LEN == 0` (`NLEN == 0xFFFF`) | returns `1`, writes nothing | [x] | `errors::err29_stored_zero_len`, `inflate::i28_empty_output` |

## Unchecked indexing / local-array overflow (undefined in C, reproduced anyway)

| #  | site | trigger | expected C result | [x] | test |
|----|------|---------|-------------------|-----|------|
| 30a | `cp_build` (line 137) | `counts[lens[n]]++` with `int counts[16]` and a code length `>= 16`: the C increments up to 1020 bytes past the array *before* its own `assert(len < 16)` fires in the second loop.  The Rust aborts at the counting loop instead. | `SIGABRT` + `Assertion 'len < 16' failed.` (the stack corruption is never observable, because the assert always follows) | [x] | `oob_tables::oob04_code_length_ge_16_sweep` |
| 30 | `cp_block` (line 267) | `cp_len_extra_bits[symbol]` / `cp_len_base[symbol]` with `symbol = decoded - 257 > 30`.  Requires `cp_decode` to return `>= 288`, which needs `tree[-1]`; for the literal tree that is `s->lookup[510..511]`, which `cp_build` has just zeroed whenever `nlit == 0`, so the `cp_decode` assert (row 20) fires first. | unreachable, but reproduced defensively | [x] | covered by the `cp_data_byte` layout emulation in `src/lib.rs`; reachability argued in the module comment |
| 31 | `cp_block` (line 270) | `cp_dist_extra_bits[distance_symbol]` / `cp_dist_base[distance_symbol]` with `distance_symbol > 31`.  **Reachable:** a dynamic block may declare `HDIST` distance codes and give them all code length `0`, so `cp_build` returns `0`; `cp_decode(s, s->dst, 0)` then reads `s->dst[-1]`, which is `s->lit[287]` — a well-formed entry whose symbol field is `287`.  The C then reads 255 entries past both arrays, landing on zero bytes past `.bss`, so `backwards_distance == 0` and the copy loop writes each byte onto itself. | returns `1`, output unchanged over the match | [x] | `oob_tables::oob01…oob03` — these **failed** before `src/lib.rs` gained the `.data`-image emulation (`cp_data_byte`), because Rust orders and pads its statics differently |
| 32 | `cp_dynamic` (lines 229–242) | RLE symbol 16/17/18 writing past `uint8_t lens[288+32]`, stopping inside `lenlens` (`lens[320..339]`) or `sym`/`nlen` (`lens[348..356]`) — dead locals | no observable change | [x] | `dynamic_overshoot::ov01`, `ov02` |
| 33 | `cp_dynamic` | overshoot reaching `ndst` at `lens[356..360]`, zeroing it, so `cp_build(0, s->dst, lens + nlit, 0)` yields an **empty** distance tree | literals still decode; a match then takes row 31/row 20 | [x] | `dynamic_overshoot::ov03` (both return `1` for literal-only payloads and both `SIGABRT` at row 20 for payloads with a match) |
| 34 | `cp_dynamic` | overshoot reaching `nlit` at `lens[360..364]`, zeroing it, so the literal tree is empty | `SIGABRT` at row 20 | [x] | `dynamic_overshoot::ov04` |
| 35 | `cp_dynamic` | overshoot reaching the symbol-18 run counter at `lens[364..368]`: the run zeroes its own counter, `--i` takes it negative, and `lens[376..380]` (the loop variable `n`) is reset on every pass, so `n` cycles in `257..=376` for ever | **infinite loop** | [x] | `dynamic_overshoot::ov05`, `ov06`, `ov07`, `ov08` — both implementations spin and are killed by the same `SIGALRM` budget |
| 36 | `cp_dynamic` | overshoot reaching the saved `%rbp` / return address at `lens[384..400]` | **unreachable** — row 35 fires first for every run long enough to get there (`n` never exceeds 376), so the frame pointer and return address are never touched | [x] | `dynamic_overshoot::ov08` (`k = 66…138`, i.e. every run that would reach 384, all end in row 35) |

## Randomised error-path sweeps

Besides the per-row tests, three sweeps compare *any* outcome the C produces —
return value, output bytes, `cp_error_reason`, assertion text, signal — against
the Rust:

| sweep | corpus | test |
|-------|--------|------|
| unstructured | random 1…40-byte inputs, random pointer alignment, random `out_bytes` | `aborts::fork_fuzz_unstructured` |
| mutated-valid | a well-formed stored / fixed / dynamic block, then 0…3 bit flips, byte replacements or truncations | `aborts::fork_fuzz_mutated_valid` |
| length boundaries | `in_bytes ∈ {0,1,2,3,len-1,len,len+1,-1}` × `out_bytes ∈ {-1,0,1,n-1,n}` × 4 alignments | `aborts::fork_fuzz_length_boundaries` |
| exec-based (compares the *whole* stderr text, not just the assertion line) | random small inputs | `aborts::abort_fuzz_random_inputs` |
