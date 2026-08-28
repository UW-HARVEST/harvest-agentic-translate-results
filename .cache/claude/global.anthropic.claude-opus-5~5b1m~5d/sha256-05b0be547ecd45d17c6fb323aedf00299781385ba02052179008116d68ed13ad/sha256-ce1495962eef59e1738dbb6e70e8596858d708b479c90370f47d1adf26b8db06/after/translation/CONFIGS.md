# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Axes the C code actually branches on

Derived from `c_src/include/lib.h` (the whole public API) and every branch in
`c_src/src/lib.c`:

**Public entry points (complete set).** `lib.h` declares exactly one function,
`bin2hex`. There is no init/context/one-shot split, no convenience wrapper over
a lower-level primitive, and no state — so the "lowest-level entry point" *is*
`bin2hex`, and it is what every row below drives directly through the `.so`.

**Runtime options / modes / flags.** None. There is no options struct, no flag
argument, no global mode setter, no `#ifdef` in `lib.c` or `lib.h`. The only
compile-time knob in `CMakeLists.txt` is `POSITION_INDEPENDENT_CODE`, which does
not change semantics. So the configuration space is spanned purely by **input
shape**, enumerated next.

**Input-shape axes** (the things the code's control flow and arithmetic depend on):

| axis | why the C distinguishes it | values covered |
|------|---------------------------|----------------|
| `bin_len` | loop trip count `while (i < bin_len)`; also feeds both abort conditions | `0`, `1`, `2`, `3`, odd/even sweep `0..=64`, `255`, `256`, `4096`, `65536` |
| `hex_maxlen` slack | `hex_maxlen <= bin_len*2U` is the only use; valid iff `>= bin_len*2 + 1` | exact minimum `2n+1`, `2n+2`, `2n+64`, `usize::MAX` |
| low nibble `c = bin[i] & 0xf` | `((c - 10U) >> 8) & ~38U` takes the wrap-around path for `c < 10` and the zero path for `c >= 10` | both sides of the `c == 9 / c == 10` boundary, and all 16 values |
| high nibble `b = bin[i] >> 4` | same branch-free correction, independent of `c` | both sides of the `b == 9 / b == 10` boundary, and all 16 values |
| nibble pair `(b, c)` | the two corrections are packed into one `unsigned int` (`lo << 8 \| hi`) and unpacked by `(char)x` / `x >>= 8`; a byte-order or packing error only shows when `b != c` | all 256 byte values, exhaustively |
| byte position parity | writes go to `hex[2i]` (high nibble) then `hex[2i+1]` (low nibble); a swap is invisible when `b == c` | asymmetric bytes at every index |
| terminator position | `hex[i*2] = 0` executes with `i == bin_len`, including `bin_len == 0` | every `bin_len` row asserts the NUL index and that nothing past it is touched |
| buffer aliasing | reads `bin[i]` then writes `hex[2i]`, `hex[2i+1]`; when the buffers overlap the read/write interleaving order is observable | `hex == bin`, `hex == bin - k`, `hex == bin + k`, disjoint |
| return value | `return hex` — must be the *same* pointer, not a copy | asserted in every row |

## Rows (each = one meaningful combination the C treats differently)

Every row calls **both** `.so`s through `libloading` with identical inputs and
compares (a) the full destination buffer byte-for-byte, including untouched
canary bytes, and (b) the returned pointer. Rows marked *randomized* use a
fixed-seed xorshift PRNG (`seed = 0x243F6A8885A308D3`) with many iterations, not
one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|-------------------------------------------|------|---|
| 1 | `bin2hex` | `bin_len = 0`, `hex_maxlen = 1` (minimum valid), `bin = NULL` and `bin = valid` | `cfg01_empty_min_maxlen` | [x] |
| 2 | `bin2hex` | `bin_len = 0`, `hex_maxlen ∈ {2, 64, usize::MAX}`, canary-filled `hex` | `cfg02_empty_slack_maxlen` | [x] |
| 3 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3` (exact minimum), **all 256 byte values** exhaustively | `cfg03_single_byte_all_256_values` | [x] |
| 4 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 64` (slack), all 256 values, canary beyond output must survive | `cfg04_single_byte_slack_canary` | [x] |
| 5 | `bin2hex` | `bin_len = 2`, `hex_maxlen = 5` (exact minimum), **all 65536 byte pairs** exhaustively (catches nibble/byte-order swaps) | `cfg05_two_bytes_all_pairs` | [x] |
| 6 | `bin2hex` | `bin_len = 3`, `hex_maxlen = 7`, *randomized* (odd length, exact minimum) | `cfg06_three_bytes_random` | [x] |
| 7 | `bin2hex` | `bin_len` sweep `0..=64` (odd and even), `hex_maxlen = 2n+1` (exact minimum), *randomized* per length | `cfg07_len_sweep_min_maxlen` | [x] |
| 8 | `bin2hex` | `bin_len` sweep `0..=64`, `hex_maxlen = 2n+2` (one byte of slack), *randomized* | `cfg08_len_sweep_one_slack` | [x] |
| 9 | `bin2hex` | `bin_len` sweep `0..=64`, `hex_maxlen = usize::MAX` (maximum allowed), *randomized* | `cfg09_len_sweep_maxlen_usize_max` | [x] |
| 10 | `bin2hex` | `bin_len = 256`, `bin[i] = i` (sequential — every byte value at a distinct index, checks index arithmetic) | `cfg10_sequential_256` | [x] |
| 11 | `bin2hex` | `bin_len = 256`, `bin[i] = 255 - i` (reverse sequential) | `cfg11_reverse_sequential_256` | [x] |
| 12 | `bin2hex` | `bin_len = 64`, all bytes `0x00` (both nibbles on the `< 10` wrap path, minimum) | `cfg12_all_zero_bytes` | [x] |
| 13 | `bin2hex` | `bin_len = 64`, all bytes `0xFF` (both nibbles on the `>= 10` path, maximum) | `cfg13_all_ff_bytes` | [x] |
| 14 | `bin2hex` | `bin_len = 64`, bytes drawn only from `0x00..=0x0F` (high nibble 0, low nibble digit-or-letter), *randomized* | `cfg14_low_nibble_only` | [x] |
| 15 | `bin2hex` | `bin_len = 64`, bytes drawn only from `{0x00,0x10,…,0xF0}` (low nibble 0), *randomized* | `cfg15_high_nibble_only` | [x] |
| 16 | `bin2hex` | `bin_len = 64`, bytes with both nibbles in `0x0..=0x9` (digit/digit — wrap path both halves), *randomized* | `cfg16_digit_digit_nibbles` | [x] |
| 17 | `bin2hex` | `bin_len = 64`, bytes with both nibbles in `0xA..=0xF` (letter/letter — zero-correction both halves), *randomized* | `cfg17_letter_letter_nibbles` | [x] |
| 18 | `bin2hex` | `bin_len = 64`, mixed digit/letter nibbles (`b < 10 <= c` and `c < 10 <= b`), *randomized* | `cfg18_mixed_nibble_classes` | [x] |
| 19 | `bin2hex` | `bin_len = 512`, bytes cycled through the nibble-boundary set `{0x09,0x0A,0x90,0xA0,0x99,0x9A,0xA9,0xAA,0x0F,0xF0,0xFF,0x00}` | `cfg19_nibble_boundary_bytes` | [x] |
| 20 | `bin2hex` | `bin_len = 4096`, *randomized* (large buffer, exact-minimum `hex_maxlen`) | `cfg20_large_4096_random` | [x] |
| 21 | `bin2hex` | `bin_len = 65536`, *randomized* (very large buffer, spans many pages) | `cfg21_very_large_65536_random` | [x] |
| 22 | `bin2hex` | fully *randomized* property sweep: random `bin_len ∈ 0..=1024`, random `hex_maxlen ∈ 2n+1 ..= 2n+1+slack`, random bytes, 2000 iterations | `cfg22_property_sweep` | [x] |
| 23 | `bin2hex` | in-place aliasing: `hex == bin` (same buffer), `bin_len` sweep `0..=32`, *randomized* — observable read/write interleaving | `cfg23_alias_hex_eq_bin` | [x] |
| 24 | `bin2hex` | partial overlap, output ahead of input: `bin = base + k`, `hex = base`, several `k`, *randomized* | `cfg24_overlap_hex_before_bin` | [x] |
| 25 | `bin2hex` | partial overlap, output behind input: `bin = base`, `hex = base + k`, several `k`, *randomized* | `cfg25_overlap_hex_after_bin` | [x] |
| 26 | `bin2hex` | unaligned `bin` and `hex` (odd byte offsets into the allocation), `bin_len` sweep, *randomized* | `cfg26_unaligned_pointers` | [x] |
| 27 | `bin2hex` | `hex` written up to the very last byte of a page followed by a `PROT_NONE` guard page (`hex_maxlen = 2n+1` exactly at the page end) — proves no byte past `2n` is written | `cfg27_exact_fit_against_guard_page` | [x] |
| 28 | `bin2hex` | return value is the *identical* pointer (not a copy/offset) for every shape above, incl. `bin_len = 0` and NULL-adjacent offsets | `cfg28_returns_same_pointer` | [x] |

All rows are implemented in `tests/differential_valid.rs`.
