# ERRORS.md — Phase A: the error-surface table

Mechanically derived from `c_src/src/lib.c` by grepping **every** rejection
site:

```
grep -n "cp_error_reason =" c_src/src/lib.c   ->  6 hits (6 distinct strings)
grep -n "assert("          c_src/src/lib.c   -> 10 hits
grep -n "return 0;|return NULL|goto cp_err|cp_err:" c_src/src/lib.c
                                              -> 3 `cp_err:` labels + 4 bare `return 0;`
                                                 + 3 `return 0;` used as `return NULL`
```

`cp_error_reason` is a *sticky* global: nothing in the library ever clears it,
so every test sets it to `NULL` in **both** libraries before the call and
compares the pointed-to C string afterwards.

Legend for "expected C result": `ret` is the value returned by the exported
entry point, `err` is the `cp_error_reason` string after the call.

## A. Explicit error returns (`cp_error_reason` + `ret == 0`)

| # | function (entry point) | trigger (the exact invalid input/condition) | expected C result | test |
|---|------------------------|---------------------------------------------|-------------------|------|
| 1 | `cp_stored` ← `cp_inflate`, btype 0 | `LEN != (uint16_t)~NLEN` (the two 16-bit fields of a stored block are not complements) | `ret = 0`, `err = "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `err01_stored_len_nlen_mismatch` |
| 2 | `cp_stored` ← `cp_inflate`, btype 0 | `!(s->bits_left / 8 <= (int)LEN)`, i.e. **more** input bytes remain than `LEN` announces (note the inverted sense — a stored block must be the *last* thing in the stream) | `ret = 0`, `err = "Stored block extends beyond end of input stream."` | `err02_stored_extends_beyond` |
| 3 | `cp_block` ← `cp_inflate`, btype 1/2 | literal symbol (`sym < 256`) decoded while `!(s->out + 1 <= s->out_end)` (output buffer full / `out_bytes` too small, incl. `out_bytes == 0` and negative) | `ret = 0`, `err = "Attempted to overwrite out buffer while outputting a symbol."` | `err03_out_symbol_overflow` |
| 4 | `cp_block` ← `cp_inflate`, btype 1/2 | match symbol whose `backwards_distance` reaches before the start of the output buffer: `!(s->out - backwards_distance >= s->begin)` | `ret = 0`, `err = "Attempted to write before out buffer (invalid backwards distance)."` | `err04_backwards_distance` |
| 5 | `cp_block` ← `cp_inflate`, btype 1/2 | match symbol whose copy would run past the end: `!(s->out + length <= s->out_end)` (checked *after* row 4) | `ret = 0`, `err = "Attempted to overwrite out buffer while outputting a string."` | `err05_out_string_overflow` |
| 6 | `cp_inflate` | `btype == 3` (the reserved 2-bit block type) | `ret = 0`, `err = "Detected unknown block type within input stream."` | `err06_btype3` |

### Ordering sub-rows (which check fires when several are violated)

| # | entry point | trigger | expected C result | test |
|---|-------------|---------|-------------------|------|
| 7 | `cp_stored` | LEN/NLEN mismatch **and** stored-block-too-long | row 1's message wins (LEN/NLEN is checked first) | `err07_stored_check_order` |
| 8 | `cp_block` | match with both an out-of-range backwards distance **and** a length past `out_end` | row 4's message wins (distance is checked first) | `err08_block_check_order` |

## B. `unfilter` rejections (`return 0`, `cp_error_reason` left untouched)

`unfilter` has **no** error strings and **no** other validation: it does not
check `raw` for `NULL`, does not check `w`/`h`/`bpp` for sign or range, and
does not bound the writes. The filter byte is a de-facto enum (`0..=4`) with a
`default:` arm, and it is read from *caller data*, so out-of-range "enum"
values are a first-class input.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| 9  | `unfilter` | `h > 0` and `raw[0] ∉ {0,1,2,3,4}` (row-0 filter byte out of range: 5, 6, 127, 128, 254, 255 — all 251 invalid values are covered exhaustively) | `ret = 0`; bytes already written are left as-is (none, the check happens before any write); `cp_error_reason` unchanged | `err09_row0_bad_filter` |
| 10 | `unfilter` | some row `y ∈ [1, h)` has a filter byte `∉ {0,1,2,3,4}` | `ret = 0`; rows `0..y` have already been unfiltered in place (partial mutation is observable) | `err10_rowy_bad_filter` |
| 11 | `unfilter` | `h <= 0` (`h == 0`, `h == -1`, `h == INT_MIN`) — not an error, but the *rejection-like* early-out path: **no byte of `raw` is read or written**, not even the filter byte | `ret = 1`, `raw` untouched | `err11_h_nonpositive_no_access` |
| 12 | `unfilter` | `raw == NULL` **and** `h <= 0` | `ret = 1`, no dereference, no crash | `err12_null_raw_h_nonpositive` |
| 13 | `unfilter` | `raw == NULL` **and** `h > 0` | reads `*NULL` → `SIGSEGV` (identical in both libraries) | `err13_null_raw_h_positive` |

## C. `cp_inflate` boundary inputs that are *not* explicitly checked

The C code performs **no** validation of its four parameters; these rows pin
down what it actually does so the Rust translation cannot "helpfully" reject
them.

| # | entry point | trigger | expected C result | test |
|---|-------------|---------|-------------------|------|
| 14 | `cp_inflate` | `in_bytes == 0` with a 4-byte-aligned `in` | `bits_left = 0`, `word_count = 0`, `final_word_available = 0`; the first `cp_read_bits` returns 0 bits ⇒ `bfinal = 0`, `btype = 0` ⇒ `cp_stored` sees `LEN = NLEN = 0` ⇒ row 1 fires: `ret = 0`, `err = "Failed to find LEN and NLEN as complements…"` | `err14_in_bytes_zero` |
| 15 | `cp_inflate` | `in == NULL`, `in_bytes == 0` | same as row 14, **no crash** (`NULL` is 4-aligned so `first_bytes == 0` and nothing is dereferenced) | `err15_null_in_zero_len` |
| 16 | `cp_inflate` | `in_bytes < 0` (`-1`, `-2`, `-3`, `-4`, `-5`, `-1000`, `INT_MIN`) | `bits_left = in_bytes * 8 <= 0`, `word_count` negative ⇒ no word is loaded, and `final_word` is assembled from `in[in_bytes - last_bytes + i]`, a read *before* the buffer. The first `cp_read_bits` then trips row 22 | `SIGABRT` / decodes zeros | `err16_negative_in_bytes` |
| 16b | `cp_inflate` | `in_bytes == INT_MIN + 1` | `last_bytes == 1`, so the `final_word` loop reads `in[INT_MIN]` — a wild read 2 GiB below the buffer | `SIGSEGV` in both libraries | `err16_negative_in_bytes` |
| 17 | `cp_inflate` | `out_bytes == 0` with a stream that emits at least one literal | row 3 fires | `err03_out_symbol_overflow` |
| 18 | `cp_inflate` | `out_bytes < 0` (`out_end < out`) | row 3 fires on the first literal (`out + 1 <= out_end` is false) | `err18_negative_out_bytes` |
| 19 | `cp_inflate` | `out == NULL`, `out_bytes == 0`, stream emits a literal | row 3 fires before any dereference ⇒ `ret = 0`, no crash. With `out_bytes < 0` instead, `out_end = NULL + negative` *wraps* to a huge address, the check passes and `*s->out = symbol` faults ⇒ `SIGSEGV` in both | `err19_null_out` |
| 20 | `cp_inflate` | truncated *stored* block: `LEN` bytes announced, fewer available. The `LEN >= remaining` check (row 2) *passes*, and `memcpy(s->out, cp_ptr(s), LEN)` reads past the end of `in` and writes past `out_end` **without any bounds check** | copies `LEN` bytes regardless — both libraries must read/write the same offsets | `err20_stored_overreads` |
| 21 | `cp_inflate` | btype 0 stored block that is *not* the last block (`bfinal == 0`) followed by more data | row 2 fires (there are still input bytes left) | `err02_stored_extends_beyond` |

## D. `assert()` sites

The reference `.so` built by the command in the task description has **no
`CMAKE_BUILD_TYPE`**, therefore **no `-DNDEBUG`**, therefore these asserts are
*live* and abort the process (`__assert_fail` appears in `nm -D -u`).

The translation therefore **reproduces them**, behind the `c-asserts` cargo
feature, which is *on by default*: a failing translated assert writes a
glibc-shaped diagnostic to stderr and calls `abort()`, so it dies with
`SIGABRT` exactly where the C library does. `--no-default-features` drops them
and reproduces a `-DNDEBUG` build of the same C source instead. The harness
picks the matching C library automatically (`common::c_ref()`).

The harness also *compares the assert diagnostics*: it captures the child's
stderr through a pipe and normalises it to
``lib.c:{line}: {func}: Assertion `{expr}' failed.`` (dropping the program name
and the source directory, which necessarily differ). That normalised string is
part of the compared `Outcome`, so "the **same** assert fired" is a checked
property, not an assumption.

