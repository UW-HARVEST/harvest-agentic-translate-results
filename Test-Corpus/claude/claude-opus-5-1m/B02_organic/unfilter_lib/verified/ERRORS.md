# ERRORS.md — error-surface table (Phase A / Phase C)

Derived mechanically from `c_src/src/lib.c` with

```sh
grep -n 'assert\|cp_error_reason *=\|return 0\|return -1\|return NULL\|goto cp_err' c_src/src/lib.c
```

Every `assert()` in `lib.c` is **live**: `c_src/CMakeLists.txt` sets no build
type and never defines `NDEBUG`, so a failing assert calls glibc
`__assert_fail()` → message on `stderr` + `SIGABRT` (signal 6). The Rust port
reproduces this through `src/cassert.rs`.

## How the error paths are compared

Every call goes through `tests/common/mod.rs`:

* the call runs in a **forked child** whose output buffer is a `MAP_SHARED`
  mapping surrounded by `PROT_NONE` guard pages, so an over/underrun becomes a
  deterministic `SIGSEGV` at the same offset in both libraries;
* the **same** input mapping (zero-filled outside the stream) is handed to both
  libraries, so even the C code's deliberate out-of-bounds head/tail reads see
  identical bytes;
* compared per call: `WTERMSIG`, exit status, return value, the
  NUL-terminated `cp_error_reason` string read back through `dlsym`, the whole
  output mapping, and the child's `stderr`;
