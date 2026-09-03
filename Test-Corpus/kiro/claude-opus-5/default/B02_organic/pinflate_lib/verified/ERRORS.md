# ERRORS.md — error-surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/lib.c` with

```sh
grep -n 'assert\|cp_error_reason\|goto cp_err\|return 0\|return 1' c_src/src/lib.c
```

Every `cp_error_reason = ...` site, every `assert(...)`, every explicit range /
complement / bounds check, and every pointer/size boundary the public entry
point can be handed. There are **exactly 6** soft-error sites and **exactly 10**
`assert()` sites in the C source; all 16 appear below, plus the generic FFI
boundaries (`G*` rows).

`cp_error_reason` is a public symbol, so for soft errors the *expected C result*
includes the exact string the C stores there. Divergence in the string is a
divergence.

`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`/`-DNDEBUG`, so the `assert()`s are
compiled in: rows `A*` are real, reachable, observable behaviour of the
reference `.so` (`SIGABRT`), not dead code.

## Soft errors (`pinflate` returns `0`, `cp_error_reason` set)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| E1 | `cp_stored` (lib.c:176-182) | `LEN != (uint16_t)(~NLEN)`: stored block whose LEN/NLEN header fields are not one's-complements | `pinflate` → `0`; `cp_error_reason` = `"Failed to find LEN and NLEN as complements within stored (uncompressed) stream."`; `out` untouched |
| E2 | `cp_stored` (lib.c:184-190) | `!(s->bits_left / 8 <= (int)LEN)` — i.e. *more* whole bytes remain in the input than `LEN` announces (note: the C check is this direction; a well-formed but *trailing-data-bearing* stored block trips it) | `pinflate` → `0`; `cp_error_reason` = `"Stored block extends beyond end of input stream."`; `out` untouched |
| E3 | `cp_block` (lib.c:257-266) | literal symbol (`sym < 256`) decoded while `!(s->out + 1 <= s->out_end)` — output buffer full / `out_bytes` too small / `out_bytes <= 0` | `pinflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a symbol."`; bytes already emitted stay in `out` |
| E4 | `cp_block` (lib.c:277-285) | match copy with `!(s->out - backwards_distance >= s->begin)` — back-reference points before the start of the output buffer | `pinflate` → `0`; `cp_error_reason` = `"Attempted to write before out buffer (invalid backwards distance)."` |
| E5 | `cp_block` (lib.c:286-294) | match copy with `!(s->out + length <= s->out_end)` — copy would run past `out + out_bytes` | `pinflate` → `0`; `cp_error_reason` = `"Attempted to overwrite out buffer while outputting a string."` |
| E6 | `pinflate` (lib.c:359-367) | `btype == 3` (the reserved DEFLATE block type) in the 3-bit block header | `pinflate` → `0`; `cp_error_reason` = `"Detected unknown block type within input stream."` |

Note there is **no** error path for `bfinal`/`btype` other than E6, and no
`return NULL` / error-enum anywhere: `pinflate` returns only `0` or `1`.