`cp_decode`'s assert shifts by `len = 32 - (key & 0xF)`, which is `32` whenever
`key & 0xF == 0`. gcc compiles both sides as 32-bit variable shifts
(`shr %cl, %esi`, verified in the disassembly of the reference `.so`), so the
count is taken modulo 32; the translation uses `wrapping_shr` to match.

| # | function | assert | trigger reachable through the FFI boundary? | expected result (default / `--no-default-features`) | test |
|---|----------|--------|---------------------------------------------|------------------------------------------------------|------|
| 22 | `cp_read_bits` | `s->bits_left > 0` | yes — any stream that is exhausted while the decoder still wants bits (`in_bytes == 0`, `in_bytes < 0`, a truncated stored-block header) | `SIGABRT` + ``cp_read_bits: Assertion `s->bits_left > 0' failed.`` / keeps decoding from the zero-filled `bits` register | `err22_assert_bits_left`, `err14`, `err15`, `err16`, `err22to27_all_reachable_assert_sites` |
| 23 | `cp_read_bits` | `!cp_would_overflow(s, n)` i.e. `(bits_left + count) - n >= 0` | yes — needs `2*count + last_bytes*8 < n`, i.e. the `final_word` path with a small `count`; hit by the scan | `SIGABRT` + ``!cp_would_overflow(s, num_bits_to_read)`` / same | `err22to27_all_reachable_assert_sites` |
| 24 | `cp_consume_bits` | `s->count >= num_bits_to_read` | yes — e.g. the two-byte input `00 00`: the stored block's alignment read leaves `count == 8` and the 16-bit `LEN` read has nothing left to load | `SIGABRT` + ``s->count >= num_bits_to_read`` / `count` goes negative and zeros are shifted in | `err24_assert_count`, `err22to27_…` |
| 25 | `cp_ptr` ← `cp_stored` | `!(s->bits_left & 7)` | yes — reached by the hand-derived 11-byte stream in the test: a non-final btype-1 block with six 8-bit literals makes `count == 13` exactly when the final partial word is loaded, so `count += s->bits_left` puts `count` and `bits_left` permanently out of step by 13; the following stored block then sees `count & 7 == 0`, skips re-alignment and reaches `cp_ptr` with `bits_left == -5` | `SIGABRT` + ``!(s->bits_left & 7)`` / `cp_ptr` returns a pointer computed from the mis-scaled `count/8` | `err25_assert_cp_ptr_alignment` |
| 26 | `cp_build` | `len < 16` (`len = lens[i]`) | yes — **not** from stream data (`cp_dynamic` can only store `cp_decode` results, and a `cp_decode` result that is not a real tree entry trips row 27 first), but directly through the writable exported table `cp_fixed_table`, which `cp_fixed` hands to `cp_build` unchecked | `SIGABRT` + ``len < 16`` / undefined behaviour: `counts[lens[n]]++` writes past `int counts[16]` | `err26_assert_build_len_via_fixed_table_override` |
| 27 | `cp_decode` | `(search >> len) == (key >> len)`, `len = 32 - (key & 0xF)` | yes — any incomplete / over-subscribed Huffman table, and always when the binary search ends at `lo == 0` and reads `tree[-1]` | `SIGABRT` + ``(search >> len) == (key >> len)`` / returns `(key >> 4) & 0xFFF` from whatever `tree[lo-1]` held | `err22to27_…`, `cfg58_decode_reads_tree_minus_one` |
| 28 | `cp_peak_bits` | `s->word_index <= s->word_count` | **no** — `word_index` is only incremented inside `if (s->word_index < s->word_count)`, so the assert can never fail. Kept for completeness. | n/a | `err28_unreachable_asserts` (documents & proves the invariant) |
| 29 | `cp_read_bits` | `num_bits_to_read <= 32` | **no** — every call site passes a literal ≤ 16, `count & 7` (0..7), `cp_len_extra_bits[]` (≤ 5), `cp_dist_extra_bits[]` (≤ 13) or `key & 0xF` (≤ 15). | n/a | `err28_unreachable_asserts` |
| 30 | `cp_read_bits` | `num_bits_to_read >= 0` | **no** — the only non-literal argument that could be negative is `s->count & 7`, and C's `&` on a negative `int` still yields `0..7`. | n/a | `err28_unreachable_asserts` |
| 31 | `cp_read_bits` | `s->count <= 64` | **no** — `count` only grows in `cp_peak_bits`, which requires `count < num_bits_to_read <= 16` first, and then adds either 32 or `bits_left = count + last_bytes*8 <= 15 + 24`. | n/a | `err28_unreachable_asserts` |

## E. `return NULL` sites in dead code

| # | function | trigger | expected C result | test |
|---|----------|---------|-------------------|------|
| 32 | `cp_chunk` | `memcmp(start+4, chunk, 4) != 0` **or** `len < minlen` | `NULL` | not reachable: `cp_chunk` is `static` and has **no** caller in `c_src/src/lib.c` (dead code kept by the translation for completeness). Verified unreachable by `err33_dead_code_not_exported`, which asserts the symbols are absent from **both** `.so`s. |
| 33 | `cp_chunk` | `png->p + (len + 12) > png->end` | `NULL` | ditto |
| 34 | `cp_find` | no chunk with the requested id / `minlen` before `png->end` | `NULL` | ditto |

## F. The C source's own out-of-bounds stack accesses

| # | site | trigger | behaviour | test |
|---|------|---------|-----------|------|
| 35 | `cp_dynamic`: `lens[n]` for `n >= 320` | a corrupt dynamic block whose code-length run pushes `n` past the end of `uint8_t lens[288+32]` | in the reference build gcc lays the frame out as shown below, so at `n == 364` the loop zeroes **its own counter** and the C library **spins forever** (confirmed by sampling `RIP` from a `SIGALRM` handler: `cp_dynamic+0x1f8..0x211`, the `case 18` loop). At `n >= 348` it overwrites `sym`/`nlen`/`ndst`/`nlit` instead, which changes the Huffman tables that get built. Not reproducible by any translation. | `err35_lens_overshoot_hangs_the_c_library` |
| 36 | `cp_dynamic`: `lens[-1]` (code-length symbol 16 as the very first symbol) | a corrupt dynamic block that starts its code-length sequence with "repeat previous" | in the reference frame that byte is the most-significant byte of the saved `s` pointer, i.e. always `0x00` for an x86-64 heap address — the same value the translation's zero-filled slack byte yields, so the two agree in practice | `err36_lens_minus_one_read` |

```text
  gcc 11.5 -O0 -fPIC frame of cp_dynamic (from objdump of the reference .so):
  -0x188  the saved `s` parameter    <- lens[-1] is its top byte, always 0x00
  -0x180  uint8_t lens[320]
  -0x40   uint8_t lenlens[19]        == lens[320..339]   (already consumed: harmless)
  -0x24   int sym                    == lens[348..352]
  -0x20   int nlen                   == lens[352..356]
  -0x1c   int ndst                   == lens[356..360]
  -0x18   int nlit                   == lens[360..364]
  -0x14   int i   (case 18 counter)  == lens[364..368]   <- infinite loop
  -0x8    int n   (outer counter)    == lens[376..380]
