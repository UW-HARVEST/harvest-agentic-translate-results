# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`: every `assert()`, every
`cp_error_reason = ...; goto cp_err`, every `return 0` / `return 0`-equivalent,
and every unchecked pointer dereference reachable from an exported entry point.

Reminder (see `SYMBOLS.md`): the C `.so` is built **without `-DNDEBUG`**, so
`assert()` is live and a failing assert produces **SIGABRT**. "abort" below
means the process dies with `SIGABRT` (6).

`R:` = observed result of the Rust `.so`. `[x]` = differential test written and
passing against both libraries.

## A. `assert()` sites

Verified per-row: the harness captures the forked child's stderr, so the exact
assertion expression glibc printed is compared against the expected row (see
`Outcome::assert_expr` in `tests/harness/mod.rs`). A row is only checked off when
the C died on *that* assertion and the Rust died with the same signal.

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| E1 | `cp_ptr` (via `cp_stored`) | `s->bits_left & 7 != 0` when a stored block calls `cp_ptr` | abort: `!(s->bits_left & 7)` | [x] **proved unreachable** |
| E2 | `cp_peak_bits` | `s->word_index > s->word_count` after increment | **unreachable** — the assert sits inside `if (word_index < word_count)`, so post-increment `word_index <= word_count` always holds | [x] |
| E3 | `cp_consume_bits` | `s->count < num_bits_to_read`, reachable from `cp_decode` (which has no `cp_read_bits` guard) and from `cp_stored`'s LEN/NLEN reads. Concrete case: `in_bytes = 4` at `in_align = 3` ⇒ `first_bytes = 1`, `word_count = 0`, `last_bytes = 3`, so the second 16-bit read finds only 8 buffered bits | abort: `s->count >= num_bits_to_read` | [x] |
| E4 | `cp_read_bits` | `num_bits_to_read > 32` — no call site can produce it (literals ≤ 16, `count & 7` ≤ 7, `key & 0xF` ≤ 15) | **unreachable** | [x] |
| E5 | `cp_read_bits` | `num_bits_to_read < 0` — `count & 7` is non-negative even for negative `count`; all other arguments are literals | **unreachable** | [x] |
| E6 | `cp_read_bits` | `s->bits_left <= 0`. Trivially reachable: `cp_inflate(in, 0, out, n)`, and any negative `in_bytes` | abort: `s->bits_left > 0` | [x] |
| E7 | `cp_read_bits` | `s->count > 64`. `cp_peak_bits` only loads when `count < num_bits_to_read <= 32`, and the tail branch adds `bits_left` where `bits_left - count == last_bytes*8 - 32 <= 24`, so `count <= 2*15 + 8 = 38` | **unreachable** | [x] |
| E8 | `cp_read_bits` | `cp_would_overflow(s, n)`, i.e. `bits_left + count - n < 0`. Reachable by truncating any well-formed stream | abort: `!cp_would_overflow(s, num_bits_to_read)` | [x] |
| E9 | `cp_build` | any `lens[i] >= 16` (and `!= 0`). Reachable **only** by mutating the exported `cp_fixed_table`; from `cp_dynamic` the code-length symbols are `0..=15` by construction | abort: `len < 16` | [x] |
| E10 | `cp_decode` | the peeked bits are not a code in the tree. Requires an **incomplete** Huffman code — the RFC-1951 fixed code is Kraft-complete, so no bit string is undecodable against it (pinned as a negative test). Reachable via a dynamic block with an incomplete literal code, and via `hi == 0` (all-zero `HCLEN` lengths) where `tree[-1]` is read | abort: `(search >> len) == (key >> len)` | [x] |

### Why E1 is unreachable