* `stderr` normalisation: glibc prints `<prog>: <__FILE__>:<line>: <func>:
  Assertion `<expr>' failed.` and CMake compiles with an **absolute**
  `__FILE__`, so everything up to the last `lib.c` is stripped and
  `lib.c:<line>: <func>: Assertion `<expr>' failed.` is compared verbatim.
  `tests/assert_message.rs` proves that the stripped part is the *only*
  difference — the `<prog>: ` prefix (glibc's `__progname` vs.
  `basename(argv[0])` in `src/cassert.rs`) is byte-identical, and the C path is
  just the build directory prepended to the same `c_src/src/lib.c`:

  ```
  C    "zz: /abs/path/to/translated_rust/c_src/src/lib.c:115: cp_consume_bits: Assertion `s->count >= num_bits_to_read' failed.\n"
  Rust "zz: c_src/src/lib.c:115: cp_consume_bits: Assertion `s->count >= num_bits_to_read' failed.\n"
  ```

* an independent transcription of `lib.c` (`tests/common/cmodel.rs`) is a third
  opinion on every input, so a row cannot pass because both libraries are
  *equally* wrong about what the C source says.

## A. `assert()` rejections

| # | function (line) | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|-----------------|------------------------------------------|-------------------|------|---|
| E1 | `cp_ptr` (95) `!(s->bits_left & 7)` | The 15-byte stream built by `inflate_errors.rs::cp_ptr_unaligned_stream()`: a static block with 3 eight-bit + 6 nine-bit literal codes, so that the *final partial word* is loaded (`count += s->bits_left`) with 81 consumed bits — not a multiple of 8 — which desynchronises `count` from `bits_left`; the following stored block's alignment read then eats 4 instead of 5 bits, `LEN`/`NLEN` are read one bit early (`FF 7F 00` makes them complements anyway) and `bits_left` ends at `-7`. | `SIGABRT`, `lib.c:95: cp_ptr: Assertion \`!(s->bits_left & 7)' failed.` | `inflate_errors.rs::e1_cp_ptr_unaligned_bits_left` | [x] |
| E2 | `cp_peak_bits` (104) `s->word_index <= s->word_count` | **provably unreachable**: `word_index` is incremented only inside `if (s->word_index < s->word_count)`, so after `++` it is `<= word_count`. | n/a | `inflate_errors.rs::e2_unreachable_word_index_invariant` (asserts the class never occurs over the 7 560-input truncation sweep) | [x] |
| E3 | `cp_consume_bits` (115) `s->count >= num_bits_to_read` | Huffman code longer than the buffered bits at EOF: the 1-byte input `[0x03]` (BFINAL=1, BTYPE=01) leaves 5 bits for a 7-bit code. 593 further instances in the truncation fuzzer. | `SIGABRT`, `lib.c:115: cp_consume_bits: Assertion \`s->count >= num_bits_to_read' failed.` | `inflate_errors.rs::e3_consume_bits_underflow` | [x] |
| E4 | `cp_read_bits` (123) `num_bits_to_read <= 32` | `cp_len_extra_bits` is a **writable exported global**; setting `cp_len_extra_bits[0] = 33` (also 64/128/255) and decoding length symbol 257 passes that value to `cp_read_bits`. No non-mutating input can reach it: all call sites pass 0…16 or `count & 7`. | `SIGABRT`, `lib.c:123: cp_read_bits: Assertion \`num_bits_to_read <= 32' failed.` | `inflate_errors.rs::e4_num_bits_gt_32_via_global` | [x] |
| E5 | `cp_read_bits` (124) `num_bits_to_read >= 0` | **unreachable through the ABI**: the argument is either a literal (1,2,3,4,5,7,16), `s->count & 7` (`count` is never negative) or a `uint8_t` table entry — never negative as an `int`; even `255` trips line 123 first (shown by E4). | n/a | `inflate_errors.rs::e5_unreachable_negative_num_bits` (+ exercises the reachable boundary `num_bits_to_read == 0`, which really happens for an already-aligned stored block) | [x] |
| E6 | `cp_read_bits` (125) `s->bits_left > 0` | input exhausted: `in_bytes == 0` (any alignment) and `in_bytes ∈ -8..-1`. For large negative `in_bytes` the `final_word` loop reads `in[in_bytes - last_bytes]`, i.e. far before `in`, and faults first (also asserted). 180 further instances in the truncation fuzzer. | `SIGABRT`, `lib.c:125: cp_read_bits: Assertion \`s->bits_left > 0' failed.` | `inflate_errors.rs::e6_zero_and_negative_in_bytes` | [x] |
| E7 | `cp_read_bits` (126) `s->count <= 64` | **unreachable**: `cp_peak_bits` tops up only while `count < num_bits_to_read <= 16`, so `count <= 15 + 32 = 47` after a word load and `count <= 15 + bits_left` after the final-word load, where E8/E6 fire first. | n/a | `inflate_errors.rs::e7_unreachable_count_gt_64` | [x] |
| E8 | `cp_read_bits` (127) `!cp_would_overflow(s, num_bits_to_read)` | more bits requested than exist in the whole stream, e.g. the 3-byte prefix of the dynamic-block stream (`dynamic[..3]`); 28 further instances in the truncation fuzzer and 12 in the random fuzzer. | `SIGABRT`, `lib.c:127: cp_read_bits: Assertion \`!cp_would_overflow(s, num_bits_to_read)' failed.` | `inflate_errors.rs::e8_would_overflow` | [x] |
| E9 | `cp_build` (154) `len < 16` | a code length ≥ 16 reaches `cp_build`. Only reachable by writing the exported `cp_fixed_table` global (`cp_fixed_table[0] = 16` / `17`); the dynamic path can only produce 0…15. `15` is checked as the passing boundary. | `SIGABRT`, `lib.c:154: cp_build: Assertion \`len < 16' failed.` | `inflate_errors.rs::e9_code_length_ge_16_via_global` | [x] |
| E10 | `cp_decode` (217) `(search >> len) == (key >> len)` | incomplete Huffman tree + a bit pattern that is no code's prefix: a dynamic block whose literal alphabet has a **single** 1-bit code (Kraft ½) followed by the bit `1`. The bit `0` (= that code) is checked as the passing case. 433+224+161 further instances in the fuzzers. | `SIGABRT`, `lib.c:217: cp_decode: Assertion \`(search >> len) == (key >> len)' failed.` | `inflate_errors.rs::e10_decode_key_mismatch` | [x] |

## B. `cp_error_reason` + `return 0` rejections

| # | function (line) | trigger (exact invalid input/condition) | expected C result | test | ✔ |
|---|-----------------|------------------------------------------|-------------------|------|---|
| E11 | `cp_stored` (176-182) | stored block with `LEN != (uint16_t)~NLEN` (60 randomized single-bit corruptions of `NLEN`, plus `LEN == NLEN == 0`) | `cp_inflate` → `0`, `cp_error_reason = "Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | `inflate_errors.rs::e11_len_nlen_mismatch` | [x] |
| E12 | `cp_stored` (184-191) | stored block with `s->bits_left / 8 > LEN` — i.e. **more input left than `LEN`** (the C test is `bits_left/8 <= LEN`), so any stored block that does not run to the very end of the input | `cp_inflate` → `0`, `cp_error_reason = "Stored block extends beyond end of input stream."` | `inflate_errors.rs::e12_stored_beyond_end` | [x] |
| E13 | `cp_block` (259-267) | literal symbol decoded while `s->out + 1 > s->out_end` (`out_bytes` 0/1/2 for a 3-literal block) | `cp_inflate` → `0`, `cp_error_reason = "Attempted to overwrite out buffer while outputting a symbol."` | `inflate_errors.rs::e13_out_full_on_literal` | [x] |
| E14 | `cp_block` (278-286) | length/distance pair reaching before the start of `out`: first symbol of the stream is a match (distances 1, 2, 5, 100, 32768), and "1 byte written, distance 2". `distance == bytes written` is checked as the passing boundary. | `cp_inflate` → `0`, `cp_error_reason = "Attempted to write before out buffer (invalid backwards distance)."` | `inflate_errors.rs::e14_backwards_distance` | [x] |
| E15 | `cp_block` (287-295) | length/distance pair with `s->out + length > s->out_end` (60 randomized `pre`/`dist`/`length`/`out_bytes` combinations) | `cp_inflate` → `0`, `cp_error_reason = "Attempted to overwrite out buffer while outputting a string."` | `inflate_errors.rs::e15_out_full_on_match` | [x] |
| E16 | `cp_inflate` (360-368) | `BTYPE == 3` (reserved), with `BFINAL` 0 and 1 | `cp_inflate` → `0`, `cp_error_reason = "Detected unknown block type within input stream."` | `inflate_errors.rs::e16_btype_3` | [x] |
| E17 | `unfilter` (439-440) | row 0 filter byte ∉ {0,1,2,3,4}: 5, 6, 0x0f, 0x7f, 0x80, 0xfe, 0xff × `h ∈ 1…4` × `bpp ∈ 1…4` | `unfilter` → `0`; **no** scanline byte modified (asserted) | `unfilter_errors.rs::e17_row0_bad_filter` | [x] |
| E18 | `unfilter` (473-474) | filter byte of some row `y >= 1` ∉ {0,1,2,3,4}, for every `y ∈ 1..h`, `h ∈ 2…5`, `bpp ∈ {1,3,4}` | `unfilter` → `0`; rows before `y` already de-filtered in place (compared byte-for-byte) | `unfilter_errors.rs::e18_rowy_bad_filter` | [x] |
| E19 | `cp_chunk` (403) / `cp_find` (415) `return 0` | `static` with no caller from any exported entry point → dead code in both libraries (translated in `src/misc.rs` for completeness). Unreachable across the FFI boundary. | n/a | `symbols.rs::no_png_chunk_symbols_exported` (proves neither `.so` exposes them, so no caller can reach them) | [x] |