```

## Documented, unavoidable divergences (UB in the C source)

These are *not* rows to be "fixed" — they are inputs on which the C library
itself has undefined behaviour, so no translation can be byte-identical.

The test harness contains a mechanical **UB oracle** for exactly this
(`common::is_layout_dependent`): it builds the *same unmodified*
`c_src/src/lib.c` a second time with
`-fstack-protector-all -fno-omit-frame-pointer --param=ssp-buffer-size=1`,
which only moves the function-local variables around. For an input on which the
C code is well defined the two C builds must agree, because nothing observable
can depend on the frame layout. Every divergence the fuzzers find is run past
this oracle, and the run only passes if **all** of them are layout-dependent.

Measured on the default (assert-enabled) configuration:

| fuzz target | cases | layout-dependent (tolerated) | unexplained |
|-------------|-------|------------------------------|-------------|
| `fuzz01_random_bytes` | 1200 | 1 | **0** |
| `fuzz02_longer_random_bytes` | 500 | 1 | **0** |
| `fuzz03_truncated_valid_streams` | 2581 | 0 | **0** |
| `fuzz04_mutated_valid_streams` | 1200 | 35 | **0** |
| `fuzz05_random_dynamic_headers` | 400 | 0 | **0** |
| `fuzz06_unfilter_random_arguments` | 4000 | 0 | **0** |

| C site | UB | why unreachable for well-formed input |
|--------|----|----------------------------------------|
| `cp_build`: `counts[lens[n]]++`, `codes[len]`, `first[len]` with `len >= 16` | OOB stack access | `cp_dynamic` only ever stores `cp_decode` results `0..15` (symbols 16/17/18 are handled separately); `cp_fixed` uses `cp_fixed_table`, whose initialiser only holds 5/7/8/9 — reachable only by *overwriting* that exported table (row 26) |
| `cp_dynamic`: `lens[-1]`, `lens[n >= 320]` | OOB stack access | rows 35, 36 |
| `cp_dynamic`: `lenlens[cp_permutation_order[i]]` with a table entry `>= 19` | OOB stack write | `cp_permutation_order`'s initialiser is a permutation of `0..18`; a caller that overwrites the exported table with a larger value makes the C code scribble over `cp_dynamic`'s frame. `CONFIGS.md` row 50 therefore only permutes within `0..18` |
| `cp_decode`: `tree[lo - 1]` with `lo == 0` | read of the `u32` before `lit`/`dst`/`len` **inside `cp_state_t`** — the translation reproduces this exactly (same `#[repr(C)]` layout, verified against a C probe in `sym06_cp_state_t_layout_matches_c`; sub-array pointers derived from the same allocation), so it is **not** a divergence | — |
| `cp_block`: `cp_len_extra_bits[symbol]` / `cp_dist_base[distance_symbol]` for a garbage `cp_decode` result | OOB read of a neighbouring global; the two `.so`s lay their globals out differently | `cp_decode` on a real tree entry returns a symbol index `< 288` (lit) / `< 32` (dist), so the indices stay in range; a garbage entry trips row 27's assert first |
| `cp_stored`: `memcpy(s->out, p, LEN)` | unchecked write past `out_end` and read past `in + in_bytes` | row 20 — both libraries do the same thing at the same offsets, so it *is* testable as long as the surrounding bytes are identical, which the shared-memory scratch guarantees |

### A note on `cp_ptr`'s source pointer for stored blocks

`cp_stored` copies from `cp_ptr(s) = (char *)(s->words + s->word_index) - s->count / 8`,
which is the true data position **only** while the final partial input word has
not been loaded yet — `cp_peak_bits` adds `s->bits_left` (not `last_bytes * 8`)
to `count` when it loads it, permanently over-counting by the `count` it had at
that moment. So a stored block copies the announced `LEN` bytes from an offset
that is short by `Δ/8` bytes whenever that path was taken. `CONFIGS.md` row 38
therefore only asserts the *decompressed content* when
`(in_bytes - first_bytes) % 4 == 0` (so `final_word_available` stays 0) and
compares C against Rust unconditionally otherwise.
