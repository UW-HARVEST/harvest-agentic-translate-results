# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Every `cp_error_reason = ...` /
`goto cp_err`, every `assert(...)`, and every implicit rejection is one row.

Tests: `translation/tests/errors.rs`.

## Build configuration note (critical)

`c_src/CMakeLists.txt` sets **no** `CMAKE_BUILD_TYPE`, so `CMAKE_C_FLAGS` is
empty and **`NDEBUG` is NOT defined**. Confirmed by `nm -D`:

```
U __assert_fail@GLIBC_2.2.5
```

All 10 `assert()`s are therefore live in the ground-truth library, and a
violated assert is observable behaviour: glibc prints to stderr and calls
`abort()` → the process dies with `SIGABRT` (6). The Rust translation
reproduces this via `cp_assert`, which aborts at the same point. The
differential harness runs every call in a `fork()`ed child so an abort is a
comparable outcome rather than the end of the test run, and it captures the
child's stderr — glibc's assert message names the exact `lib.c` line, which is
how each assert row below is pinned to the assert it claims to trigger.

Legend for "expected C result":

* `ret 0 + reason` — `pinflate` returns `0` and `cp_error_reason` holds the
  listed string.
* `SIGABRT` — `assert()` fails → `__assert_fail` → `abort()`.
* `SIGALRM` — does not terminate (the harness arms a 40 ms watchdog).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `cp_stored` (`lib.c:176-183`) | `btype==0` stored block whose `LEN != (uint16_t)~NLEN` | `ret 0` + `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | tested |
| 2 | `cp_stored` (`lib.c:185-190`) | `btype==0`, LEN/NLEN consistent, but `s->bits_left/8 > LEN` (more input bytes remain than the block declares) | `ret 0` + `"Stored block extends beyond end of input stream."` | tested |
| 3 | `cp_block` (`lib.c:259-267`) | literal symbol (`sym < 256`) decoded while `s->out + 1 > s->out_end` | `ret 0` + `"Attempted to overwrite out buffer while outputting a symbol."` | tested |
| 4 | `cp_block` (`lib.c:278-286`) | length/distance pair with `s->out - backwards_distance < s->begin` | `ret 0` + `"Attempted to write before out buffer (invalid backwards distance)."` | tested |
| 5 | `cp_block` (`lib.c:287-295`) | length/distance pair with `s->out + length > s->out_end` | `ret 0` + `"Attempted to overwrite out buffer while outputting a string."` | tested |
| 6 | `pinflate` (`lib.c:360-367`) | block header with `btype == 3` (reserved) | `ret 0` + `"Detected unknown block type within input stream."` | tested |
| 7 | `cp_read_bits` (`lib.c:125`) | `assert(s->bits_left > 0)` — a bit is requested with the input exhausted: `in_bytes == 0`, `in == NULL`, negative `in_bytes`, or any truncated stream | `SIGABRT` | tested (C stderr pinned to `lib.c:125`) |
| 8 | `cp_read_bits` (`lib.c:127`) | `assert(!cp_would_overflow(s, n))` i.e. `(bits_left + count) - n < 0`. Reached e.g. by a 2-byte input at `ptr%4 == 3` selecting a stored block: after the alignment read `count == 0` while `bits_left == 8`, so the 16-bit LEN read trips it | `SIGABRT` | tested (pinned to `lib.c:127`) |
| 9 | `cp_read_bits` (`lib.c:123`) | `assert(num_bits_to_read <= 32)` | `SIGABRT` | **unreachable** — every call site passes a literal ≤ 16 (`1, 2, 3, 4, 5, 7, 16`) or `s->count & 7` (≤ 7) or a table entry (`cp_len_extra_bits` ≤ 5, `cp_dist_extra_bits` ≤ 13) |
| 10 | `cp_read_bits` (`lib.c:124`) | `assert(num_bits_to_read >= 0)` | `SIGABRT` | **unreachable** — same argument; all call sites are non-negative |
| 11 | `cp_read_bits` (`lib.c:126`) | `assert(s->count <= 64)` | `SIGABRT` | **unreachable** — `count` only grows by `+= 32` from below `num_bits_to_read ≤ 16`, or by `+= bits_left` in the final-word branch, and is capped by the 64-bit `bits` register's accounting |
| 12 | `cp_consume_bits` (`lib.c:115`) | `assert(s->count >= num_bits_to_read)` — `cp_decode` calls `cp_consume_bits(s, key & 0xF)` with **no** guard, and `cp_read_bits`'s `cp_would_overflow` test double-counts (`bits_left + count`), so a 16-bit read can pass it and still find too few buffered bits | `SIGABRT` | tested (pinned to `lib.c:115`); also the most frequent outcome of the truncation fuzz (732/853) |
| 13 | `cp_peak_bits` (`lib.c:104`) | `assert(s->word_index <= s->word_count)` | `SIGABRT` | **unreachable** — guarded by `if (s->word_index < s->word_count)` immediately above the post-increment |
| 14 | `cp_ptr` (`lib.c:95`) | `assert(!(s->bits_left & 7))` | `SIGABRT` | **unreachable**, proven: `bits_left = 8*in_bytes - consumed`, and when `cp_ptr` runs `consumed == 3 + (count & 7) + 32`. `count ≡ -consumed (mod 8)` holds through the header because every refill adds a multiple of 8 (`+= 32`, or `+= bits_left` which happens only at `consumed == 0` since `count` starts at `first_bytes*8 ∈ {0,8,16,24}` and can never be 1). Hence `count & 7 == 5`, `consumed == 40`, and `bits_left` is always a multiple of 8. `tests/errors.rs::row14_...` searches `in_bytes 1..24 × align 0..4` and reports 0 hits, consistent with the proof |
| 15 | `cp_build` (`lib.c:154`) | `assert(len < 16)` — a code length ≥ 16 in the length array | `SIGABRT` | **unreachable from the bit stream**: `cp_dynamic` fills `lens[]` from `cp_decode` over the 19-symbol code-length tree, whose written entries carry symbol indices 0..18, and 16/17/18 are consumed by the `switch`, so `lens[] ∈ 0..15`. It **is** reachable through the exported `cp_fixed_table`, which a caller may write to — tested for values 16, 17, 31, 255, all `SIGABRT` with C stderr `lib.c:154: cp_build: Assertion 'len < 16' failed` |
| 16 | `cp_decode` (`lib.c:217`) | `assert((search >> (32-(key&0xF))) == (key >> (32-(key&0xF))))` — the bit pattern is not a code in the tree. **Not** reachable from a fixed block (the fixed tree is a complete code over all 288 symbols, so every pattern decodes); reachable from a dynamic block with an incomplete literal tree, and whenever `hi == 0` so that `tree[-1]` is read with `key & 0xF == 0` (then `32 - 0 == 32`, and gcc emits `shr %cl` whose count is masked to 5 bits, degenerating the test to `search == key`) | `SIGABRT` | tested with a hand-built incomplete dynamic tree (pinned to `lib.c:217`) plus dynamic fuzzing |
| 17 | `pinflate` (`lib.c:315`) | `calloc` returns `NULL`; the result is not checked and `s->bits = 0` dereferences it | `SIGSEGV` | untestable (OOM only). Rust differs here by design: the global allocator aborts on failure rather than returning NULL |
| 18 | `pinflate` | `in == NULL`. `first_bytes = 0`, `words = NULL`, `word_count = in_bytes/4`. With `in_bytes == 0` row 7 fires before any dereference | `SIGABRT` | tested |
| 19 | `pinflate` | `out == NULL` with `out_bytes == 0`: `out_end == begin == NULL`, so a literal hits row 3 with no write; an empty block succeeds and returns 1 | `ret 0` + row-3 reason, / `ret 1` | tested (both sub-cases) |
| 20 | `pinflate` | negative `in_bytes`: `bits_left = in_bytes*8 <= 0`, so the first `cp_read_bits` asserts before the (also out-of-range) `final_word` loop can matter | `SIGABRT` | tested for `-1, -2, -3, -4, -17, i32::MIN/8` |
| 21 | `pinflate` | negative `out_bytes`: `out_end = out + out_bytes < begin`, so the first literal hits row 3; an empty block still returns 1 | `ret 0` + row-3 reason / `ret 1` | tested for `-1, -2, -1000` |
| 22 | `pinflate` | `btype` outside 0..3 | n/a | **impossible** — `btype = cp_read_bits(s, 2)` is masked to two bits, so the `switch` is exhaustive and Rust's `_ => {}` arm is dead. Proven by `every_block_type_value`, which drives all four. The equivalent "out-of-range value across the FFI boundary" for this API is an out-of-range `int` size, covered by rows 20/21 and by `out_of_range_int_parameters` (the cross product of `in_bytes ∈ {0,1,2,3,n-1,n,-1,i32::MIN}` and `out_bytes ∈ {0,1,2,3,1024,-1,i32::MIN,i32::MAX}`) |
| 23 | `pinflate` | `in_bytes == 0` and `out_bytes == 0` with valid pointers | `SIGABRT` (row 7) | tested |
| 24 | `pinflate` | `in_bytes` larger than the real allocation | `SIGSEGV` / garbage-driven | excluded: reads unmapped memory, no defined behaviour to match. (The harness does place a 64 KiB zeroed guard after the input so that `cp_stored`'s unbounded `memcpy` — see row 25 — reads defined bytes.) |
| 25 | `cp_dynamic` + `cp_block` | **non-termination.** The repeat opcodes have no bound check on `n`, so `lens[288+32]` overflows into `cp_dynamic`'s neighbouring locals; once a write lands on `n` itself (`%rbp-0x8`, i.e. `lens[376]`) `n` is reset (`0x178 & ~0xff | v`) and the code-length loop restarts. `pinflate` never returns | `SIGALRM` under the harness watchdog | tested — `fuzz_structured_dynamic_blocks` generated 21 411 overshooting programs across three seeds, all matching. This is why `src/lib.rs` reproduces gcc's `cp_dynamic` frame layout byte for byte |
| 26 | `cp_stored` (`lib.c:194`) | `memcpy(s->out, p, LEN)` is bounded by **neither** buffer: `LEN` can be up to 65535 regardless of `out_bytes`, and `cp_ptr` can point up to 8 bytes before `in` | out-of-bounds read and write | tested differentially with zeroed guard regions on both buffers (the copied bytes and the 64 KiB past the compared out region — hashed — must match) |

## Checklist

Every row is either covered by a passing differential test or carries an
explicit reachability proof:

* [x] 1 — `row01_stored_len_nlen_mismatch` (32 randomised LEN/NLEN pairs)
* [x] 2 — `row02_stored_beyond_input` (32 randomised)
* [x] 3 — `row03_literal_overruns_out` (`out_bytes == 0` and 32 off-by-one)
* [x] 4 — `row04_backwards_distance_before_begin` (15 fixed + 32 off-by-one)
* [x] 5 — `row05_match_overruns_out` (48 randomised)
* [x] 6 — `row06_block_type_3` (both `bfinal`, 7 paddings, non-first block)
* [x] 7 — `row07_assert_bits_left_positive` (+ 119 hits in the truncation fuzz)
* [x] 8 — `row08_assert_would_overflow` (hand-derived + 64 randomised)
* [x] 9 — unreachable (proof in the table)
* [x] 10 — unreachable (proof in the table)
* [x] 11 — unreachable (proof in the table)
* [x] 12 — `row12_assert_consume_underflow` (+ 732 hits in the truncation fuzz)
* [x] 13 — unreachable (proof in the table)
* [x] 14 — unreachable (proof in the table; 768-case search finds 0 hits)
* [x] 15 — `row15_assert_code_length_below_16` via the exported table
* [x] 16 — `row16_assert_decode_prefix` (built incomplete tree + 400 fuzz)
* [x] 17 — untestable (OOM only); documented deviation
* [x] 18 — `row18_null_in_pointer`
* [x] 19 — `row19_null_out_pointer`
* [x] 20 — `row20_negative_in_bytes`
* [x] 21 — `row21_negative_out_bytes`
* [x] 22 — impossible; `every_block_type_value` + `out_of_range_int_parameters`
* [x] 23 — `row23_zero_in_zero_out`
* [x] 24 — excluded (unmapped memory), reasoning in the table
* [x] 25 — `fuzz_structured_dynamic_blocks`
* [x] 26 — covered by rows 1-2 and the guarded buffers

Broad safety nets over the same surface: `fuzz_malformed_streams` (4 000 random
byte strings × random alignment × random `out_bytes`: 1 264 aborts, 2 697
rejections, 39 successes), `fuzz_truncated_valid_streams` (853 aborts over every
truncation point of 40 valid streams) and `tests/fuzz.rs::fuzz_campaign`
(100 000 cases over four independent seeds: ~48 % successes, ~42 % rejections,
~10 % aborts, plus non-termination cases).