## C. Generic FFI boundary conditions

| # | entry point | trigger | expected C result | test | ✔ |
|---|-------------|---------|-------------------|------|---|
| E20 | `unfilter` | `h ∈ {0, -1, -2, -1000, INT_MIN+1}` — the `if (h > 0)` guard means `*raw` is never even read, so even an invalid filter byte is accepted | `1`, buffer bit-identical to the input | `unfilter_errors.rs::e20_non_positive_h` | [x] |
| E21 | `unfilter` | `raw == NULL`, `h <= 0` | `1`, no dereference | `unfilter_errors.rs::e21_e22_null_raw` | [x] |
| E22 | `unfilter` | `raw == NULL`, `h > 0` (1, 2, 5) | `SIGSEGV` (unconditional `*raw++`) | `unfilter_errors.rs::e21_e22_null_raw` | [x] |
| E23 | `unfilter` | `w == 0` / `bpp == 0` ⇒ `len == 0`; filter bytes are still validated, and for `bpp > len` the `for (x = 0; x < bpp; x++)` prologues of filters 2/3/4 write past the (empty) scanline — including over the *next* row's filter byte | same return value and same buffer | `unfilter_errors.rs::e23_zero_len` | [x] |
| E24 | `unfilter` | `w < 0` or `bpp < 0` ⇒ negative `len`, pointers walk backwards (8 sign combinations × `h ∈ 1…4` × filter ∈ {0,1,2,3,4,9}); run in a forked child on a **doubly** guarded mapping so under- and overruns fault identically | same return value and same mapping bytes | `unfilter_errors.rs::e24_negative_w_bpp` | [x] |
| E25 | `unfilter` | filter byte exactly one past the valid range (`5`) in every row, plus 6, 0x7f, 0x80, 0xff | `0` | `unfilter_errors.rs::e25_one_past_valid_filter`, `e17_row0_bad_filter`, `e18_rowy_bad_filter` | [x] |
| E26 | `cp_inflate` | `in == NULL`, `in_bytes ∈ {1,2,3,4,8,64}` | `SIGSEGV` (`words[0]` resp. the final-partial-word loop); `in_bytes == 0` never dereferences `in` and aborts at `lib.c:125` instead (also asserted) | `inflate_errors.rs::e26_null_in` | [x] |
| E27 | `cp_inflate` | `out == NULL`, `out_bytes == 0`, stream that emits a literal | `0` + `"…outputting a symbol."` (the check precedes any store) | `inflate_errors.rs::e13_out_full_on_literal` | [x] |
| E28 | `cp_inflate` | `out_bytes < 0` (`-1`, `-7`, `-4096`, `INT_MIN/2`) ⇒ `out_end < out` | `0` + `"…outputting a symbol."` | `inflate_errors.rs::e28_negative_out_bytes` | [x] |
| E29 | `cp_inflate` | **every** truncation (`1..len`) of 15 valid streams × 3 alignments × 3 `out_bytes` = 7 560 inputs; the reachable outcome classes are established mechanically instead of guessed | identical return / signal / output / message for every one | `inflate_errors.rs::e29_all_truncations_and_assert_coverage`, `sweep_libraries_agree_on_every_input` | [x] |
| E30 | `cp_inflate` | the whole 2-bit `BTYPE` "enum": 0 (stored), 1 (static), 2 (dynamic) decode; 3 (the value with no valid variant) is rejected | 0/1/2 → `1`; 3 → `0` + `"Detected unknown block type…"` | `inflate_errors.rs::e30_all_btype_values` | [x] |
| E31 | `unfilter` | **all 256** filter-byte values (the moral equivalent of an out-of-range enum crossing the FFI boundary), in row 0 and in row 1 | 0…4 succeed with the documented per-filter arithmetic, 5…255 → `0` | `unfilter_errors.rs::e31_all_256_filter_bytes` | [x] |
| E32 | `cp_inflate` | randomized fuzzing: 1 200 uniform random byte strings (lengths 1…40), 720 bit-flipped valid streams, 1 052 truncations, 1 500 structurally-plausible random streams (random Huffman tables) — 4 472 inputs × 2 libraries, each also checked against the independent model | identical in all three implementations | `inflate_fuzz.rs::fuzz_random_streams`, `fuzz_bitflipped_valid_streams`, `fuzz_truncated_valid_streams`, `fuzz_structured_streams` | [x] |

