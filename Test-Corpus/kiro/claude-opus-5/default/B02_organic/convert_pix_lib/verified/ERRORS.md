# ERRORS.md — Phase A error-surface table

Derived mechanically from `c_src/src/lib.c` with:

```
grep -n "return 0;\|return NULL\|return -1\|goto cp_err\|cp_error_reason =" lib.c
grep -n "assert(" lib.c
```

`R` = reachable from the public ABI (`cp_inflate` / `convert_pix` / the exported
globals). `assert()` is **live** in the C build (see `SYMBOLS.md`), so an
assert row means the C library `abort()`s (SIGABRT / exit 134).

## Explicit error returns

| # | function | trigger (exact invalid input/condition) | expected C result | test | verified |
|---|----------|------------------------------------------|-------------------|------|----------|
| 1 | `cp_stored` (lib.c:169-176) | btype==0 stored block where `LEN != (uint16_t)~NLEN` | `cp_inflate` → `0`; `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `err01_stored_len_nlen_mismatch` (300 randomized + off-by-one boundaries) | ✅ |
| 2 | `cp_stored` (lib.c:178-184) | btype==0 stored block where `!(s->bits_left/8 <= (int)LEN)`, i.e. more whole bytes remain in the input than `LEN` | `cp_inflate` → `0`; reason = `"Stored block extends beyond end of input stream."` | `err02_stored_extends_beyond_input` (300 randomized + exact/`+1` boundary) | ✅ |
| 3 | `cp_block` (lib.c:252-260) | literal symbol (<256) decoded when `!(s->out + 1 <= s->out_end)` — output buffer already full | `cp_inflate` → `0`; reason = `"Attempted to overwrite out buffer while outputting a symbol."` | `err03_literal_overruns_out`, `g02_out_bytes_zero_with_literal`, `g05_negative_out_bytes` | ✅ |
| 4 | `cp_block` (lib.c:272-279) | length/distance pair whose `backwards_distance` makes `s->out - backwards_distance < s->begin` | `cp_inflate` → `0`; reason = `"Attempted to write before out buffer (invalid backwards distance)."` | `err04_invalid_backwards_distance` (300 randomized + `dist == produced` / `produced+1` boundary) | ✅ |
| 5 | `cp_block` (lib.c:281-288) | length/distance pair where `!(s->out + length <= s->out_end)` — match runs past the output buffer | `cp_inflate` → `0`; reason = `"Attempted to overwrite out buffer while outputting a string."` | `err05_string_overruns_out` (300 randomized + exact-fit boundary + `dist==1` memset path) | ✅ |
| 6 | `cp_inflate` (lib.c:355-360) | block header with `btype == 3` | `cp_inflate` → `0`; reason = `"Detected unknown block type within input stream."` | `err06_unknown_block_type` (bfinal 0/1, and as a second block) | ✅ |
| 7 | `cp_unfilter` (lib.c:434) | row 0 filter byte `> 4` | `return 0` | `err07_to_err10_unreachable_static_rejections` | ✅ unreachable, verified structurally |
| 8 | `cp_unfilter` (lib.c:468) | row `y>=1` filter byte `> 4` | `return 0` | same | ✅ unreachable, verified structurally |
| 9 | `cp_chunk` (lib.c:397) | `memcmp(start+4, chunk, 4) != 0`, **or** `len < minlen`, **or** `png->p + len + 12 > png->end` | `return NULL` | same | ✅ unreachable, verified structurally |
| 10 | `cp_find` (lib.c:409) | scan reaches `png->p >= png->end` without a matching chunk | `return NULL` | same | ✅ unreachable, verified structurally |

Rows 7-10 live in `static` functions with **no caller anywhere in the
translation unit** (confirmed: `grep -n cp_unfilter c_src/src/lib.c` and
`grep -n cp_chunk|cp_find` show only their definitions). They are unreachable
across the FFI boundary in both libraries. The test asserts that *neither* `.so`
exports them, so no differential call is possible — and that the Rust does not
export them either, which would itself break symbol parity.

## Live `assert()` rejections (C aborts; the Rust transcribes each one)

The Rust translation transcribes every `assert()` as `cp_assert!`, which writes a
diagnostic and calls `process::abort()`. Verification compares **wait status AND
the assertion expression that fired**, so "both crashed" is not accepted — they
must crash on the same assert. Tested by `tests/phase_c_aborts.rs`.

| # | function | trigger (exact invalid input/condition) | expected C result | test | verified |
|---|----------|------------------------------------------|-------------------|------|----------|
| 11 | `cp_ptr` (lib.c:89) | `assert(!(s->bits_left & 7))`. Needs three things at once: a stored block that is not the first block; `cp_peak_bits` having taken its `final_word` branch while `count % 8 != 0` (that branch does `count += bits_left`, double counting the buffered bits, which lets later reads drive `bits_left` **negative** — and `-13 & 7 == 3`); and `LEN == (uint16_t)~NLEN` still holding on bits already past the end of input (achieved by truncating the stream just past the LEN field so NLEN reads back as 0). | SIGABRT | `err11_cp_ptr_alignment_assert` (targeted 320-candidate sweep) | ✅ reached 5x, identical on both |
| 12 | `cp_peak_bits` (lib.c:98) | `assert(s->word_index <= s->word_count)` — **cannot fire**: guarded by the enclosing `if (s->word_index < s->word_count)`, so after `++` it is at most equal. | never fires | n/a (transcribed anyway) | ✅ by construction |
| 13 | `cp_consume_bits` (lib.c:109) | `assert(s->count >= num_bits_to_read)` — consume more bits than are buffered (stream truncated mid-symbol) | SIGABRT | `r13_truncated_mid_symbol` + swept 28x/8x | ✅ |
| 14 | `cp_read_bits` (lib.c:117) | `assert(num_bits_to_read <= 32)` — **cannot fire**: every call site passes a literal ≤ 16 or `count & 7` (≤ 7) | never fires | n/a (transcribed anyway) | ✅ by construction |
| 15 | `cp_read_bits` (lib.c:118) | `assert(num_bits_to_read >= 0)` — **cannot fire**: every call site passes a non-negative literal or `count & 7` | never fires | n/a (transcribed anyway) | ✅ by construction |
| 16 | `cp_read_bits` (lib.c:119) | `assert(s->bits_left > 0)` — read a bit after the input is fully consumed: `in_bytes == 0`, negative `in_bytes`, or a stream in which no block sets BFINAL | SIGABRT | `g1_in_bytes_zero`, `g4_in_bytes_negative`, `r16_no_final_block` + swept 72x/9x | ✅ |
| 17 | `cp_read_bits` (lib.c:120) | `assert(s->count <= 64)` — **cannot fire**: `cp_peak_bits` refills at most once per call and only while `count < num_bits ≤ 16`, so `count ≤ 16 + 32 = 48` | never fires | n/a (transcribed anyway) | ✅ by construction |
| 18 | `cp_read_bits` (lib.c:121) | `assert(!cp_would_overflow(s, n))`, i.e. `(bits_left + count) - n < 0` — ask for more bits than the stream can still supply | SIGABRT | swept 6x/1x | ✅ |
| 19 | `cp_build` (lib.c:148) | `assert(len < 16)` — a code length ≥ 16 in `lens[]`. Not reachable through the stream (lengths come from 3-bit reads or the 0..15 symbol alphabet) but **is** reachable by a caller writing ≥ 16 into the exported `cp_fixed_table`. Note the C's `counts[lens[n]]++` in the *first* loop is already an out-of-bounds write for `len > 15`; the abort happens later, at this assert. The Rust widens `counts` to 256 entries so it aborts at the same site rather than on a bounds check. | SIGABRT at `len < 16` | `r19_fixed_table_len_ge_16` | ✅ |
| 20 | `cp_decode` (lib.c:211) | `assert((search >> len) == (key >> len))` — the peeked bits match no code in the tree (corrupt Huffman stream). Note `len = 32 - (key & 0xF)` is 32 when `key & 0xF == 0`, and `uint32_t >> 32` on x86-64 masks the count to 5 bits; the Rust uses `wrapping_shr` to match. | SIGABRT | `r20_corrupt_huffman` + swept 81x | ✅ |

## Generic FFI boundary cases (no explicit check in C — both must agree)

| # | entry point | trigger | expected C result | test | verified |
|---|-------------|---------|-------------------|------|----------|
| G1 | `cp_inflate` | `in_bytes == 0` (empty input) | trips assert #16 → SIGABRT | `g1_in_bytes_zero` | ✅ |
| G2 | `cp_inflate` | `out_bytes == 0` with a literal to emit | error row #3 → `0` | `g02_out_bytes_zero_with_literal` | ✅ |
| G3 | `cp_inflate` | `out_bytes == 0`, stored block with `LEN == 0` | `1` (memcpy of 0 bytes) | `g03_out_bytes_zero_stored_len_zero` | ✅ |
| G4 | `cp_inflate` | negative `in_bytes` | `bits_left < 0` → assert #16 → SIGABRT | `g4_in_bytes_negative` | ✅ |
| G5 | `cp_inflate` | negative `out_bytes` (`out_end < out`) | error row #3 or #5 → `0` | `g05_negative_out_bytes` (-1, -4, -1000, and a match) | ✅ |
| G6 | `convert_pix` | `bpp` not in `{1,2,3,4}` — the `switch` has **no `default`**, so no pixel is written and `dst` does not advance, while `src` still advances by `bpp` per column | returns `void`, `dst` untouched | `g06_convert_pix_bpp_out_of_range` (`INT_MIN`, -1000, -4, -1, 0, 5, 6, 7, 8, 255, 256, 65536, `INT_MAX` × 5 shapes) | ✅ |
| G7 | `convert_pix` | `w == 0` | `dst` untouched; `src` advanced by `h` | `row42_convert_pix_w_zero` | ✅ |
| G8 | `convert_pix` | `h == 0` | `dst` and `src` untouched (loop body never runs) | `row43_convert_pix_h_zero` | ✅ |
| G9 | `convert_pix` | `w < 0` or `h < 0` (incl. `INT_MIN`) | loops never execute; `dst` untouched | `g09_convert_pix_negative_w_h` | ✅ |
| G10 | exported globals | caller mutates `cp_len_base` / `cp_dist_base` / `cp_len_extra_bits` / `cp_dist_extra_bits` / `cp_permutation_order` / `cp_fixed_table` before `cp_inflate` | both libraries must read the *mutated* global and produce the same output | `CONFIGS.md` rows 31-35 | ✅ |
| G11 | `cp_error_reason` | read after a successful `cp_inflate` | not written on success — retains its previous value | `row36_error_reason_global_roundtrip` | ✅ |
| G12 | `cp_inflate` | `in` pointer at each alignment mod 4 (`first_bytes` ∈ {0,1,2,3}) × each `last_bytes` | derived from the address; both libraries must agree for the same pointer | `CONFIGS.md` rows 5, 15, 24 (all 16 shapes) | ✅ |
| G13 | `cp_inflate` | `out == NULL` with `out_bytes == 0` (no store performed) | error row #3 → `0` | `g_null_out_zero_len` | ✅ |
| G14 | `convert_pix` | `src == NULL` / `dst == NULL` where the C performs no access (`h == 0`, or `w == 0` for `dst`) | returns without touching memory | `g_convert_pix_null_pointers_no_deref` | ✅ |
| G15 | `cp_inflate` | literal symbols 286/287 and distance symbols 30/31 — the `+2` padding slots, whose `cp_len_base`/`cp_dist_base` entries are `0`, giving a zero-length match / zero distance | reachable and well defined: `while (length--)` with `length == 0` copies nothing | `g_padding_symbols_286_287_and_dist_30_31` | ✅ |
| G16 | `cp_inflate` | random garbage interpreted as a deflate stream | must accept/reject identically | `g_random_garbage_streams` (4000 in-process), `fuzz_full_input_space_matches` (300 incl. all btypes and the abort half) | ✅ |

Note on out-of-range enum values: this ABI declares no `enum` parameters. The
equivalent class of input is the un-`default`-ed `switch` on `bpp` in
`convert_pix` (row G6, 13 out-of-range values tested) and the `btype` `switch` in
`cp_inflate` (row #6 — `btype == 3` is the only out-of-range value a 2-bit read
can produce, and it is tested in three positions).

## Verification status

All 20 numbered rows and all 16 generic rows above are covered by a passing
differential test. Run them with:

```
cd translation
cargo test --release --test phase_c_errors -- --test-threads=1   # returning rows
cargo test --release --test phase_c_aborts -- --test-threads=1   # abort rows (~4 min)
```