`cp_stored` aligns with `cp_read_bits(s, s->count & 7)`, which is only correct if
`count ≡ bits_left (mod 8)`. That invariant holds initially
(`count = first_bytes*8`, `bits_left = in_bytes*8`) and is preserved by
`consume` (both decrease by `n`) and by the 32-bit word load. The tail branch
(`count += s->bits_left`) *can* break it, but only when `count > 0`, and during a
stored block's prologue the tail branch can only fire at the very first
`cp_read_bits(s, 1)` where `count == 0` — which sets `count = bits_left`, keeping
them equal. After the alignment read `bits_left ≡ 0 (mod 8)`, and the two 16-bit
LEN/NLEN reads preserve that. `tests/t04_errors.rs::e1_cp_ptr_bits_left_alignment`
searches Huffman-then-stored streams across all four input alignments and
confirms the assertion is never reported.

`E10` is the dominant validity gate: it makes most malformed Huffman input abort
rather than fall through into the out-of-range `cp_len_base[]` paths. Note the
shift `32 - (key & 0xF)` is `32` when `key & 0xF == 0`, which is C UB; at `-O0`
on x86-64 `shr %cl` masks the count to 5 bits, i.e. a shift by 0. The Rust
reproduces exactly that with `wrapping_shr`.

## B. `cp_error_reason` + `return 0` sites (graceful rejection)

All of these make `cp_inflate` return `0` and leave `cp_error_reason` pointing
at the exact C string literal. Tests compare the **string contents** as well as
the return value.

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| E11 | `cp_stored` | stored block (btype=0) where `LEN != (uint16_t)~NLEN` | `cp_inflate` → `0`; `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."` | [x] |
| E12 | `cp_stored` | stored block where `s->bits_left / 8 > (int)LEN` (declared `LEN` shorter than the bits remaining in the input) | `cp_inflate` → `0`; `cp_error_reason` = `"Stored block extends beyond end of input stream."` | [x] |
| E13 | `cp_block` | literal symbol (`< 256`) decoded when `s->out + 1 > s->out_end` (output buffer full) | `cp_inflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a symbol."` | [x] |
| E14 | `cp_block` | length/distance pair whose `backwards_distance` reaches before `s->begin` (`out - dist < begin`) | `cp_inflate` → `0`; `cp_error_reason` = `"Attempted to write before out buffer (invalid backwards distance)."` | [x] |
| E15 | `cp_block` | length/distance pair where `s->out + length > s->out_end` | `cp_inflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a string."` | [x] |
| E16 | `cp_inflate` | block header with `btype == 3` | `cp_inflate` → `0`; `cp_error_reason` = `"Detected unknown block type within input stream."` | [x] |

Note the check ordering in `cp_block`: the **distance** check (E14) is evaluated
*before* the **length** check (E15), and both happen after both `cp_read_bits`
calls, so an out-of-range `distance_symbol` can trip an assert first.

Note E12 is `>=`-flavoured in a surprising direction: the C rejects when
*more* input remains than `LEN`, so a stored block must be the **last** thing
in the input stream.

## C. `unfilter` rejection sites

| # | function | trigger (exact invalid input/condition) | expected C result | [ ] |
|---|----------|------------------------------------------|-------------------|-----|
| E17 | `unfilter` | `h > 0` and row-0 filter byte `raw[0] > 4` (i.e. 5..255) | return `0`, after having advanced past the filter byte only (no data written) | [x] |
| E18 | `unfilter` | `h > 1` and filter byte of some row `y >= 1` is `> 4` | return `0`; rows `0..y` are already filtered in place (partial mutation is observable) | [x] |

`unfilter` has **no** error path other than E17/E18: `w`, `h`, `bpp` are never
validated, and there is no null check.

## D. Generic FFI boundary cases (not `assert`/`return 0` sites, but real inputs)

