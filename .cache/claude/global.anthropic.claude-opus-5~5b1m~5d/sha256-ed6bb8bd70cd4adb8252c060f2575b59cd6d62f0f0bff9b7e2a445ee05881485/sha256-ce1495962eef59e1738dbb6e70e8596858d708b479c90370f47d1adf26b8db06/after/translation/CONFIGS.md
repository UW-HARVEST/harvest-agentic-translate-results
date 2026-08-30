# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the C source and the public header.

## Axis enumeration (from the source, not from guesses)

**Runtime options / modes / flags:** none. `c_src/include/driver.h` exposes a
single declaration, `void driver(int x)`. There is no init function, no context
struct, no setter, no global, no environment-variable read, and no `#ifdef` in
`src/driver.c`. So the "options" axis is a singleton.

**Public entry points (the FULL set, including the lowest level):**

| entry point | linkage | in ABI? |
|---|---|---|
| `driver(int)` | external | yes — tested directly via `dlsym` |
| `print_hex(unsigned char*, int)` | `static` (internal) | no — unreachable from outside; its behaviour is covered transitively, and it is *correct* that it is not exported (see `SYMBOLS.md`) |

So the lowest-level ABI entry point and the only entry point coincide: `driver`.

**Input shapes the code special-cases:** `driver` has one by-value `int`. The
code does not branch on its value at all, so the meaningful shapes are the
*byte-pattern classes* of the 4 bytes that `memcpy` copies and `%02x` formats —
these are where value-dependent bugs (endianness, sign extension, zero padding)
actually live:

- byte order (which of the 4 bytes lands at index 0) — endianness
- bytes `< 0x10` — must be zero-padded to two hex digits by `%02x`
- bytes `>= 0x80` — must not sign-extend through the `char`→`int` promotion
- sign bit of `x` set vs. clear
- extremes: `0`, `-1`, `INT_MIN`, `INT_MAX`
- fixed length: `len` is always `sizeof(int) == 4`, so exactly 4 loop iterations

## Configuration-surface table

One row per combination the C actually distinguishes. Every row is driven with
**many randomized inputs (fixed seed)** plus its named boundary values, and both
`.so`s are compared byte-for-byte on captured `stdout`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `driver` | `x == 0` — all four bytes zero; forces `%02x` zero-padding on every byte | `boundary_extremes` | [x] |
| 2 | `driver` | `x == -1` (`0xFFFFFFFF`) — all bytes `0xff`, max sign-extension pressure | `boundary_extremes` | [x] |
| 3 | `driver` | `x == INT_MAX` (`0x7FFFFFFF`) — sign bit clear, all other bits set | `boundary_extremes` | [x] |
| 4 | `driver` | `x == INT_MIN` (`0x80000000`) — sign bit set, all other bits clear | `boundary_extremes` | [x] |
| 5 | `driver` | `x == 1` — smallest positive; byte 0 is `01`, bytes 1..3 are `00` (endianness discriminator) | `endianness_discriminators` | [x] |
| 6 | `driver` | `x == 0x01020304` — all four bytes distinct and ordered; pins byte order exactly | `endianness_discriminators` | [x] |
| 7 | `driver` | one byte `0x80`..`0xff` and the rest small — isolates signed-`char` sign extension per byte position | `high_bytes_no_sign_extension` | [x] |
| 8 | `driver` | every byte `< 0x10` (e.g. `0x01020304`, `0x0f0e0d0c`) — zero-padding in all 4 positions | `low_nibble_zero_padding` | [x] |
| 9 | `driver` | single bit set, swept across all 32 bit positions | `single_bit_sweep` | [x] |
| 10 | `driver` | all 256 byte values placed in each of the 4 byte positions (1024 cases) — exhaustive per-position byte coverage | `all_byte_values_each_position` | [x] |
| 11 | `driver` | uniformly random `i32`, 4000 seeded samples over the full range | `randomized_full_range` | [x] |
| 12 | `driver` | repeated / consecutive calls in one process — checks output framing (one `\n`-terminated line per call) and that no state leaks between calls | `repeated_calls_framing` | [x] |
| 13 | `driver` | interleaved C-then-Rust and Rust-then-C call ordering on the shared libc `stdout` — checks the translation does not depend on stream state or ordering | `interleaved_call_order` | [x] |

## Feature combinations

`Cargo.toml` declares no `[features]`, so the cross-product of features is the
single default configuration. The test runner script enumerates the feature list
from `Cargo.toml` programmatically and confirms this rather than assuming it.

---

## Verification results

Run with `./run_tests.sh` (which builds the C `.so`, builds the Rust cdylib,
diffs symbols, then runs the differential suite for every feature combination in
both `debug` and `release`).

All 13 rows pass, in both profiles, across randomized inputs:

- 15 tests pass (12 differential + 3 symbol-parity) per configuration.
- Default suite performs ~5,400 differential `driver` comparisons.
- `extended_randomized_sweep` (`cargo test -- --ignored`) performs a further
  **210,000** comparisons (200k seeded random + dense walks at both range ends);
  all byte-identical.

### Harness-integrity note (important)

`cargo test` does **not** build the `cdylib` for this crate (`crate-type =
["cdylib"]`). Two harness defects were found and fixed while validating that the
tests can actually fail:

1. **Stale artifact.** The path search silently fell back to an older
   `libdriver.so`, so edits to `src/lib.rs` were not being tested at all.
   `assert_so_is_fresh` now aborts if the `.so` predates any `src/**/*.rs`.
2. **Non-restored fd 1.** A panic inside the capture window left fd 1 pointing
   at the temp file, discarding every later message (including the panic report
   that explained the failure). The redirect is now restored via `Drop`, and
   library loading happens before the redirect.

### Mutation testing (proof the suite has detection power)

Each deliberate bug was injected into `src/lib.rs`, rebuilt, and the suite re-run:

| mutation | result |
|---|---|
| `to_ne_bytes` -> `to_be_bytes` (byte order) | 12 tests FAILED |
| `%02x` -> `%x` (lost zero-padding) | 12 tests FAILED |
| removed the trailing `printf("\n")` | 12 tests FAILED |
| printed 3 bytes instead of `sizeof(int)` | 12 tests FAILED |

`src/lib.rs` was restored to its original contents afterwards (verified).
