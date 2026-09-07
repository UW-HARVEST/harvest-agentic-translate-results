# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c`. There are **no** `assert`s, **no**
null-pointer checks, **no** error enums and **no** min/max constants in the C.
Every rejection funnels through the single local `int ret`, and the function
returns `ret` (always exactly `-1`) instead of `(int)bin_pos` whenever `ret != 0`.

Grep-identified rejection sites (line numbers in `c_src/src/lib.c`):

* L31–L33 `if (bin_pos >= bin_maxlen) { ret = -1; break; }`
* L43–L46 `if (state != 0U) { hex_pos--; ret = -1; }`
* L52–L53 `else if (hex_pos != hex_len) { ret = -1; }`
* L22–L28 `break` on a non-hex byte that is not ignorable (not itself an error,
  but it is the *rejection of input* that feeds L52)
* L55–L56 `if (ret != 0) return ret;`
* L47–L48 `if (ret != 0) bin_pos = 0;` — side effect of every rejection

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| 1 | `hex2bin` | Output buffer full: a valid hex digit is reached while `bin_pos >= bin_maxlen` (`bin_maxlen` smaller than `floor(valid_digits/2)`), `hex_end_p != NULL` | returns `-1`; `*hex_end_p == &hex[i]` where `i` is the index of the first digit that did not fit; bytes already written to `bin[0..bin_maxlen)` are **left in place** (not rolled back) | [x] |
| 2 | `hex2bin` | `bin_maxlen == 0` with at least one valid hex digit at `hex[0]` | returns `-1`; `*hex_end_p == &hex[0]`; nothing written to `bin` | [x] |
| 3 | `hex2bin` | Odd number of valid hex digits consumed, i.e. `state != 0U` when the loop exits (L43) | `hex_pos` is **decremented by one**, returns `-1`; `*hex_end_p == &hex[last_digit_index]` (points *at* the unpaired digit, not past it) | [x] |
| 4 | `hex2bin` | `hex_end_p == NULL` and the scan stopped early on a non-hex, non-ignorable byte (`hex_pos != hex_len`, L52) | returns `-1` | [x] |
| 5 | `hex2bin` | `hex_end_p == NULL`, `ignore == NULL`, and `hex` contains any byte outside `[0-9A-Fa-f]` | returns `-1` (the L22 `break` cannot be skipped because `ignore` is NULL) | [x] |
| 6 | `hex2bin` | `hex_end_p == NULL`, `ignore != NULL`, and an *ignorable* byte appears at an **odd** nibble position (`state != 0U`, so the L23 `state == 0U` guard fails and L28 `break` runs) | returns `-1` — ignorable bytes are only skippable on byte boundaries | [x] |
| 7 | `hex2bin` | `hex_end_p == NULL` and an odd number of valid hex digits (both L45 and L52 fire) | returns `-1` (single sentinel; the two conditions do not compound into a different value) | [x] |
| 8 | `hex2bin` | Any rejection at all (L47) | `bin_pos` is forced to `0` before the return, so the caller can never observe a partial count — the return value is exactly `-1`, never a short positive length | [x] |
| 9 | `hex2bin` | `bin_maxlen` exhausted **and** `hex_end_p == NULL` (L31 sets `ret=-1`, then L50 takes the `else if` branch which cannot clear it) | returns `-1` | [x] |
| 10 | `hex2bin` | Buffer-full rejection where the offending digit is a **high** nibble (`state == 0U`): the `bin_pos >= bin_maxlen` check runs for high nibbles too even though a high nibble writes nothing | returns `-1` (i.e. `hex_len` of `2*bin_maxlen + 1` valid digits still fails, not "succeeds then truncates") | [x] |

## Generic FFI boundary cases (not distinct C branches, tested anyway)

