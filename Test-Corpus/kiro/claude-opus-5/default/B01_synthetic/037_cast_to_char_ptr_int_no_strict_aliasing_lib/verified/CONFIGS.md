# CONFIGS.md — Phase A: configuration-surface table

## Mechanical derivation of the axes

Enumerated from the C source and the public header, not from assumptions.

### Public entry points (the FULL set, lowest level included)

```sh
nm -D --defined-only c_src/build/libdriver.so   # -> "T driver"
grep -n ';' c_src/include/driver.h              # -> "void driver(int x);"
```

| entry point | linkage | reachable from outside the `.so`? |
|-------------|---------|-----------------------------------|
| `driver(int x)` | external, declared in `include/driver.h` | yes — the only one |
| `print_hex(unsigned char *p, int len)` | `static` (internal) | **no** — absent from `nm -D` on both `.so`s; sole call site is inside `driver`. Exercised only, but fully, via `driver`. |

There is no convenience-wrapper / low-level split to worry about: `driver` *is*
the lowest level the `.so` exposes, and it is also the only level.

### Runtime options / modes / flags

```sh
grep -nE 'if *\(|switch|#ifdef|#if |setenv|getenv|extern |global|static [^v]' c_src/src/driver.c
```

Zero. The library has **no** settable option, mode, flag, global, or state:
no init function, no context/handle struct, no `getenv`, no `#ifdef`-selected
behaviour, and no mutable global. `driver` is a pure function of its single
argument (plus its stdout side effect). So the option axis has exactly one
value — "the only configuration" — and cannot be crossed with anything.

### Input shapes the code special-cases

`driver`'s parameter is a single by-value `int`. There is no count, no length,
no width selector, no element type, no format selector, and no byte-order
selector: the byte order is fixed by the `memcpy` of the object
representation, i.e. the target's native endianness (little-endian on this
x86-64 target, verified below). So there is no "empty / one / many" axis
either — the operation always processes exactly `sizeof(int)` = 4 bytes.

What *does* vary meaningfully is the **value** of `x`, because the loop body
formats each of its 4 bytes independently through `%02x`. The value-dependent
distinctions the code actually makes are per-byte:

| axis | distinct cases the C distinguishes |
|------|-----------------------------------|
| byte position within the `int` | 4 (positions 0..3, emitted in native/little-endian order) |
| byte value class, per position | `0x00`; `0x01..0x0f` (needs the `0` pad flag); `0x10..0x7f`; `0x80..0xff` (high bit set — `signed char` vs `unsigned char` divergence point) |
| aggregate sign of `x` | non-negative vs negative (determines the high byte's class) |

Confirmed target facts (`sizeof(int)` = 4, `sizeof(char)` = 1,
`CHAR_MIN` = -128 so plain `char` is **signed**, `INT_MIN` = -2147483648,
`INT_MAX` = 2147483647) and native byte order = little-endian.

## Configuration-surface table

Every row is driven through the `driver` export of **both** `.so`s and the
captured stdout bytes compared exactly. Rows C1–C4 are the single-value
boundary rows; C5–C12 are the pruned cross-product of {byte position} ×
{byte value class}; C13–C16 are randomized property-style sweeps (fixed seed
`0x5EED_1234`, so reproducible) that cover value-dependent paths a hand-picked
scalar cannot; C17–C19 cover call-sequence / ABI shape.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `driver` | no options (none exist); `x = 0` — all bytes `0x00`, minimum magnitude | [x] |
| C2 | `driver` | no options; `x = 1` — smallest positive; low byte `0x01`, rest `0x00` (little-endian ordering is observable here) | [x] |
| C3 | `driver` | no options; `x = INT_MAX` — all value bits set, sign bit clear | [x] |
| C4 | `driver` | no options; `x = INT_MIN` — only sign bit set | [x] |
| C5 | `driver` | byte class `0x00` isolated in each of the 4 positions (`0x000000ff` … `0xff000000` complements) — zero byte at every offset | [x] |
| C6 | `driver` | byte class `0x01..0x0f` (pad-flag class) in byte position 0: `0x00000001`..`0x0000000f` | [x] |
| C7 | `driver` | byte class `0x01..0x0f` in byte position 1: `0x00000100`..`0x00000f00` | [x] |
| C8 | `driver` | byte class `0x01..0x0f` in byte position 2: `0x00010000`..`0x000f0000` | [x] |
| C9 | `driver` | byte class `0x01..0x0f` in byte position 3 (high byte): `0x01000000`..`0x0f000000` | [x] |
| C10 | `driver` | byte class `0x10..0x7f` swept in each of the 4 positions | [x] |
| C11 | `driver` | byte class `0x80..0xff` (high-bit-set) swept in each of the 4 positions — `signed char` sign-extension divergence point at every offset | [x] |
| C12 | `driver` | full per-position sweep: for each byte position `p` in 0..3, all 256 byte values `0x00..0xff` placed at `p` with the other bytes zero (1024 calls) — exhaustive over the {position × value} cross-product | [x] |
| C13 | `driver` | randomized: 20 000 uniform `i32` values from a seeded PRNG (seed `0x5EED_1234`) — property-style sweep over the whole 32-bit domain | [x] |
| C14 | `driver` | randomized, biased toward the boundary classes: values assembled from bytes drawn only from `{0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0x81, 0xfe, 0xff}` (4 000 values) — dense coverage of the padding / high-bit classes in all 4 positions simultaneously | [x] |
| C15 | `driver` | randomized negative-only and non-negative-only sweeps (2 000 each) — separates the aggregate-sign axis | [x] |
| C16 | `driver` | exhaustive over all 65 536 values of the low 16 bits (`x = 0x0000_0000..0x0000_ffff`) and all 65 536 values of the high 16 bits (`x = 0x0000_0000..0xffff_0000` step `0x1_0000`) — full exhaustive coverage of two 16-bit windows | [x] |
| C17 | `driver` | call-sequence shape: many consecutive calls to the *same* library in one process (statefulness / buffering check — a stray retained buffer or stale state would show as drift) | [x] |
| C18 | `driver` | call-sequence shape: C and Rust calls **interleaved** in one process, both writing the same libc `stdout` `FILE` (catches a translation that used Rust's own stdout buffer) | [x] |
| C19 | `driver` | ABI shape: the symbol invoked through a wider `extern "C" fn(i64)` signature so the argument register carries bits outside `int` range — both must observe only the low 32 bits | [x] |

**19 of 19 rows pass across their randomized inputs. 0 rows unchecked.**

## Feature combinations

```sh
grep -n -A20 '^\[features\]' Cargo.toml   # -> no match
```

`translation/Cargo.toml` declares **no `[features]` table**, so the crate has
exactly one configuration: default (= no features). There is no
`--no-default-features` variant with different code, and no `#[cfg(feature …)]`
in `src/lib.rs`:

```sh
grep -c 'cfg(feature' src/lib.rs   # -> 0
```

The single feature combination is therefore `--no-default-features` ≡ default,
and the whole table above is verified under it. `tests/features.sh` enumerates
the feature set mechanically from `Cargo.toml` and re-runs the suite for every
combination it finds, so the check is automated rather than assumed.