| # | function | trigger | expected C result | [ ] |
|---|----------|---------|-------------------|-----|
| E19 | `unfilter` | `raw == NULL`, `h > 0` | dereferences NULL → SIGSEGV | [x] |
| E20 | `unfilter` | `raw == NULL`, `h <= 0` | no dereference; `prev = raw; raw += len;` only → returns `1` | [x] |
| E21 | `unfilter` | `h == 0` (with any `w`, `bpp`, valid `raw`) | returns `1`, buffer untouched | [x] |
| E22 | `unfilter` | `h < 0` (negative count) | `if (h > 0)` false, loop `y=1; y<h` false → returns `1`, untouched | [x] |
| E23 | `unfilter` | `w == 0` → `len == 0`; `h` rows | only filter bytes consumed, no data loop runs; returns `1` | [x] |
| E24 | `unfilter` | `bpp == 0` → `len == 0` regardless of `w`; same as E23 | returns `1`, untouched | [x] |
| E25 | `unfilter` | `bpp > len` (e.g. `w=2,bpp=3` → `len=6`, `bpp=3`; and `w=1,bpp=4` → `len=4`) plus `bpp` deliberately `> len` via `w=1,bpp=1` vs `w=1,bpp=5`: row-0 loops don't run, but row `y>=1` `for (x=0;x<bpp;x++)` writes `bpp` bytes past `len` | identical over-write in both | [x] |
| E26 | `unfilter` | negative `bpp` (e.g. `bpp = -1`) → `len = -w`; row-0 loop `x=bpp; x<len` runs from negative index | identical negative-index reads/writes | [x] |
| E27 | `unfilter` | filter byte exactly at boundary `4` (valid) and `5` (first invalid) | `4` → paeth path returns `1`; `5` → returns `0` | [x] |
| E28 | `cp_inflate` | `in_bytes == 0` | `bits_left = 0` → abort (E6) | [x] |
| E29 | `cp_inflate` | `in_bytes < 0` | `bits_left = in_bytes*8 < 0` → abort (E6) | [x] |
| E30 | `cp_inflate` | `out_bytes == 0` with a stream that emits a literal | E13 (`return 0`) | [x] |
| E31 | `cp_inflate` | `out_bytes < 0` → `out_end < out`, so `out+1 <= out_end` fails immediately | E13 (`return 0`) | [x] |
| E32 | `cp_inflate` | `in == NULL`, `in_bytes > 0` | `first_bytes = 0`; reads `((uint8_t*)NULL)[...]`/`words[0]` → SIGSEGV | [x] |
| E33 | `cp_inflate` | `out == NULL`, stream emits ≥ 1 literal, `out_bytes > 0` | `out_end = 0+out_bytes`, `out+1 <= out_end` passes → NULL write → SIGSEGV | [x] |
| E34 | `cp_inflate` | `out == NULL`, `out_bytes == 0`, literal | E13 (`return 0`), no dereference | [x] |
| E35 | `cp_inflate` | btype value out of documented range — impossible from the bitstream (2 bits ⇒ 0..3), so the "out-of-range enum" surface is exactly E16 (`btype == 3`) | `cp_inflate` → `0`, E16 message | [x] |
| E36 | `unfilter` | filter byte out-of-range enum values swept over **all** 251 invalid values `5..=255`, at row 0 and at row 1 | return `0` for every one | [x] |
| E37 | `cp_inflate` | empty stored block that is the whole input (`LEN == 0`, correct `NLEN`, `bfinal=1`) — valid, checks E12 boundary `bits_left/8 == 0 <= 0` | returns `1`, nothing written | [x] |
| E38 | `cp_inflate` | stored block whose `LEN` is one byte larger than the data actually present | `bits_left/8 <= LEN` passes, `memcpy` reads past `in` (over-read) | [x] |
| E39 | `cp_build` (via mutated `cp_fixed_table`) | caller writes `15` into `cp_fixed_table` (valid, no abort) vs `16` (abort, E9) — boundary one past the documented range | `15` → no abort; `16` → abort | [x] |

## E. Dead code (no exported caller) — documented, not testable via the `.so`