| # | condition | expected C result | [x] |
|---|-----------|-------------------|-----|
| G1 | `hex == NULL`, `hex_len == 0`, `hex_end_p != NULL` | loop never runs; returns `0`; `*hex_end_p == NULL` (`&hex[0]` on a null pointer) | [x] |
| G2 | `bin == NULL`, `bin_maxlen == 0`, `hex_len == 0` | returns `0`; `bin` never dereferenced | [x] |
| G3 | `hex_len == 0` with `hex` non-NULL | returns `0`; `*hex_end_p == hex` | [x] |
| G4 | `ignore == ""` (empty, non-NULL) — `strchr("", c)` still matches `c == 0` | a NUL byte in `hex` is treated as ignorable at even nibbles; any other non-hex byte breaks | [x] |
| G5 | Embedded NUL byte in `hex` with `ignore != NULL` at an **even** nibble | skipped as ignorable (the `strchr` NUL-terminator quirk), scan continues past it | [x] |
| G6 | Embedded NUL byte in `hex` with `ignore == NULL` | `break` immediately (no ignore set to consult) | [x] |
| G7 | Bytes one step outside every accepted range: `/`(0x2F) `:`(0x3A) `@`(0x40) `G`(0x47) `` ` ``(0x60) `g`(0x67) | all rejected as non-hex (`break`) | [x] |
| G8 | High-bit bytes `0x80..0xFF`, incl. `0xC1..0xC6` and `0xE1..0xE6` (which would alias to `A..F`/`a..f` if bit 7 were ignored) | all rejected as non-hex — `c & ~32U` preserves bit 7 | [x] |
| G9 | Oversized `bin_maxlen` (`usize::MAX`) with a short `hex` | succeeds, returns digit count / 2 | [x] |
| G10 | Oversized `hex_len` is **not** testable safely (the C reads `hex[hex_pos]` unconditionally, so a length past the allocation is UB in both languages) — covered instead by "`hex_len` shorter than the buffer", i.e. the scan must stop at `hex_len` and not at a NUL | `*hex_end_p == &hex[hex_len]` on full consumption | [x] |
| G11 | Every one of the 256 byte values, in nibble position 0, 1 and 2, crossed with `ignore ∈ {NULL, "", " \t", "\x00\xFF"}` × `hex_end_p ∈ {NULL, non-NULL}` × `bin_maxlen ∈ {0,1,8}` | identical behaviour in both implementations (12 288 cases) | [x] |

## Notes on the enum-parity requirement

`hex2bin` takes **no enum parameters** (`uint8_t*`, `size_t`, `const char*`,
`size_t`, `const char*`, `const char**`) and returns a plain `int` sentinel, not
an error enum — there is no out-of-range-enum input class for this API. The
equivalent "any bit pattern is a legal input" class here is the `hex` byte
values, which is covered exhaustively: every one of the 256 possible byte values
is driven through both implementations (see `configs` row 24 and
`error_paths::all_256_bytes_*`).

## Where each row is tested

`tests/error_paths.rs`, one test per row (every test loads BOTH `.so`s via
`libloading` and asserts the same sentinel, the same `hex_end_p` and the same
output-window bytes — not merely "both failed"):

| row | test |
|-----|------|
| 1 | `e01_buffer_full_midstream` |
| 2 | `e02_bin_maxlen_zero` |
| 3 | `e03_odd_digit_count_decrements_hex_pos` |
| 4 | `e04_null_hex_end_early_stop` |
| 5 | `e05_null_hex_end_null_ignore_nonhex` (incl. all 246 non-hex byte values) |
| 6 | `e06_ignorable_at_odd_nibble` |
| 7 | `e07_null_hex_end_and_odd_count` |
| 8 | `e08_no_partial_count_ever` |
| 9 | `e09_buffer_full_with_null_hex_end` |
| 10 | `e10_buffer_full_on_high_nibble` (plus the succeeding `2*maxlen` boundary) |
| G1–G3 | `g01_g03_null_and_zero_lengths` |
| G4–G6 | `g04_g06_nul_byte_quirk` |
| G7 | `g07_one_past_each_range_boundary` |
| G8 | `g08_high_bit_bytes_rejected` |
| G9 | `g09_oversized_bin_maxlen` |
| G10 | `g10_hex_len_bounds_the_scan` |
| G11 | `g11_exhaustive_byte_space_all_positions` |

Reproduce: `cd translation && cargo build && cargo test --test error_paths`
(the `cargo build` is required — `cargo test` does **not** rebuild a
`crate-type = ["cdylib"]` artifact, and the harness refuses to run against a
stale `.so`).
