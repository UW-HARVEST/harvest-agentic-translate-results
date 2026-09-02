# ERRORS.md — Error-surface table (Phase A / gate for Phase C)

Derived mechanically from `c_src/src/lib.c` by grepping every
`cp_error_reason = …` / `goto cp_err` / `return 0` / `return NULL` /
`assert(...)` / explicit range or bounds comparison. Line numbers refer to
`c_src/src/lib.c`.

## Note on `assert()` — build configuration matters

`c_src/CMakeLists.txt` sets **no** `CMAKE_BUILD_TYPE` and no `-DNDEBUG`, so the
`.so` produced by the documented build command has **live** `assert()`s (it
imports `__assert_fail`). The Rust translation elides them (`NDEBUG` semantics),
which is what a `cdylib` with `panic = "abort"` should do — an aborting library
has no return value to compare.

Consequences for testing, and how this file handles it:

* Rows **E1–E8** are *real, returned* rejections (`cp_inflate` → `0`,
  `unfilter` → `0`). They are asserted for equality of return value **and** of
  the `cp_error_reason` string. These are the rows that matter.
* Rows **A1–A10** are `assert()` sites. For each, the table records the C
  behaviour under **both** builds. Every test that can reach an `assert` runs the
  C side inside a `fork()`ed child (`tests/common/mod.rs::run_in_child`) so a
  `SIGABRT` cannot take the harness down, and the differential comparison is made
  against a second C build (`c_src/build_ndebug/`, configured with
  `-DCMAKE_C_FLAGS=-DNDEBUG`). Because no `assert` expression in this file has
  side effects (`cp_would_overflow` is pure), the `NDEBUG` build is
  bit-identical to the default build on every input where the asserts hold —
  so testing against `build_ndebug` is a strict *superset* of testing against
  `build`, and row A-tests verify the Rust matches the `NDEBUG` C exactly.

## E — Rejections the C actually returns

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `cp_stored` (L176-182) → `cp_inflate` | stored block (`btype==0`) whose 16-bit `LEN` is not the one's complement of the 16-bit `NLEN` | `cp_inflate` returns `0`; `cp_error_reason == "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."`; nothing copied to `out` |
| E2 | `cp_stored` (L184-190) → `cp_inflate` | stored block where `s->bits_left / 8 > (int)LEN`, i.e. more input bytes remain after the `LEN`/`NLEN` header than `LEN` claims (e.g. a stored block that is *not* the last thing in the stream, or `LEN` smaller than the trailing input) | `cp_inflate` returns `0`; `cp_error_reason == "Stored block extends beyond end of input stream."` |
| E3 | `cp_block` (L259-267) → `cp_inflate` | literal symbol (`symbol < 256`) decoded when `s->out + 1 > s->out_end`, i.e. `out` is already full (includes `out_bytes == 0` and `out_bytes < 0`) | `cp_inflate` returns `0`; `cp_error_reason == "Attempted to overwrite out buffer while outputting a symbol."` |
| E4 | `cp_block` (L278-286) → `cp_inflate` | length/distance pair whose `backwards_distance` reaches before the start of `out` (`s->out - backwards_distance < s->begin`) — e.g. a match at output offset 0, or distance > bytes emitted so far | `cp_inflate` returns `0`; `cp_error_reason == "Attempted to write before out buffer (invalid backwards distance)."` |
| E5 | `cp_block` (L287-295) → `cp_inflate` | length/distance pair where the copy would run past the end of `out` (`s->out + length > s->out_end`); checked *after* E4 so a too-short `out` with a valid distance hits this one | `cp_inflate` returns `0`; `cp_error_reason == "Attempted to overwrite out buffer while outputting a string."` |
| E6 | `cp_inflate` (L361-367) | `btype == 3` in a block header (the reserved DEFLATE block type) | `cp_inflate` returns `0`; `cp_error_reason == "Detected unknown block type within input stream."` |
| E7 | `unfilter` (L438-441) | `h > 0` and the row-0 filter byte `raw[0]` is `>= 5` (any of 5…255) | `unfilter` returns `0`; `raw` unmodified |
| E8 | `unfilter` (L472-475) | some row `y` in `1 .. h` has filter byte `>= 5` (any of 5…255) | `unfilter` returns `0`; rows `0 .. y-1` already unfiltered in place, row `y` onwards untouched — the partial mutation must match byte-for-byte |

### E — dead rejection sites (no caller in the C TU; kept for completeness)