Outcome classes actually produced by the sweep (printed by
`e29_all_truncations_and_assert_coverage`) — every one compared between the two
`.so`s and the model:

```
abort lib.c:95  cp_ptr            abort lib.c:115 cp_consume_bits
abort lib.c:125 cp_read_bits      abort lib.c:127 cp_read_bits
abort lib.c:217 cp_decode         ok
ret0 Attempted to overwrite out buffer while outputting a string.
ret0 Attempted to overwrite out buffer while outputting a symbol.
ret0 Attempted to write before out buffer (invalid backwards distance).
ret0 Detected unknown block type within input stream.
ret0 Failed to find LEN and NLEN as complements within stored (uncompressed) stream.
ret0 Stored block extends beyond end of input stream.
```

The fuzzers add `SIGSEGV` (a stored block whose `LEN` runs past the output
mapping — `cp_stored` has no output bounds check).

## D. Conditions that are **C undefined behaviour**

`lib.c` performs a few operations whose observable effect depends on stack
layout / stack garbage rather than on the input, so there is no defined C result
for the Rust port to reproduce. They are **detected mechanically** by the
independent model (`tests/common/cmodel.rs`), which flags:

| C operation (UB) | source | detected as |
|------------------|--------|-------------|
| `lens[n]` written for `n >= 288 + 32` — a `16`/`17`/`18` run overshooting `nlit + ndst` writes up to 137 bytes past `uint8_t lens[288+32]` | `lib.c:230-250` | `cp_dynamic: lens[n] written for n >= 320, …` |
| `lens[-1]` read when the first code-length symbol is `16` (uninitialised stack) | `lib.c:234-237` | `cp_dynamic: lens[-1] read (uninitialised stack)` |
| `counts[lens[n]]++` with `lens[n] >= 16` (past `int counts[16]`) | `lib.c:142-143` | `cp_build: counts[N]++ past int counts[16]` |
| `cp_len_extra_bits[symbol-257]` / `cp_dist_*[dsym]` read past the tables when a garbage Huffman key decodes outside `257..=287` / `0..=31` | `lib.c:273-277` | `cp_block: cp_len_extra_bits[N] past uint8_t[29+2]` |