`cp_chunk` and `cp_find` (`return 0`/NULL on chunk-not-found) and
`cp_make_pixel`/`cp_make_pixel_a` are `static` with no callers in `lib.c`; they
are not reachable through any exported symbol. They are translated in
`src/lib.rs` for fidelity but cannot be differentially tested through the `.so`
boundary.

## F. `cp_dynamic`'s `lens[288 + 32]` overflow — MODELLED, not left to chance

`cp_dynamic` declares `uint8_t lens[288 + 32];` **uninitialised** and runs

```c
for (int n = 0; n < nlit + ndst;) { ... case 18: for (int i = 11 + read(7); i; --i, ++n) lens[n] = 0; ... }
```

`nlit + ndst <= 320`, but one `case 18` adds up to 138, so `n` can go from 319 to
457 — up to 137 bytes past the array. At `-O0` every local lives in memory and is
re-read on each use, so those writes really do rewrite `cp_dynamic`'s own
variables. The frame layout, read off
`objdump -d c_src/build/CMakeFiles/*.dir/src/lib.c.o`:

| `%rbp` offset | variable | `lens[]` index |
|---------------|----------|----------------|
| `-0x188` | `s` (parameter spill) | `-8 .. 0` |
| `-0x180` | `lens[320]` | `0 .. 320` |
| `-0x40`  | `lenlens[19]` | `320 .. 339` |
| `-0x2d`  | padding | `339 .. 348` |
| `-0x24`  | `sym` | `348 .. 352` |
| `-0x20`  | `nlen` | `352 .. 356` |
| `-0x1c`  | `ndst` | `356 .. 360` |
| `-0x18`  | `nlit` | `360 .. 364` |
| `-0x14`  | `case 18` counter `i` | `364 .. 368` |
| `-0x10`  | `case 17` counter `i` | `368 .. 372` |
| `-0x0c`  | `case 16` counter `i` | `372 .. 376` |
| `-0x08`  | `n` | `376 .. 380` |
| `-0x04`  | permutation-loop `i` | `380 .. 384` |
| `+0x00`  | saved `%rbp` | `384 .. 392` |
| `+0x08`  | return address | `392 .. 400` |

Two consequences that a naive "give the array some slack" translation gets wrong:

* **`lens[-1]`** — read by `case 16` when `n == 0` — is byte 7 of the spilled `s`
  pointer, i.e. the most-significant byte of a heap address. On x86-64 user space
  that is reliably `0x00`, so the read is *deterministic*, not indeterminate.
* **`n` can never escape.** Writing a code-length byte `v` to `lens[376]` sets
  `n`'s low byte, making `n = (376 & ~0xff) | v = 0x100 | v`, so `n` folds back to
  ~256 and climbs again. The saved `%rbp` and return address at `lens[384..400)`
  are therefore never reached, and this is what makes some malformed dynamic
  blocks spin for a long time in *both* libraries.
* **`nlit` can become huge.** Writing to `lens[361..364)` sets `nlit`'s upper
  bytes, so `nlit` can reach ~10^6. The final `cp_build(s, s->lit, lens, nlit)`
  then walks megabytes off the end of both the stack array and `s->lit`, and both
  libraries take SIGSEGV.

`src/lib.rs` therefore models the frame byte-for-byte (`cp_dyn_frame`, with
compile-time `offset_of!` assertions) and routes every `lens[]` access through a
bounds check that faults (`cp_lost_control`) where the C would be scribbling
arbitrarily far up its stack. `tests/t06_lens_overflow.rs` drives the overflow
deliberately (rows C38a–C38e in `CONFIGS.md`) instead of relying on fuzzing to
stumble into it.

## G. Known-divergent-by-nature: nothing

No remaining behaviour is left unmatched. The two constructs that looked
unmatchable at first (`lens[-1]` and the `lens[]` overflow) turned out to be
deterministic at `-O0` and are modelled; see section F.