## Hard errors — live `assert()`s (`SIGABRT`)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| A1 | `cp_ptr` (lib.c:95) | `assert(!(s->bits_left & 7))` — a stored block reached with the remaining input bit count not byte aligned | `SIGABRT` |
| A2 | `cp_peak_bits` (lib.c:104) | `assert(s->word_index <= s->word_count)` — unreachable (guarded by the enclosing `if`), asserted for fidelity | never fires; must also never fire in Rust |
| A3 | `cp_consume_bits` (lib.c:115) | `assert(s->count >= num_bits_to_read)` — consuming more bits than are buffered; reached via `cp_decode`'s `cp_consume_bits(s, key & 0xF)` on a truncated stream | `SIGABRT` |
| A4 | `cp_read_bits` (lib.c:123) | `assert(num_bits_to_read <= 32)` — only reachable through a corrupted `cp_len_extra_bits`/`cp_dist_extra_bits` global (both are writable exports) | `SIGABRT` |
| A5 | `cp_read_bits` (lib.c:124) | `assert(num_bits_to_read >= 0)` — likewise via a corrupted extra-bits global (`uint8_t` → `int` is never negative, so only a negative `s->count & 7` could reach it) | `SIGABRT` |
| A6 | `cp_read_bits` (lib.c:125) | `assert(s->bits_left > 0)` — **`in_bytes == 0`** (empty input) or `in_bytes < 0`, or input exhausted mid-stream | `SIGABRT` |
| A7 | `cp_read_bits` (lib.c:126) | `assert(s->count <= 64)` — bit accumulator over-full | `SIGABRT` |
| A8 | `cp_read_bits` (lib.c:127) | `assert(!cp_would_overflow(s, n))`, i.e. `(bits_left + count) - n < 0` — **truncated stream**: asking for more bits than the input still holds. The most frequently reached assert for random/garbage input | `SIGABRT` |
| A9 | `cp_build` (lib.c:154) | `assert(len < 16)` — a code length ≥ 16 in the literal/distance length vector (only producible by a corrupt dynamic-header decode, or a corrupted `cp_fixed_table` export) | `SIGABRT` (preceded in C by an out-of-bounds `counts[lens[n]]++` stack write, unobservable because the abort follows) |
| A10 | `cp_decode` (lib.c:217) | `assert((search >> (32 - (key & 0xF))) == (key >> (32 - (key & 0xF))))` — the peeked bits do not match the Huffman entry found by the binary search. Fires when the code is incomplete/over-subscribed, when `nlit`/`ndst`/`nlen` is `0` (so `lo == 0` and `tree[-1]` — a *neighbouring struct field* — is read), or when the matched slot is a zero (never-written) tree entry, in which case `key == 0`, `32 - 0 == 32` and the `>> 32` of a `uint32_t` is x86 `shr`-mod-32 (no shift), so `search != 0` guarantees the abort | `SIGABRT` |

## Generic FFI boundaries (covered even though not in the C's own check list)

| # | entry point | trigger | expected C result |
|---|-------------|---------|-------------------|
| G1 | `pinflate` | `in == NULL`, `in_bytes == 0` | `first_bytes = 0`, `bits_left = 0` → A6 → `SIGABRT` |
| G2 | `pinflate` | `in == NULL`, `in_bytes > 0` | dereferences `NULL` in `cp_peak_bits` → `SIGSEGV` |
| G3 | `pinflate` | `in` valid, `in_bytes == 0` | A6 → `SIGABRT` |
| G4 | `pinflate` | `in` valid, `in_bytes < 0` (e.g. `-1`, `INT_MIN`) | `bits_left < 0` → A6 → `SIGABRT` |
| G5 | `pinflate` | `out == NULL`, `out_bytes == 0`, valid stream with ≥1 literal | `out_end == out` → E3 (`0` + symbol message), **no** fault |
| G6 | `pinflate` | `out == NULL`, `out_bytes > 0`, valid stream with ≥1 literal | writes through `NULL` → `SIGSEGV` |
| G7 | `pinflate` | `out_bytes < 0` (e.g. `-1`, `INT_MIN`) | `out_end < out` → E3 |
| G8 | `pinflate` | `out_bytes` exactly one byte short of the decoded size | E3 (literal-terminated) or E5 (match-terminated) |
| G9 | `pinflate` | `in_bytes` one byte short of a valid stream (truncation by 1) | A8 or A3 → `SIGABRT` (or a soft error if truncation lands on a bounds check first) |
| G10 | `pinflate` | `in_bytes` *larger* than the real stream (trailing garbage) | trailing bytes are parsed as another block once `bfinal` is 0; with `bfinal` set they are ignored → `1` |
| G11 | `pinflate` | oversized `in_bytes` (`INT_MAX`) with a small buffer | `word_count` huge → reads past the buffer → `SIGSEGV` (or garbage) |
| G12 | "out-of-range enum" analogue: `btype` | `btype` is read with `cp_read_bits(s, 2)`, so `0..3` is total and `3` is the no-valid-variant value → row E6. There is no other C `enum`/mode parameter in the public API, so E6 **is** the out-of-range-enum row. | see E6 |
| G13 | `pinflate` | `in` at each of the 4 possible 4-byte alignments (`first_bytes ∈ {0,1,2,3}`) with `in_bytes` short enough that `word_count < 0` | `word_count` negative → `final_word` path only → A8 → `SIGABRT` |
| G14 | writable exports | consumer stores an out-of-range value into `cp_len_extra_bits` / `cp_dist_extra_bits` (`> 32`) before calling | A4 → `SIGABRT` |
| G15 | writable exports | consumer zeroes `cp_fixed_table` before calling with a `btype == 1` block | all lengths 0 → `nlit == 0` → `cp_decode` with `hi == 0` → `tree[-1]` → A10 → `SIGABRT` |