Inputs flagged this way are **counted and reported**, not compared:

* truncation sweep: 72 of 7 560 inputs (0.95 %),
* random fuzzer: 23 of 1 200, structured fuzzer: 45 of 1 500,
* bit-flip fuzzer: 0, truncation fuzzer: 0.

Each fuzz row additionally asserts that fewer than 25 % of its inputs are
UB-tainted, so the rows cannot "pass" by skipping everything. The **7 488
well-defined sweep inputs and all 4 400+ well-defined fuzz inputs produce
byte-identical results in the C `.so`, the Rust `.so` and the model.**

Two further C-UB conditions are simply not fed to the libraries, because their
"result" is compiler-dependent rather than merely stack-dependent:

* `unfilter`: `w * bpp` signed-overflow, and any `w/h/bpp` that makes the
  scanline walk leave the caller's allocation by more than the test padding;
* mutating `cp_permutation_order` to a value `> 18` (`lenlens[order[i]]` would
  index past `uint8_t lenlens[19]`), so `i29_mutate_permutation_order` only ever
  installs genuine permutations of `0…18`.

Note that `cp_inflate` reading `in[i]` outside `0..in_bytes` (the unaligned head,
the final partial word and `cp_stored`'s source pointer) is *not* in this list:
the harness hands the **same** page-guarded, zero-filled mapping to both
libraries, so those reads are perfectly deterministic and are compared.
