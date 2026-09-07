# CONFIGS.md — configuration-surface table

The mirror of `ERRORS.md`, for **valid** inputs. Axes derived mechanically from
the source, not guessed.

## Axis derivation

### Axis 1 — build-time / feature configuration

```sh
$ grep -nE "\[features\]|^[a-z-]+ = " translation/Cargo.toml   # no [features] section
$ grep -nE "#if|#ifdef|#ifndef" -r c_src/src c_src/include
c_src/include/driver.h:24:#ifndef DRIVER_H_        <- include guard only
$ grep -nE "option\(|add_definitions|target_compile_definitions" c_src/CMakeLists.txt
(no matches)
```

* Rust crate declares **no** `[features]` → the only feature combination is the
  default (empty) one. `--no-default-features` is therefore identical to the
  default build, and both are exercised (see Phase D).
* C build declares **no** CMake `option()`s and **no** `#ifdef` other than the
  header include guard → exactly one C build configuration.
* No `[[bin]]` target in `Cargo.toml` and no `add_executable` in
  `CMakeLists.txt` → the project builds **no binary driver**, so there is no
  stdout-of-executable comparison to perform. Only `add_library(driver SHARED)`.

### Axis 2 — runtime options / modes / flags

```sh
$ grep -nE "\bif\b|\bswitch\b|\?|&&|\|\||static [a-z_]+ [a-z_]+ =|extern" -r c_src/src c_src/include
(no matches other than the include guard)
```

**Zero** runtime options: no flags, no modes, no global/static mutable state, no
setters, no init/teardown, no environment variables read. The library is a pure
function of its single argument (plus the `stdout` stream it writes to).

### Axis 3 — public entry points (full set, including the lowest level)

| entry point | linkage | reachable from a differential test? |
|-------------|---------|--------------------------------------|
| `void driver(int x)` | external, in `include/driver.h`, `T` in `nm -D` | YES — this is simultaneously the highest **and** the lowest-level public entry point; the library has exactly one |
| `static void print_hex(unsigned char *p, int len)` | internal (`static`) | NO — not in `nm -D` for the C `.so`, so it is not part of the surface under test. Its behaviour is verified transitively (the 16 byte-pairs + newline that `driver` emits are produced entirely by it) |

There is no convenience/one-shot wrapper hiding a lower layer: `driver` *is* the
low-level entry point.

### Axis 4 — input shapes the code special-cases

`driver` takes one `int`. The code contains no branch on its value, so the C
distinguishes inputs only through the **byte image** that `%02x` renders. The
meaningful shape classes are therefore the distinct byte-pattern classes of a
32-bit two's-complement integer, plus the interaction with the struct layout:

* `house_t` layout (`int floors; int bedrooms; double bathrooms;`):
  offsets 0 / 4 / 8, `sizeof == 16`, `alignof == 8`, **no interior padding** and
  no tail padding on the LP64 target — so the `{0}` initialiser's treatment of
  padding bytes is not an observable axis here, but the test asserts the total
  output length (33 bytes) on every row so any layout drift is caught.
* `bedrooms` is always `3` and `bathrooms` always `2.0` — constant across all
  configurations; their byte images (`03000000` and `0000000000000040`) act as a
  layout/endianness fingerprint checked by every row.
* `print_hex` loop shape: `len` is always the constant `16`, so exactly one loop
  trip-count (16) is reachable — the "empty / one / many" axis collapses to
  "many", and the row set instead varies the *data* the loop renders.

## Configuration table

One row per combination the C actually treats differently (cross-product of
axes 1–4, pruned to reachable combinations). Every row is driven through **both**
`.so` files via `libloading`, with **many randomized inputs** (fixed seed
`0x5EED_1234_5678_9ABC`, SplitMix64) plus the named boundary values, and stdout
compared byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | default features; `floors == 0` (all bytes zero — the `{0}` identity image) | [x] |
| 2 | `driver` | default features; `floors` small positive, single non-zero low byte (`1`, `2`, `3`, `7`, `42`, `127`) | [x] |
| 3 | `driver` | default features; `floors` small negative (`-1`, `-2`, `-3`, `-42`, `-128`) — high bytes become `ff` | [x] |
| 4 | `driver` | default features; `floors == INT_MAX` (`0x7fffffff`) upper boundary | [x] |
| 5 | `driver` | default features; `floors == INT_MIN` (`0x80000000`) lower boundary | [x] |
| 6 | `driver` | default features; byte-boundary values that flex each `%02x` field independently: `0x000000ff`, `0x0000ff00`, `0x00ff0000`, `0xff000000`, `0xffffffff` | [x] |
| 7 | `driver` | default features; values with **embedded zero bytes** (`0x00ff00ff`, `0xff00ff00`, `0x00010000`) — catches any string/NUL-termination mistake in the hex formatting | [x] |
| 8 | `driver` | default features; values whose bytes are `< 0x10` (`0x01020304`, `0x0f0f0f0f`) — exercises the zero-**padding** of `%02x` (a `%x` translation would drop a digit here) | [x] |
| 9 | `driver` | default features; values whose bytes are `>= 0x80` (`0x80808080`, `0xdeadbeef`, `0xcafebabe`) — exercises `unsigned char` promotion; a signed-char translation would print `ffffff80`-style garbage | [x] |
| 10 | `driver` | default features; powers of two and one-off neighbours across the whole width (`1<<k`, `(1<<k)-1`, `-(1<<k)` for k = 0..31) | [x] |
| 11 | `driver` | default features; **randomized** full-range `i32` (uniform over all 2^32 bit patterns), 4096 draws, fixed seed | [x] |
| 12 | `driver` | default features; **randomized byte-biased** `i32` (each byte drawn independently from `{0x00,0x01,0x0f,0x10,0x7f,0x80,0xfe,0xff}`), 4096 draws, fixed seed — dense coverage of per-byte formatting edges | [x] |
| 13 | `driver` | default features; **repeated invocation / interleaving**: many `driver` calls in one captured stdout region, alternating C and Rust, asserting each emits exactly 33 bytes and that no state leaks between calls | [x] |
| 14 | `driver` | default features; **output framing**: exactly `2*sizeof(house_t)` hex digits followed by a single `\n`, no leading/trailing whitespace, lowercase hex — asserted on every row above | [x] |
| 15 | `driver` | `--no-default-features` (identical to default, since no `[features]` exist) — full rows 1–14 re-run under that build | [x] |

## Status

All 15 rows pass under every feature combination. Row *n* is implemented by
`tests/differential.rs::phase_b_row_NN_*`; row 15 (feature combinations) is
covered by `./run_differential.sh`, which enumerates the combos from
`Cargo.toml` and re-runs rows 1–14 under each:

```
FEATURE COMBO: default                 -> result: ok. 26 passed; 0 failed
FEATURE COMBO: --no-default-features   -> result: ok. 26 passed; 0 failed
FEATURE COMBO: --all-features          -> result: ok. 26 passed; 0 failed
```

Total distinct inputs driven through both `.so`s: ~10 000 per combination
(4096 uniform-random + 4096 byte-biased-random + 1024 boundary/enumerated +
~1600 repeated/interleaved calls), all with a fixed seed.

Note: `cargo test` does not rebuild a cdylib, so testing without a preceding
`cargo build` compares against a stale `.so`. `assert_artifacts_fresh()` in the
test harness now refuses to run in that state — see `SYMBOLS.md`.
