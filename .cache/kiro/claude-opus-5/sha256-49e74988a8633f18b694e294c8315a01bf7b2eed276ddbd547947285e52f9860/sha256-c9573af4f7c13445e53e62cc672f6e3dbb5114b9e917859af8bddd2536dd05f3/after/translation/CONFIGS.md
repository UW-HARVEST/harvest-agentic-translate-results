# CONFIGS.md — Phase B configuration-surface table

## How this table was derived

Mechanical enumeration of every axis the C code actually branches on or is
data-dependent upon.

### Public entry points (complete set)

`nm -D --defined-only` on the C `.so` plus the public header give the full list:

| entry point | signature | level |
|-------------|-----------|-------|
| `driver` | `void driver(int floors)` | this is simultaneously the highest **and** the lowest-level public entry point — there is no convenience wrapper and no deeper public layer |

`print_hex` is `static` (internal linkage) and therefore not a public entry
point. It is nonetheless exercised on every call, and its two behaviours
(the `%02x` per-byte formatting loop, and the trailing newline) are covered by
the rows below; there is no way to reach it with any other `(p, len)` pair
because `driver` always passes `(&house, sizeof(house_t))`.

### Runtime options / modes / flags

**None.** Grepped for `if`, `switch`, `#ifdef`, and for any setter, global,
context struct, or option parameter in the header:

* the header declares exactly one function and no types, enums, or globals;
* `driver.c` contains **no** `if`, `switch`, `else`, or conditional
  `#ifdef`/`#if` (the only preprocessor conditional is the `DRIVER_H_` include
  guard);
* there is no global/static mutable state, no init/config function, no
  environment-variable read, and no locale dependence (`%02x` and `\n` are
  locale-invariant).

So the configuration cross-product has exactly **one** option combination: the
empty one. All remaining axes are input **shape** axes.

### Input shape axes the code is sensitive to

| axis | why it matters, from the source |
|------|---------------------------------|
| value of `floors` | copied verbatim into `house.floors`, whose 4 bytes are then printed; every distinct value produces distinct output |
| per-byte value within `floors` | `printf("%02x", p[i])` formats each byte independently; bytes `< 0x10` take the zero-padding path, `>= 0x80` exercise the `unsigned char` → `int` promotion (must not sign-extend) |
| host byte order / struct layout | the struct is printed as raw memory, so field offsets, `sizeof(house_t) == 16`, and little-endian ordering are all observable in the output |
| field-boundary interaction | `bedrooms == 3` and `bathrooms == 2.0` are constants, but they sit adjacent to `floors` in memory, so a wrong offset or wrong padding shows up as a shifted/overlapping byte run |
| padding bytes | `int,int,double` on the LP64 ABI packs to offsets 0/4/8 with `sizeof == 16` and **zero** padding bytes; `house_t house = {0}` still zero-initialises the whole object, so a layout with padding would also be observable |
| `double` bit pattern | `2.0` must serialise as IEEE-754 `0x4000000000000000` in little-endian byte order |
| invocation count / sequence | each call constructs a fresh automatic `house`; output must not depend on previous calls or on which library was called before |
| output framing | exactly `2 * 16` hex characters followed by one `\n`, i.e. 33 bytes, for every input |

## Configuration-surface table

One row per combination the C treats differently. Every row is run against
**both** `.so` files through `libloading` and compared byte-for-byte; rows
marked "randomized" use many inputs from a fixed-seed PRNG.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options (none exist) + `floors = 0` — all-zero low field, minimal case | [x] |
| 2 | `driver` | `floors = 1` — smallest non-zero, single set bit in byte 0 | [x] |
| 3 | `driver` | `floors = 3` — equal to the hard-coded `bedrooms`, so a field-mixup bug is masked in the individual bytes but not in their order | [x] |
| 4 | `driver` | `floors = -1` — all 32 bits set, exercises `unsigned char` promotion on all four bytes (`ff ff ff ff`, must not print `ffffffff` sign-extended per byte) | [x] |
| 5 | `driver` | `floors = INT_MAX` (`0x7fffffff`) — positive extreme, mixed `ff`/`7f` bytes | [x] |
| 6 | `driver` | `floors = INT_MIN` (`0x80000000`) — negative extreme, high bit only | [x] |
| 7 | `driver` | one-hot byte placement: `0x000000ff`, `0x0000ff00`, `0x00ff0000`, `0xff000000` — isolates byte ORDER (catches big/little-endian and offset errors) | [x] |
| 8 | `driver` | one-hot bit placement: all 32 values `1 << k`, `k = 0..31` — isolates every bit position | [x] |
| 9 | `driver` | zero-padding shape: every `floors` in `0x00..=0xFF` — the only formatting branch (`%02x` widening for values `< 0x10`) | [x] |
| 10 | `driver` | byte-boundary shapes: `0x7f/0x80/0x81`, `0x7fff/0x8000/0x8001`, `0x7fffff/0x800000/0x800001` — carry across each byte lane | [x] |
| 11 | `driver` | randomized full 32-bit domain, fixed seed, 20000 inputs — value-dependent paths across the whole domain | [x] |
| 12 | `driver` | randomized small magnitudes, fixed seed (`-1024..=1024`), 4096 inputs — dense coverage near zero where sign handling changes | [x] |
| 13 | `driver` | randomized bytewise-sparse values, fixed seed (values assembled from `{0x00,0x01,0x0f,0x10,0x7f,0x80,0xfe,0xff}` in each of the 4 lanes, all 4096 combinations) — full cross-product of the per-byte formatting cases | [x] |
| 14 | `driver` | invocation sequence: the same `floors` called twice in a row, then a different value, then the first again — no residual state | [x] |
| 15 | `driver` | interleaved cross-library sequence: C, Rust, C, Rust … in one process with shared `stdout` — no cross-contamination or buffering divergence | [x] |
| 16 | `driver` | structural invariants observed on random inputs: output is exactly 33 bytes, is lowercase hex + `\n`, bytes 8..16 are `03000000`, bytes 16..32 are `0000000000000040` (constant `bedrooms`/`bathrooms` image incl. the `sizeof == 16` / zero-padding layout) | [x] |
