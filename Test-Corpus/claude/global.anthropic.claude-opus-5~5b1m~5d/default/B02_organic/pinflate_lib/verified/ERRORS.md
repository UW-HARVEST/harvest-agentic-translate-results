# ERRORS.md — error / rejection surface (Phase A)

Mechanically derived from `c_src/src/lib.c`. Every distinct rejection site in the
translation unit is listed: all 6 `cp_error_reason = …; goto cp_err;` branches
(grep `cp_error_reason =`), all 10 `assert(...)` statements (grep `assert(`), and
the generic FFI boundaries (`pinflate`'s four parameters are completely
unvalidated — there is **no** null check and **no** length check anywhere).

Two things are important for reading the table:

* The reference `.so` is built **with asserts live** (`nm -D` shows
  `U __assert_fail`), so a failing `assert` is an observable result: glibc prints
  `<prog>: <file>:<line>: <func>: Assertion \`<expr>' failed.` to stderr and
  raises `SIGABRT` (exit by signal 6). The Rust translation reproduces this by
  calling `__assert_fail` with the identical expression text, line number and
  function name (`build.rs` reproduces the C `__FILE__`, i.e. the absolute path
  of `c_src/src/lib.c`, so even the stderr text is byte-identical).
* `cp_error_reason` is a **write-only-on-error** global: `pinflate` never clears
  it. The tests therefore write a null pointer into both libraries' exported
  `cp_error_reason` before every call and compare the resulting C strings.

Legend for *status*: `PASS` = a differential test exists and passes;
`PASS (proof)` = the site is provably unreachable through the public API, the
proof is given, and the test asserts the reachable neighbour behaviour instead.

## A. Explicit error returns (`pinflate` returns `0`, sets `cp_error_reason`)

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| E1 | `cp_stored` (l.176) | stored block whose `LEN != (uint16_t)~NLEN` | `pinflate` → `0`; `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."`; `out` untouched | PASS |
| E2 | `cp_stored` (l.185) | stored block where `s->bits_left / 8 > (int)LEN`, i.e. more input bytes remain than the block claims (also fires for `LEN` too small w.r.t. `in_bytes`, and for `LEN==0` with input left) | `pinflate` → `0`; `cp_error_reason` = `"Stored block extends beyond end of input stream."` | PASS |
| E3 | `cp_block` (l.260) | literal symbol (`symbol < 256`) decoded while `s->out + 1 > s->out_end` (out buffer full / `out_bytes == 0` / `out_bytes < 0` / `out == NULL`) | `pinflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a symbol."` | PASS |
| E4 | `cp_block` (l.279) | length/distance pair whose `backwards_distance` exceeds the bytes produced so far (`s->out - backwards_distance < s->begin`), unsigned pointer compare | `pinflate` → `0`; `cp_error_reason` = `"Attempted to write before out buffer (invalid backwards distance)."` | PASS |
| E5 | `cp_block` (l.288) | length/distance pair with a valid distance but `s->out + length > s->out_end` | `pinflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a string."` | PASS |
| E6 | `pinflate` (l.362) | `btype == 3` (the reserved DEFLATE block type) | `pinflate` → `0`; `cp_error_reason` = `"Detected unknown block type within input stream."` | PASS |

Note the check ORDER inside `cp_block`'s match branch, which the translation
preserves: length extra bits are read, then the distance symbol + its extra bits
are read, *then* E4 is tested, *then* E5. A stream that violates both E4 and E5
reports E4.

## B. `assert()` sites (abort: `SIGABRT` + assertion message on stderr)

