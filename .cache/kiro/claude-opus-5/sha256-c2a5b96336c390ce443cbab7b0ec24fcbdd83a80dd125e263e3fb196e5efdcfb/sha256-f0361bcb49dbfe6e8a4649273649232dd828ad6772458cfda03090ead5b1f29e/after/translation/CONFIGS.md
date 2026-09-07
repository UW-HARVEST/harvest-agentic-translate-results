# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**Runtime options / modes / flags:** none. The C source has no global state, no
context/handle struct, no option setters, no `switch`, and no `#ifdef`
(`grep '#if\|switch\|enum' c_src/src/lib.c` → no matches). The only public
header declares a single function. Therefore the entire configuration surface
is the 4-tuple of arguments.

**Public entry points (full set, lowest level included):**

| entry point | level |
|-------------|-------|
| `bin2hex(char *hex, size_t hex_maxlen, const uint8_t *bin, size_t bin_len)` | the only symbol; it *is* the lowest-level entry point — there is no convenience wrapper to hide behind |

**Input-shape axes (from the branches the C takes):**

- `A` — `bin_len` → loop trip count: `0` (loop body never runs), `1`, `2`,
  odd `> 1`, even `> 1`, large, `256`. Also the guard boundary at `LIMIT`
  (covered in `ERRORS.md`).
- `B` — high nibble `b = bin[i] >> 4` → the branch-free select
  `(((b - 10U) >> 8) & ~38U)` behaves differently for `b < 10` vs `b >= 10`.
  Boundary values: `0`, `9`, `10`, `15`.
- `C` — low nibble `c = bin[i] & 0xf` → same select, same boundaries
  `0`, `9`, `10`, `15`. `b` and `c` are **independent** axes (the two halves of
  `x` are computed separately), so their cross-product matters.
- `D` — `hex_maxlen` slack past the guard: exactly `bin_len*2+1` (minimum
  valid), `bin_len*2+2`, generous, `SIZE_MAX`.
- `E` — `bin` pointer: non-null, or NULL when `bin_len == 0` (legal, since the
  loop never dereferences it).
- `F` — pointer alignment / offset of `hex` and `bin` within their allocations
  (the C uses byte accesses only, but this is a distinct shape a consumer
  passes).
- `G` — buffer overlap between `hex` and `bin` (the C does no overlap check, so
  the aliasing behaviour is part of the contract and must match).
- `H` — observable outputs to compare: the written bytes, the NUL terminator
  position, the bytes *past* `bin_len*2+1` (must be untouched), and the
  returned pointer (must be identically `hex`).

## Rows (pruned cross-product of the axes the C distinguishes)

Every row is run against BOTH `.so`s via `libloading` and compared
byte-for-byte over the whole output buffer plus the returned pointer.
Rows marked *randomized* use many inputs from a fixed-seed PRNG.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `bin2hex` | `bin_len = 0`, `hex_maxlen = 1` (exact min), `bin` non-null → only the NUL write | [x] |
| C2 | `bin2hex` | `bin_len = 0`, `hex_maxlen = 1`, `bin = NULL` (axis E: legal null) | [x] |
| C3 | `bin2hex` | `bin_len = 0`, `hex_maxlen = 64` (generous slack, axis D) | [x] |
| C4 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3` (exact min), byte `0x00` → `b=0, c=0` (both below 10) | [x] |
| C5 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3`, byte `0x99` → `b=9, c=9` (both at the `<10` boundary) | [x] |
| C6 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3`, byte `0xAA` → `b=10, c=10` (both at the `>=10` boundary) | [x] |
| C7 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3`, byte `0xFF` → `b=15, c=15` (both max) | [x] |
| C8 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3`, byte `0x9A` → `b=9, c=10` (mixed: high below, low at/above) | [x] |
| C9 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3`, byte `0xA9` → `b=10, c=9` (mixed, other direction) | [x] |
| C10 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 3`, **exhaustive** over all 256 byte values (full B×C cross-product) | [x] |
| C11 | `bin2hex` | `bin_len = 1`, `hex_maxlen = 4096` (slack far past the minimum; slack must stay untouched) | [x] |
| C12 | `bin2hex` | `bin_len = 2`, `hex_maxlen = 5` (exact min), *randomized* bytes | [x] |
| C13 | `bin2hex` | `bin_len = 2`, `hex_maxlen = 6` (min + 1, axis D) | [x] |
| C14 | `bin2hex` | `bin_len` odd `∈ {3,5,7,9,11,13,15,17,31,63}`, exact-min `hex_maxlen`, *randomized* | [x] |
| C15 | `bin2hex` | `bin_len` even `∈ {4,6,8,10,12,16,32,64}`, exact-min `hex_maxlen`, *randomized* | [x] |
| C16 | `bin2hex` | `bin_len = 256` with `bin[i] = i` (every byte value once, in order) | [x] |
| C17 | `bin2hex` | `bin_len = 4096`, exact-min `hex_maxlen`, *randomized* | [x] |
| C18 | `bin2hex` | `bin_len = 4096`, generous `hex_maxlen` (`= 2*4096 + 1 + 977`) | [x] |
| C19 | `bin2hex` | `bin_len` sweep `0..=64`, exact-min `hex_maxlen`, *randomized* content per length | [x] |
| C20 | `bin2hex` | `bin_len` sweep `0..=64`, `hex_maxlen = bin_len*2 + 2` | [x] |
| C21 | `bin2hex` | `hex_maxlen = SIZE_MAX` (extreme axis-D value) with small `bin_len` and a real small buffer | [x] |
| C22 | `bin2hex` | content pattern: all `0x00` bytes, `bin_len = 33` | [x] |
| C23 | `bin2hex` | content pattern: all `0xFF` bytes, `bin_len = 33` | [x] |
| C24 | `bin2hex` | content pattern: alternating `0x0F`/`0xF0`, `bin_len = 33` (nibble halves swap each byte) | [x] |
| C25 | `bin2hex` | content pattern: only nibbles `9`/`10` (`0x99,0x9A,0xA9,0xAA`) repeated, `bin_len = 40` (packs the select boundary densely) | [x] |
| C26 | `bin2hex` | axis F: `hex` and `bin` at every offset `0..8` inside their allocations (unaligned pointers), `bin_len = 7` | [x] |
| C27 | `bin2hex` | axis G: `bin` overlaps the tail of the `hex` buffer (`bin = hex + bin_len`, in-place-style conversion), `bin_len = 16` | [x] |
| C28 | `bin2hex` | axis G: `bin` overlaps the *front* of the `hex` buffer (`bin = hex`, destructive aliasing), `bin_len = 8` | [x] |
| C29 | `bin2hex` | axis H: returned pointer identity `== hex` checked for both `.so`s, over `bin_len ∈ {0,1,7,64}` | [x] |
| C30 | `bin2hex` | full *randomized* property sweep, fixed seed: 4000 cases, random `bin_len ∈ 0..512`, random `hex_maxlen ∈ [min, min+64]`, random bytes, sentinel-filled buffers compared in full | [x] |

## Binary / driver executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`. `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` and no `[[bin]]`. **No binary is built by either
side**, so the "compare binary stdout" gate is not applicable.