| # | function | trigger | expected C result |
|---|----------|---------|-------------------|
| E9  | `cp_chunk` (L403) | chunk tag mismatch, `len < minlen`, or `png->p + len + 12 > png->end` | returns `NULL` |
| E10 | `cp_find` (L415)  | no chunk with matching tag and `len >= minlen` fits before `png->end` | returns `NULL` |

`cp_chunk` / `cp_find` are `static` and never called, so they are unreachable
through the `.so` boundary. Not testable differentially; noted only so the grep
is exhaustive.

## A — `assert()` sites

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| A1  | `cp_ptr` (L95) | `cp_stored` reaches the `memcpy` while `s->bits_left & 7 != 0` (bit position not byte-aligned) | default build: `SIGABRT`. `NDEBUG`: computes `p` anyway and copies |
| A2  | `cp_peak_bits` (L104) | `s->word_index > s->word_count` after the increment — unreachable, the branch is guarded by `word_index < word_count` | never fires; both builds identical |
| A3  | `cp_consume_bits` (L115) | `s->count < num_bits_to_read` — reachable when the input is exhausted (`peak` had nothing to add) | default: `SIGABRT`. `NDEBUG`: masks/shifts the short bit buffer, `count`/`bits_left` go negative |
| A4  | `cp_read_bits` (L123) | `num_bits_to_read > 32` — unreachable: all call sites pass a literal `1,2,3,4,5,7,16`, `s->count & 7` (≤7), or a table entry ≤ 13 | never fires |
| A5  | `cp_read_bits` (L124) | `num_bits_to_read < 0` — unreachable for the same reason | never fires |
| A6  | `cp_read_bits` (L125) | `s->bits_left <= 0`, i.e. every bit of the input has already been consumed and the decoder wants more (truncated stream, `in_bytes == 0`, `in_bytes < 0`) | default: `SIGABRT`. `NDEBUG`: keeps decoding from an empty/stale bit buffer, `bits_left` goes further negative |
| A7  | `cp_read_bits` (L126) | `s->count > 64` — unreachable: `peak` only tops up when `count < num_bits_to_read ≤ 32`, so `count ≤ 63` before the `+32` | never fires |
| A8  | `cp_read_bits` (L127) | `cp_would_overflow(s, n)`, i.e. `(bits_left + count) - n < 0` — reachable on a truncated stream | default: `SIGABRT`. `NDEBUG`: proceeds |
| A9  | `cp_build` (L154) | a code length `>= 16` in `lens[]`. Unreachable from `cp_dynamic` (3-bit `lenlens` ≤ 7; `lens[]` entries are `sym ≤ 15` or `0`) and from `cp_fixed` unless a caller mutates the exported `cp_fixed_table`, which is **not** `static` | fires only if `cp_fixed_table` is corrupted by the caller; also note `counts[lens[n]]++` at L142 indexes a 16-entry stack array, so `lens[n] >= 16` is an out-of-bounds stack write in both languages |
| A10 | `cp_decode` (L217) | `(search >> len) != (key >> len)` — the bit pattern matched no code in `tree`. Reachable whenever `hi == 0` (empty tree ⇒ `tree[-1]` is read) or the peeked bits are not a prefix of any code | default: `SIGABRT`. `NDEBUG`: returns `(key >> 4) & 0xFFF` from whatever `tree[lo-1]` held — for `lo == 0` that is the struct field preceding `tree`, which is why `cp_state_t` must keep C layout |

## Generic FFI boundary cases (not in the table above, still tested in Phase C)

The C code contains **no** null checks and **no** enum parameters, so there is no
"invalid enum value" row in the strict sense. The boundary cases that do exist:

