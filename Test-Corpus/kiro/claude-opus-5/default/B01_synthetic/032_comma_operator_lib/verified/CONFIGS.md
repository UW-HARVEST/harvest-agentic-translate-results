# CONFIGS.md — Phase B configuration-surface table

Derived **mechanically** from the axes the C code actually branches on.

## Mechanical derivation of the axes

Public API surface (`c_src/include/driver.h`), complete:

```c
void driver(int x);
```

There is exactly **one** public entry point and it is simultaneously the
lowest-level one — there is no convenience wrapper layered over a lower API, so
"exercise the low-level entry points, not just the wrappers" is satisfied by
calling `driver` itself.

Runtime options / modes / flags: **none**. Grep found no global/static state, no
setters, no option struct, no `#ifdef`-gated behaviour, no environment reads.
The only compile-time preprocessor directives are the `#include` and the header
include guard.

Body, complete:

```c
for (int i = 0, j = 0; i < x; i++, j += 2) {
    printf("%d %d\n", i, j);
}
```

Axes the code actually distinguishes:

| axis | why it is an axis | values that matter |
|------|-------------------|--------------------|
| sign of `x` | `i < x` with `i` starting at 0 decides zero vs. non-zero iterations | negative, zero, positive |
| magnitude of `x` | number of `printf` calls; also determines the **decimal width** of `i` and `j` in `%d`, which is the only formatting variation the code can produce | 1, 2..9, 10..99, ... up to 7+ digits |
| digit-width skew between `i` and `j` | `j == 2*i`, so `j` gains a decimal digit before `i` does; crossing `j`'s power-of-ten boundaries is a distinct output shape | `x` near 5, 50, 500, 5000, ... |
| `stdout` buffer boundary | `printf` to a fully-buffered stream flushes on 4096-byte boundaries; output must be identical regardless of where lines straddle the boundary | `x` chosen so total bytes straddle 4 KiB / 64 KiB |
| signed overflow of `j` | `j += 2` overflows `int` at `i == 2^30` (UB in C) | `x > 2^30` |
| call sequencing / residual state | the function has no statics, so repeated and interleaved C/Rust calls must be independent | repeat, alternate C→Rust→C |

There are no other axes: no input buffers, no element types, no widths, no byte
order, no counts, no formats. The cross-product below is the full set of
combinations the C treats differently, pruned to those distinctions.

## Configuration-surface table

Every row is exercised with **many randomized inputs** drawn from a fixed-seed
xorshift PRNG (seed `0x5EED_1234_ABCD_F00D`), not a single hand-picked value.
"C-vs-Rust byte-identical stdout" is the assertion in every row.

