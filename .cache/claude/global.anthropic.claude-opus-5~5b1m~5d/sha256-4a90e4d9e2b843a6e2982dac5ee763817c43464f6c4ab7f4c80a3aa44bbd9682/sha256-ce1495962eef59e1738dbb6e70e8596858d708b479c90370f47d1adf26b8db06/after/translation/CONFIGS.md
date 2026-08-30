# CONFIGS.md — Phase A configuration-surface table

Derived **mechanically** from the C source, headers and `CMakeLists.txt`.

## Axes the C code actually distinguishes

**Runtime options / modes / flags:** *none.* The public header declares one
function and no state, no context struct, no init/config call, no globals, no
`#ifdef` other than the header guard. `grep` for `if`/`switch`/`#if` finds zero
real branches (see `ERRORS.md`). So there is exactly **one** mode.

**Public entry points (the FULL set, including the lowest level):**

| entry point | exported? | declared in `driver.h`? | role |
|-------------|-----------|-------------------------|------|
| `printHexCharLine(char)` | yes (`nm -D` → `T`) | **no** — header-private but ABI-public | the low-level formatter/printer |
| `driver(char)` | yes (`nm -D` → `T`) | yes | the convenience wrapper: `data + 1` then calls the low-level one |

`driver` is the one-shot wrapper; `printHexCharLine` is the lowest-level entry
point and is therefore tested **directly**, not only through `driver`.

**Input shapes the code effectively special-cases** (via the `char`→`int`
promotion and the `%02x` conversion, which is where all value-dependent
behaviour lives):

* S1 `charHex == 0` — `%02x` pads both digits → `00`
* S2 `0x01..0x0f` — one significant digit, `%02x` pads one → `0f`
* S3 `0x10..0x7f` — exactly two digits, no padding → `7f`
* S4 `0x80..0xff` (negative signed `char`) — sign-extends to negative `int`,
  `%x` reinterprets as `unsigned` → **eight** digits, `ffffff80`…`ffffffff`
* S5 the `0x7f`/`0x80` transition — signed-overflow boundary of `data + 1`
* S6 the `0xff`→`0x00` transition — wrap-around boundary of `data + 1`
* S7 exhaustive: **all 256** bit patterns (the domain is finite, so the
  cross-product can be covered exhaustively, not just sampled)
* S8 ABI shape: value delivered in a full 32-bit register with non-zero upper
  bits (caller-declared-as-`int`), which the callee truncates
* S9 call *sequence* / repetition shape: many calls in a row, so that stdio
  buffering, output interleaving and ordering across the two `.so`s are
  compared, not just a single isolated call
* S10 byte-order / width: not applicable — the only datum is 1 byte wide, so
  there is no endianness axis (documented here for completeness)

## Configuration surface (cross-product, pruned to what C distinguishes)

Every row is exercised against **both** `.so`s via `libloading`, with
randomized inputs (fixed seed `0x5EED_D1FF`) inside the row's value class in
addition to the listed boundary values.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `printHexCharLine` | S1: `charHex = 0` — double zero padding | [x] |
| 2 | `printHexCharLine` | S2: `charHex` random in `0x01..=0x0f` — single zero padding | [x] |
| 3 | `printHexCharLine` | S3: `charHex` random in `0x10..=0x7f` — no padding, two digits | [x] |
| 4 | `printHexCharLine` | S4: `charHex` random in `0x80..=0xff` — negative, sign-extended, eight digits | [x] |
| 5 | `printHexCharLine` | S3/S4 boundary pair: `0x7f` then `0x80` | [x] |
| 6 | `printHexCharLine` | S7: exhaustive sweep over all 256 bit patterns, one call per capture | [x] |
| 7 | `printHexCharLine` | S8: symbol re-declared as `fn(c_int)`, random full-width 32-bit values incl. `i32::MIN`, `i32::MAX`, `0x1234_5678`, `256`, `-1` | [x] |
| 8 | `printHexCharLine` | S9: 256 calls in one capture (exhaustive, ascending) — ordering + buffering | [x] |
| 9 | `printHexCharLine` | S9: 1000 randomized calls in one capture — bulk interleaving | [x] |
| 10 | `driver` | S1 input: `data = 0` → `result = 1` → `01` | [x] |
| 11 | `driver` | S2 input: `data` random in `0x00..=0x0e` (result stays single-digit) | [x] |
| 12 | `driver` | S3 input: `data` random in `0x0f..=0x7e` (result two digits, no wrap) | [x] |
| 13 | `driver` | S4 input: `data` random in `0x80..=0xfe` (negative, result still negative → eight digits) | [x] |
| 14 | `driver` | S5 boundary: `data = 0x7f` → signed-overflow narrowing → `result = -128` → `ffffff80` | [x] |
| 15 | `driver` | S6 boundary: `data = 0xff` (`-1`) → `result = 0` → `00` | [x] |
| 16 | `driver` | S7: exhaustive sweep over all 256 bit patterns, one call per capture | [x] |
| 17 | `driver` | S8: symbol re-declared as `fn(c_int)`, random full-width 32-bit values incl. `i32::MIN`, `i32::MAX`, `0x1234_5678`, `256`, `-1` | [x] |
| 18 | `driver` | S9: 256 calls in one capture (exhaustive, ascending) — ordering + buffering | [x] |
| 19 | `driver` | S9: 1000 randomized calls in one capture — bulk interleaving | [x] |
| 20 | `driver` + `printHexCharLine` | S9 composed pipeline: interleaved random calls to *both* entry points in a single capture, so the wrapper and the low-level function are exercised together in one output stream | [x] |
| 21 | `driver` vs `printHexCharLine` | consistency of the composition the C performs internally: for every `d`, `driver(d)` output must equal `printHexCharLine(d.wrapping_add(1))` output, checked on both libraries | [x] |
| 22 | both | S10: single-byte datum — no endianness/width axis; asserted by checking the captured output contains no bytes beyond the ASCII hex + `\n` set over the exhaustive sweep | [x] |
| 23 | `driver` (→ `printHexCharLine`) | S11 **link-time/ABI axis**: `printHexCharLine` is a non-`static` global, so gcc emits `call printHexCharLine@plt` inside `driver` — an *interposable* call. Probed by a C consumer that `dlopen`s each `.so` with and without an `LD_PRELOAD`ed replacement `printHexCharLine`, for BOTH the `debug` and `release` Rust artifacts. | [x] |

### Row → test mapping (Phase B)

Rows 1–22 are `cfg_row01_…` … `cfg_row22_…` in
`translation/tests/phase_b_configs.rs` (one test per row, same numbering).
Row 23 is `sym_internal_call_is_interposable_like_the_c` in
`translation/tests/phase_d_symbols.rs`, because it needs an out-of-process
consumer and therefore lives with the other ABI-level checks.

### Note on axis S11

This axis is not a runtime *option*, but it is a configuration the C code
genuinely branches on at the link level, and it turned up a real divergence —
see "Divergence found and fixed" in `SYMBOLS.md`. It is the reason the test
suite is run against every built profile rather than just the one matching the
test binary.

## Feature combinations

`Cargo.toml` has **no `[features]` section**, so `default`, `--all-features`
and `--no-default-features` all compile the identical crate. Rows above are
therefore complete for every feature combination; the runner script
`run_all.sh` still executes all three explicitly.