| case | C behaviour | tested as |
|---|---|---|
| `cp_inflate(NULL, 0, …)` / `unfilter(…, NULL)` | dereferenced unconditionally ⇒ `SIGSEGV`; no check exists | both sides asserted to fault identically, in a forked child |
| `cp_inflate(in, 0, …)` | `bits_left = 0` ⇒ A6 | fork + `NDEBUG` build |
| `cp_inflate(in, negative, …)` | `bits_left < 0`, `word_count < 0`, `last_bytes = (neg & 3)` ⇒ A6 | fork + `NDEBUG` build |
| `out_bytes == 0` / `out_bytes < 0` | `out_end <= begin` ⇒ E3 or E5 on the first output | direct |
| `in` pointer alignment mod 4 | changes `first_bytes`, `words`, `word_count`, `last_bytes` — a *behavioural* input, see `CONFIGS.md` rows C1-C4 | direct, all 4 alignments |
| `unfilter` filter byte `5 … 255` | E7 / E8; every one of the 251 invalid values checked | direct |
| `unfilter` with `h == 0` / `h < 0` | row-0 block skipped, `raw` never read, returns `1` | direct |
| `unfilter` with `w == 0` or `bpp == 0` (`len == 0`) | all inner loops empty; still consumes one filter byte per row | direct |
| `unfilter` with `bpp > len` | row-0 loops don't run; rows ≥ 1 run only the `x < bpp` prologue, which reads `prev[x]` past the row | direct, sizes padded |
| `unfilter` `w * bpp` overflowing `int` | signed overflow in C (UB); Rust uses `wrapping_mul` | not tested (nondeterministic in C) |
| symbol values out of table range in `cp_block` | `symbol - 257 > 30` indexes `cp_len_extra_bits` / `cp_len_base` out of bounds, reading whatever global follows — layout differs between the two `.so`s | not testable; requires an empty `lit` tree (A10) first |
| `lens[]` overrun in `cp_dynamic` | `sym == 18` near `n == nlit+ndst-1` writes up to 137 bytes past `uint8_t lens[320]`, corrupting `cp_dynamic`'s own stack frame | the one genuinely non-comparable class — see "The `lens[]` overrun" below. Detected exactly, per input, by an instrumented oracle |


## Verification outcome

All eight `E` rows and all reachable `A` rows have passing differential tests
(`tests/phase_c_errors.rs`), and a randomized fuzz differential
(`tests/fuzz_differential.rs`) has run ~50 000 malformed `cp_inflate` inputs and
~15 000 `unfilter` inputs across five seeds. Every divergence found reduces to a
single class, below; there were **zero** divergences on inputs where the C stays
within defined behaviour.

### The `lens[]` overrun — the only non-comparable class

`cp_dynamic` declares `uint8_t lens[288 + 32]` (320 bytes) and fills it with

```c
for (int n = 0; n < nlit + ndst;) {
  int sym = cp_decode(s, s->len, s->nlen);
  switch (sym) {
  ...
  case 18: for (int i = 11 + cp_read_bits(s, 7); i; --i, ++n) lens[n] = 0; break;
  ...
  }
}
```

A code-length symbol 18 decoded at `n == nlit + ndst - 1` writes 138 more
entries, so the highest index reached is `nlit + ndst + 136`. With `nlit <= 288`
and `ndst <= 32` that is 456 — up to **137 bytes past the array**. Once
`nlit + ndst >= 184` the writes land in `cp_dynamic`'s own stack frame and
clobber `nlit`, `ndst`, and the loop counters `n` and `i`.

The effect is decided entirely by the stack frame layout. An instrumented copy of
`c_src/src/lib.c` — identical except for added `fprintf`s, which move the locals —
*terminates* on exactly the input that makes the real build spin forever. There is
no defined behaviour here for the Rust to reproduce.

How the test suite handles it: `tools/build_lens_probe.sh` builds a copy of
`c_src/src/lib.c` whose only changes are a padded `lens[]` and a new exported
`int cp_lens_overrun` flag set whenever the fill loop writes at or past index 320.
Because the decode path up to that point is the same code, the flag is an *exact*
per-input predicate for "the real C entered undefined behaviour". `compare_or_ub`
tolerates a divergence only when the oracle reports `true`, and **fails the test**
on any divergence where it reports `false`. `fuzz_overshoot_free_streams_match_exactly`
additionally asserts the tolerated count stays at zero for streams that never
reach `cp_dynamic`.

### Rust changes made for this row class

Two changes were needed so the Rust stays *total* on the inputs where the C reads
or writes out of bounds. Neither alters behaviour for any in-range input:

1. `cp_build`'s `codes` / `first` / `counts` were 16-element Rust arrays, so
   `counts[lens[n]]++` with a code length `>= 16` (row A9, reachable from a
   malformed stream via `s->nlen == 0`) hit a bounds-check panic. Under
   `panic = "abort"` that becomes `SIGABRT` — a much larger divergence than the
   C's silent out-of-bounds stack write, and wrong under `NDEBUG` where the C
   does not abort at all. They are now 256 entries wide, which covers every value
   a `uint8_t` code length can take.