| # | function : line | assert expression | trigger (the exact invalid input/condition) | expected C result | status |
|---|-----------------|-------------------|---------------------------------------------|-------------------|--------|
| A1 | `cp_ptr` : 95 | `!(s->bits_left & 7)` | **reachable.** `cp_ptr` is only called from `cp_stored`, right after `cp_read_bits(s, s->count & 7)`, which aligns the reader *only while* `count ≡ -(bits consumed) (mod 8)`. `cp_peak_bits`'s final-word fold (`count += bits_left`) breaks that invariant whenever it happens at a bit position that is not a multiple of 8: afterwards `count ≡ -2·C₀ - C`, the pad read leaves `consumed ≡ -C₀`, and `bits_left ≡ C₀ (mod 8)`. Concrete trigger (`c_a1_cp_ptr`): a 6-byte, 4-byte-aligned input = `bfinal=0, btype=1`, two 8-bit literals, end-of-block, then `bfinal=1, btype=0` and `LEN=0xFFFF`; the fold happens after 19 bits, and `cp_ptr` sees `bits_left == -13`, `bits_left & 7 == 3`. | `SIGABRT`, message ``Assertion `!(s->bits_left & 7)' failed.`` | PASS |
| A2 | `cp_peak_bits` : 104 | `s->word_index <= s->word_count` | — | **unreachable**: guarded by `if (s->word_index < s->word_count)` one line above, so after the post-increment `word_index <= word_count` holds. | `SIGABRT` if it could fire | PASS (proof) — 74 000+ differential cases, including every 1-byte and every 2-byte input at all four alignments, never reach it in C or in Rust, and both libraries contain the identical check |
| A3 | `cp_consume_bits` : 115 | `s->count >= num_bits_to_read` | truncated stream: `cp_decode` calls `cp_consume_bits(s, key & 0xF)` with **no** bits_left/overflow pre-check, so a stream that ends in the middle of a Huffman code aborts here. Also reachable from `cp_read_bits` when `cp_would_overflow` passes but `cp_peak_bits` has nothing left to fold (`word_index == word_count && !final_word_available` while `bits_left > 0`, which happens whenever `in` is unaligned so that `word_count*4 + first_bytes + last_bytes < in_bytes`). | `SIGABRT`, message `Assertion \`s->count >= num_bits_to_read' failed.` | PASS |
| A4 | `cp_read_bits` : 123 | `num_bits_to_read <= 32` | only reachable through the mutable exported tables: set `cp_len_extra_bits[k] = 33` (or `cp_dist_extra_bits[k] = 33`) and decode a length/distance symbol `k`. All in-library call sites pass `count&7 ≤ 7`, or the constants 1/2/3/4/5/7/16. | `SIGABRT`, message `Assertion \`num_bits_to_read <= 32' failed.` | PASS |
| A5 | `cp_read_bits` : 124 | `num_bits_to_read >= 0` | — | **unreachable**: the argument is either `s->count & 7` (and `count >= 0` is guaranteed by A3 + `cp_peak_bits` only adding), or a `uint8_t` from one of the exported tables promoted to `int` (0…255), or a literal constant. No expression can be negative. | `SIGABRT` if it could fire | PASS (proof) — covered by the A4 test, which pushes the same argument up to 255; `> 32` fires first, at line 123, in both libraries |
| A6 | `cp_read_bits` : 125 | `s->bits_left > 0` | input exhausted: `in_bytes == 0`; `in == NULL, in_bytes == 0`; any stream whose blocks consume every bit and then keep reading (e.g. a 1-byte stored-block header); `in_bytes < 0` (`bits_left = in_bytes*8 < 0`); `in_bytes` large enough that `in_bytes*8` wraps to `0` (e.g. `0x20000000`, `INT_MIN`) | `SIGABRT`, message `Assertion \`s->bits_left > 0' failed.` | PASS |
| A7 | `cp_read_bits` : 126 | `s->count <= 64` | — | **unreachable**: `count` only grows in `cp_peak_bits`, by 32 when a word is folded (from `count < 16`, so `count ≤ 47`) or by `bits_left` when the final partial word is folded; at that moment `count ≤ 15` and `bits_left ≤ 8*(first_bytes+last_bytes) + 15 ≤ 39`, giving `count ≤ 54`. | `SIGABRT` if it could fire | PASS (proof) — the sweeps (every 1-byte and every 2-byte input, plus 2 880 random 2…96-byte inputs at all alignments) never reach it in C or in Rust |
| A8 | `cp_read_bits` : 127 | `!cp_would_overflow(s, num_bits_to_read)` i.e. `bits_left + count - n >= 0` | reachable without mutating anything: a 6-byte, 4-byte-aligned input with `btype = 2` and `HCLEN ≥ 11` runs out of bits inside the 3-bit code-length loop (`bits_left == count == 1`, `n == 3`). Generally: any read of `n` bits once `2*bits_left + (count-bits_left) < n`. | `SIGABRT`, message ``Assertion `!cp_would_overflow(s, num_bits_to_read)' failed.`` | PASS |
| A9 | `cp_build` : 154 | `len < 16` | only reachable through the mutable exported table: set `cp_fixed_table[k] >= 16` and decode a `btype == 1` block (`cp_fixed` → `cp_build`). Inside the library `lens[]` is either `cp_fixed_table` (5…9) or `cp_dynamic`'s `lens[]`, whose values come from the 19-symbol code-length tree and are therefore ≤ 15. | `SIGABRT`, message `Assertion \`len < 16' failed.` | PASS |
| A10 | `cp_decode` : 217 | `(search >> len) == (key >> len)` | the decoded bits do not match the entry the binary search landed on. Canonical trigger: a `btype == 2` block whose code-length code lengths are **all zero** ⇒ `s->nlen == 0` ⇒ `hi == 0` ⇒ the loop never runs, `lo == 0`, and the code reads `tree[-1]` (aliasing `s->dst[31]`, which is `0` from `calloc`) ⇒ `len = 32`, `key >> 32 == 0` but `search >> 32 == search != 0` (x86 shift-count masking). Same for a fixed/dynamic block whose literal tree is empty (`nlit == 0`, `tree[-1]` aliases `s->lookup[510..511]`). | `SIGABRT`, message ``Assertion `(search >> len) == (key >> len)' failed.`` | PASS |

## C. Generic FFI boundaries (no check exists in the C — the behaviour *is* the contract)

| # | entry point | trigger | expected C result | status |
|---|-------------|---------|-------------------|--------|
| N1 | `pinflate` | `in == NULL`, `in_bytes == 0` | `first_bytes = 0`, nothing dereferenced, first `cp_read_bits` aborts on A6 (`SIGABRT`) | PASS |
| N2 | `pinflate` | `out == NULL`, `out_bytes == 0`, stream = empty fixed block (EOB only) | returns `1`, nothing written | PASS |
| N3 | `pinflate` | `out == NULL`, `out_bytes == 0`, stream emits one literal | returns `0` with E3 (`out+1 <= out_end` fails on the null pointer; no dereference) | PASS |
| N4 | `pinflate` | `out_bytes < 0` (`out_end < out`) | E3 for the first literal / E5 for the first match; `cp_stored` ignores `out_end` completely and **still memcpy's `LEN` bytes** (translated verbatim) | PASS |
| N5 | `pinflate` | `in_bytes < 0` (`-1`, `-3`, `-4`, `-8`, `INT_MIN`) × all 4 alignments | `bits_left <= 0`; `last_bytes = (in_bytes - first_bytes) & 3` may be non-zero, in which case the final-word loop reads `in[in_bytes-last_bytes .. in_bytes-1]`, i.e. **before** the buffer, and only then does A6 abort. Both libraries read the same out-of-range bytes because the test hands them one shared, 8 KiB-padded input buffer. For `INT_MIN` the same read is a wild address ⇒ `SIGSEGV` instead. | `SIGABRT` (A6), or `SIGSEGV` when `last_bytes != 0` and `|in_bytes|` is large | PASS |
| N6 | `pinflate` | `in_bytes` one step past the representable bit count: `0x20000000` / `0x40000000` (`*8` wraps to `0`), `INT_MAX`, `INT_MAX-1` | `0x20000000` at alignment 0: `last_bytes == 0`, nothing is read, A6 aborts. At the other alignments and for `INT_MAX`: `last_bytes != 0` ⇒ wild read at `in + in_bytes - last_bytes` ⇒ `SIGSEGV` in both | PASS |
| N7 | `pinflate` | `out_bytes` huge (`INT_MAX`) with a short valid stream | `out_end = out + INT_MAX` (never dereferenced past the real data) ⇒ normal success | PASS |
| N8 | `pinflate` | `btype` "out of range": the 2-bit field can only hold 0…3, and 3 is the reserved value ⇒ E6. `pinflate`'s `switch` also has an empty `default`, so a value outside 0…3 would fall through and re-loop; unreachable because the value comes from `cp_read_bits(s, 2)`. The equivalent "invalid enum across FFI" for this API is an out-of-range `in_bytes`/`out_bytes` (N4–N7) and an out-of-range value written into one of the 7 exported tables (A4/A9, plus `cp_len_base`/`cp_dist_base`/`cp_permutation_order`/`cp_error_reason` writes). | see E6 / A4 / A9 | PASS |

## D. Rejections that do **not** exist in the C (documented so the Rust does not add them)

* No `NULL` check on `in`, `out`, or the `calloc` result (`s->bits = 0` would
  fault on OOM).
* No check that a stored block fits in `out` — `cp_stored` `memcpy`s `LEN` bytes
  with no `out_end` comparison. The Rust must overflow the caller's buffer in
  exactly the same way.
* No check that `nlit ≤ 288` / `ndst ≤ 32` beyond the 5-bit field widths, no
  check that `n` stays inside `cp_dynamic`'s `lens[288+32]` (the repeat codes
  16/17/18 can walk `n` up to 457 and overwrite the neighbouring stack slots —
  reproduced by the emulated frame in `cp_dynamic`), and no Kraft/completeness
  check on any Huffman table.
* No check that a length/distance symbol is within the RFC-1951 range: symbols
  286/287 index `cp_len_base[29..30] == 0` (length 0) and distance symbols 30/31
  index `cp_dist_base[30..31] == 0` (distance 0, `src == dst`, a no-op copy).
  Both are valid-path rows in `CONFIGS.md`, not errors.

### Known limit of the emulation

Nine bytes of `cp_dynamic`'s frame (`%rbp-0x2d … %rbp-0x24`, the padding between
`lenlens[19]` and `sym`, i.e. `lens[339..348]`) are **uninitialised** in the C and
zero in the emulated frame. They can only be *read* if `cp_build` is called with
`lens + nlit` where `nlit + ndst > 339`, which in turn requires the repeat codes
to have already walked `n` past `lens[348]` and clobbered `nlit`/`ndst` — and the
same clobbering also overwrites the low byte of `n` itself (`lens[376]`), which is
what makes the loop non-terminating (section F). No input in the 74 000-case
sweep — including every 1-byte and every 2-byte input — reaches a *terminating*
execution that reads those nine bytes; both libraries either agree exactly or
hang together. This is the one place where the C's behaviour is genuinely
unspecified (it depends on stack residue), so it is recorded here rather than
papered over.

## E. Where each row is verified

| rows | test id in `tests/differential.rs` |
|------|------------------------------------|
| E1 | `c_e1_len_nlen` (24 cases: 6 LEN/NLEN combinations × 4 input alignments) |
| E2 | `c_e2_stored_beyond` (24 cases) |
| E3, N3 | `c_e3_out_symbol` (20 cases: 4 literal counts × `out_bytes ∈ {0, 1, n-1, -1, -1000}`, plus `out == NULL`) |
| E4 | `c_e4_back_dist` (6 cases, distances 1…32768 against 0…100 produced bytes) |
| E5 | `c_e5_out_string` (8 cases, lengths 3…258, one byte short and far short) |
| E6 | `c_e6_unknown_btype` (21 cases: 4 alignments × 5 input lengths, plus `btype=3` as a second block) |
| A1 | `c_a1_cp_ptr` (3 cases) |
| A3 | `c_a3_consume_bits` (12 cases: 4 alignments × 3 literal counts) |
| A4, A5 | `c_a4_read_bits_width` (8 cases: `cp_len_extra_bits[0]` / `cp_dist_extra_bits[0]` ∈ {33, 64, 127, 255}) |
| A6, N1, N5, N6 | `c_a6_bits_left` (37 cases) and `c_boundaries` |
| A8 | `c_a8_would_overflow` (9 cases: HCLEN ∈ {11, 12, 19} × 3 HLIT/HDIST combinations) |
| A9 | `c_a9_build_len` (28 cases: `cp_fixed_table[k] ∈ {16,17,56,255}` for k ∈ {0,1,143,287,288,300,319}) |
| A10 | `c_a10_decode_key` (10 cases: `nlen == 0` and `nlit == 0` variants) |
| A2, A7 (unreachable) + everything at once | `c_sweep_tiny` (all 256 1-byte inputs × 4 alignments), `c_sweep_two` (all 65 536 2-byte inputs), `c_sweep_random` (2 880 random 2…96-byte inputs × random alignments and `out_bytes`) |
| N2, N4, N7 | `c_boundaries` (45 cases) |

The sweeps compare, for every input: the return value, the `cp_error_reason`
string, the whole padded output allocation, the exported tables, and — when the
library dies — the terminating signal together with the byte-exact stderr text.
Of the 65 536 exhaustive 2-byte inputs, all 65 536 abort in both libraries with a
byte-identical assertion message.

## F. Non-termination

`cp_dynamic` has no bound on `n`, and its repeat codes (16/17/18) can walk `n`
past `lens[288+32]` into the neighbouring stack slots — including `n` itself
(`lens[376]` aliases the low byte of `n` in gcc's -O0 frame), which rewinds `n`
to 256 and makes the loop run forever. `c_sweep_random` finds two such inputs
(cases 2287 and 2698); **both libraries hang on exactly those inputs and on no
others**, still running after 45 s. The translation reproduces this by emulating
the gcc frame layout (see the `Frame` type in `src/lib.rs`), which was verified
against `objdump -d` of `c_src/build/CMakeFiles/*/src/lib.c.o`:

```
-0x188  s (spilled parameter)   -0x20  nlen     -0x10  i (case 17)
-0x180  lens[288+32]            -0x1c  ndst     -0x0c  i (case 16)
-0x40   lenlens[19]             -0x18  nlit     -0x08  n
-0x24   sym                     -0x14  i (18)   -0x04  i (permutation loop)
```