## Checklist (Phase C) — all rows have a passing differential test

Tests assert the *specific* rejection, not merely "both failed": soft errors
compare the returned `int` **and** the `cp_error_reason` string; hard errors
compare the signal **and** the assertion site
(`lib.c:<line>: <fn>: Assertion `<expr>' failed.`, which the Rust reproduces
verbatim via `cp_assert_fail`).

| row | test | status |
|-----|------|--------|
| E1  | `phase_c_errors.rs` "E1 stored: LEN != ~NLEN"                   | [x] |
| E2  | `phase_c_errors.rs` "E2 stored: bits_left/8 > LEN"              | [x] |
| E3  | `phase_c_errors.rs` "E3 literal with a full output buffer"      | [x] |
| E4  | `phase_c_errors.rs` "E4 match reaching before the start"        | [x] |
| E5  | `phase_c_errors.rs` "E5 match copy overrunning the buffer"      | [x] |
| E6  | `phase_c_errors.rs` "E6 reserved block type 3"                  | [x] |
| A1  | `phase_c_errors.rs` "A1 ... stored block reached unaligned"     | [x] |
| A2  | never fires in ~35 000 differential calls (guarded by the enclosing `if`) — **proved unreachable**, asserted absent | [x] |
| A3  | `phase_c_errors.rs` "A3 ... buffered bits exhausted"            | [x] |
| A4  | `phase_c_errors.rs` "A4 ... consumer poked cp_len_extra_bits"   | [x] |
| A5  | `num_bits_to_read` only ever comes from a `uint8_t` table entry, a literal 1/2/3/4/5/7/16, or `count & 7` (never negative) — **proved unreachable**, asserted absent | [x] |
| A6  | `phase_c_errors.rs` "A6 ... empty and negative in_bytes"        | [x] |
| A7  | `count` is only ever raised by `+32` from below `num_bits_to_read <= 32`, or by `+bits_left` once the words are exhausted (`<= 15 + 24`) — **proved unreachable**, asserted absent | [x] |
| A8  | `phase_c_errors.rs` "A8 ... truncated stream" (256 hits in the sweep) | [x] |
| A9  | `phase_c_errors.rs` "A9 ... code length >= 16"                  | [x] |
| A10 | `phase_c_errors.rs` "A10 ... bogus Huffman entry"               | [x] |
| G1-G16 | `phase_c_boundaries.rs` (16 rows)                           | [x] |

### A1 is reachable, and only just

`bits_left` at `cp_ptr` is congruent mod 8 to the consumed-bit count at the
moment the **final-word** load happened, because that load adds `bits_left`
(not 32) to `count`. So A1 needs a final-word load at a consumed count that is
not a multiple of 8, and ~38 bits of slack left for the stored header. The test
builds exactly that (`in_bytes = 15`, a non-final fixed block of 2 x 8-bit plus
7 x 9-bit literals so the load lands at bit 82, then a stored block with
`LEN = 0xFFFF`, `NLEN = 0`), and a 576-case sweep confirms it independently.

### G16 — the `lens[320]` stack overflow (added during Phase C)

Not an `assert` and not an error return, but a distinct rejection path that the
error surface has to account for: the 16/17/18 run opcodes advance `n` without
re-checking the loop bound, so `n` reaches up to 457 in a 320-byte stack array
and overwrites `cp_dynamic`'s own locals in a fully determined order
(`lenlens`, `sym`, `nlen`, `ndst`, `nlit`, the run counters, `n` itself, the
saved `rbp`, the return address). `tests/phase_c_boundaries.rs` row G16 drives
13 different overflow depths and both libraries agree on all of them.