2. `cp_build` indexed `tree` with `slot as i32 as isize`. In C, `uint32_t slot`
   is *zero*-extended for pointer arithmetic, so a slot above `2^31` became a
   large negative offset in Rust. Now `tree.add(slot as usize)`.
3. `cp_dynamic`'s `lenlens[19]` is indexed by `cp_permutation_order[i]`, and that
   table is exported and mutable — a caller can drive the index past 18. The
   backing store is now padded so the write is defined instead of panicking.

### `assert()` and build configuration, restated

The `assert()`-guarded rows are compared against the `-DNDEBUG` C build. That is
sound rather than a weakening: no `assert` expression in this file has side
effects (`cp_would_overflow` is pure), so the `NDEBUG` build is bit-identical to
the documented build on every input where the asserts hold — testing against it is
a strict *superset*. For each `A` row the test additionally asserts that the
documented build really does `SIGABRT`, confirming the row's trigger is genuine
and not merely hypothetical.

## Row → test coverage map (Phase C gate)

Every row above maps to a named test in `tests/phase_c_errors.rs`. All are
passing; `tools/verify.sh` re-runs them.

| row | test | status |
|---|---|---|
| E1  | `e1_stored_len_nlen_mismatch` — 240 randomized bad `NLEN`s across all 4 input alignments, plus the ±1 boundary | [x] |
| E2  | `e2_stored_extends_beyond_input` — stored-block-followed-by-more-data, `LEN` under-declared, and the `declared == real - 1` boundary | [x] |
| E3  | `e3_literal_overflows_out` — `out_bytes` 0, `out_bytes == k` with `k+1` literals (asserts the partial output), negative `out_bytes`, and `out == NULL` with `out_bytes == 0` | [x] |
| E4  | `e4_backwards_distance_before_begin` — 150 randomized over-long distances plus an exhaustive `dist == produced` / `produced + 1` boundary sweep | [x] |
| E5  | `e5_string_overflows_out` — 150 randomized short buffers, the exact-fit boundary, and the zero-length (`lc` 29/30) case that must land on E4 instead | [x] |
| E6  | `e6_reserved_block_type` — `btype == 3` at both `bfinal` values, all 4 alignments, and as a *second* block (first block's output verified intact) | [x] |
| E7  | `e7_unfilter_bad_row0_filter` — all 251 invalid filter values × 3 `bpp`, plus `h ∈ {1,2,5}` and the accepted boundary value 4 | [x] |
| E8  | `e8_unfilter_bad_row_filter` — all 251 invalid values, every row index, asserting the partial in-place mutation matches and the tail is untouched | [x] |
| E9  | `cp_chunk` — `static` with no caller; unreachable through the `.so`. Documented, not testable | [n/a] |
| E10 | `cp_find` — same | [n/a] |
| A1  | `a1_a3_stored_unaligned_and_short_buffer` | [x] |
| A2  | `a2_a4_a5_a7_unreachable_asserts` — verified statically against `c_src/src/lib.c` (the guard `if (s->word_index < s->word_count)` makes it unreachable) | [x] |
| A3  | `a1_a3_stored_unaligned_and_short_buffer` | [x] |
| A4  | `a2_a4_a5_a7_unreachable_asserts` — scans every `cp_read_bits` call site and both extra-bit tables (max 5 and 13) | [x] |
| A5  | `a2_a4_a5_a7_unreachable_asserts` — same scan | [x] |
| A6  | `a6_a8_truncated_stream_bits_left_exhausted`, `a6_in_bytes_zero_and_negative` (`in_bytes` 0, −1, −3, −4, −17, −4096) | [x] |
| A7  | `a2_a4_a5_a7_unreachable_asserts` | [x] |
| A8  | `a6_a8_truncated_stream_bits_left_exhausted` | [x] |
| A9  | `a9_corrupted_fixed_table_code_length` — pokes lengths 16/17/31/255 into the exported `cp_fixed_table` in both `.so`s, asserts the documented C aborts, asserts the Rust stays defined, and restores the tables | [x] |
| A10 | `a10_empty_and_mismatched_huffman_tree` — empty literal tree via `HCLEN = 4`, an incomplete literal tree, and 250 random garbage streams | [x] |

Generic boundary cases: `boundary_null_pointers`,
`boundary_oversized_and_negative_lengths`,
`boundary_unfilter_out_of_range_filter_tag_and_extreme_dims` (all 256 filter-tag
values in row 0 and row 1; `w`/`h`/`bpp` at `i32::MIN`, `i32::MAX`, `±1`, `0`).