| #   | entry point(s) | configuration (options set + input shape) | [x] |
|-----|----------------|--------------------------------------------|-----|
| C1  | `driver` | `x` = every value in `-8..=64` exhaustively (empty / one / few / many, plus the whole sign transition) | [x] |
| C2  | `driver` | `x` randomized in `1..=9` — single-digit `i`, `j` crosses from 1 to 2 digits at `i>=5` | [x] |
| C3  | `driver` | `x` randomized in `10..=99` — 2-digit `i`, `j` reaches 3 digits | [x] |
| C4  | `driver` | `x` randomized in `100..=999` — 3-digit `i`, `j` reaches 4 digits | [x] |
| C5  | `driver` | `x` randomized in `1_000..=9_999` — 4-digit `i`, `j` reaches 5 digits | [x] |
| C6  | `driver` | `x` randomized in `10_000..=99_999` — 5-digit `i`, `j` reaches 6 digits | [x] |
| C7  | `driver` | `x` randomized in `100_000..=999_999` — 6-digit `i`, `j` reaches 7 digits | [x] |
| C8  | `driver` | `x` randomized in `1_000_000..=4_000_000` — 7-digit `i`, `j` reaches 7 digits; output > 40 MiB, many buffer flushes | [x] |
| C9  | `driver` | `x = 5_000_000` — digest pipeline cross-checked against an independently computed model digest (guards the C9/C13 method itself) | [x] |
| C9b | `driver` | `x = 2^30` (`1073741824`) — `j` reaches `2147483646`, the largest value that still fits, **without** overflowing. 21,955,653,463 bytes streamed per side; digests equal (`0x9f861b5ee8f98b2d`) | [x] |
| C9c | `driver` | `x = 2^30 + 5` — the **signed-overflow** region: `j` wraps `2147483646 → -2147483648` and the emitted lines gain a `-`. 21,955,653,578 bytes per side; digests equal (`0xa6371096caa70656`); final line `1073741828 -2147483640` on both | [x] |
| C10 | `driver` | `x` at each power-of-ten boundary and boundary±1 (`9,10,11,99,100,101,...,1_000_001`) — decimal-width transitions for `i` | [x] |
| C11 | `driver` | `x` at each `j`-digit boundary and ±1 (`x = 5, 6, 50, 51, 500, 501, ...`) — decimal-width transitions for `j` only | [x] |
| C12 | `driver` | `x` chosen so total output straddles the 4 KiB / 8 KiB / 64 KiB / 128 KiB `stdout` buffer boundary exactly, and ±1, ±2 either side | [x] |
| C13 | `driver` | `x = i32::MAX` (`2147483647`) — the **full** run, 2,147,483,647 iterations, `j` wrapping past the halfway point. 46,096,159,855 bytes streamed per side; digests equal (`0x285c20ffc2afca05`) | [x] |
| C14 | `driver` | repeated invocation: same `x` called 50× in a row on the same loaded handle — asserts no residual state accumulates in either library | [x] |
| C15 | `driver` | interleaved invocation: `C(x1) → Rust(x1) → C(x2) → Rust(x2) → …` over 200 randomized `x`, all sharing the one process-wide libc `stdout` — asserts the shared `FILE*` is left in the same state by both | [x] |
| C16 | `driver` | `stdout` in **line-buffered** mode (fd 1 = a tty-like pipe with `setvbuf(_IOLBF)`) vs **fully-buffered** (regular file) vs **unbuffered** (`_IONBF`) — the only stream-mode axis reachable from outside; identical bytes required in all three | [x] |
| C17 | `driver` | randomized `x` over the full negative range `i32::MIN..=-1` (no output; pairs with ERRORS E3) | [x] |
| C18 | `driver` | randomized `x` over `1..=2000` interleaved with negative `x` in the same process, to catch any `i`/`j` state leaking across calls | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(driver SHARED src/driver.c)`
and installs headers/lib. There is **no** `add_executable`, and
`translation/Cargo.toml` declares only `crate-type = ["cdylib"]` with no
`[[bin]]`. The "compare C and Rust binary stdout" requirement is therefore
**not applicable** — there is no driver binary in either tree.

## Feature combinations

`translation/Cargo.toml` has **no** `[features]` section and no optional
dependencies, so the complete set of feature combinations is the single default
one. Verified mechanically by `run_all.sh`, which parses `[features]` out of
`Cargo.toml`, builds the power set, and loops `cargo check` + the full test
suite over every combination it finds. Reported output:

```
declared non-default features: 0  [none]
combinations to verify: 1
```

## How the rows were run

* Rows C1–C8, C10–C12, C14–C18 plus a 400-input sweep over the whole `i32`
  domain: `tests/valid_paths.rs`, full byte-for-byte comparison of the captured
  fd-1 output. 17 tests, all passing (~255 s).
* Rows C9/C9b/C9c/C13: `tests/overflow.rs`, `#[ignore]`d because each takes
  1–6 minutes. Output is streamed through a pipe and compared as
  (FNV-1a 64 digest, byte count, line count, first 128 bytes, last 128 bytes)
  because 22–46 GiB per side cannot be held in memory. C9 validates that
  pipeline against an independently computed model digest first.
* Every row additionally cross-checks the **C** output against an independent
  Rust model of the loop, so a harness that silently captured nothing on both
  sides cannot pass.

## Non-vacuity check

The suite was validated by deliberate sabotage and then reverted:

* changing `j.wrapping_add(2)` → `wrapping_add(3)` in `src/lib.rs` made
  Phase B/C tests fail with `divergence for driver(...) ... first differing
  byte at 6`;
* renaming the exported symbol to `driver_renamed` made `tests/symbols.rs`
  fail with `symbols exported by the C .so but missing from the Rust .so:
  ["driver"]`.

## Gate

- [x] Every row above passes across its randomized inputs
      (`translation/tests/valid_paths.rs`, plus `tests/overflow.rs` for C9/C13).
