# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`.

## Axes the C code actually branches on

**Runtime options / modes / flags.** The public header declares exactly one
function and no types, no enums, no setters, no globals:

```c
void driver(float x);
```

`grep -c "extern\|static [a-z_]* [a-z_]* =\|#ifdef" c_src/src/driver.c` finds no
mutable global state and no compile-time feature branch (only the `DRIVER_H_`
include guard). So **the option axis is empty**: there is nothing to configure,
and `driver` is stateless — no init, no teardown, no ordering requirement.

**Full set of public entry points.** `driver` is the only external-linkage
symbol; `print_hex` is `static`. There is no "convenience wrapper vs low-level
entry point" split to worry about — `driver` *is* the lowest level the library
exposes, and it is exercised directly through the `.so` export in every row
below. (`print_hex` is reached transitively through `driver`, which is the only
way C callers can reach it either.)

**Input shapes the code special-cases.** The one branch in the library is the
loop guard `i < len` with `len` fixed at `sizeof(float) == 4`, so control flow is
identical for all inputs. What *does* vary observably is the **object
representation of the `float`** — the four bytes fed to `%02x` — because
`printf("%02x", p[i])` is value-dependent (digit count, zero padding,
zero-extension of the `unsigned char`). The meaningful shape axes are therefore:

* IEEE-754 class of the value: normal, subnormal, zero, infinity, NaN
* sign bit (drives whether the most-significant byte is `>= 0x80`)
* per-byte magnitude: `0x00`, `0x01..0x0f` (needs `%02x` zero padding),
  `0x10..0x7f`, `0x80..0xff` (needs zero-extension, not sign-extension)
* byte position of those magnitudes (little-endian layout: index 0 is the LSB of
  the mantissa, index 3 holds the sign bit and the exponent's high bits)
* call count: one call vs many calls in sequence (interleaving / statelessness)

The cross-product, pruned to combinations the code distinguishes observably:

## Configuration-surface table

| #   | entry point(s) | configuration (options set + input shape)                                                                     | randomized inputs / row | test | [x] |
|-----|----------------|----------------------------------------------------------------------------------------------------------------|-------------------------|------|-----|
| C1  | `driver`       | no options (none exist); positive normal floats, random mantissa+exponent                                       | 4096, seeded            | `b_c1_positive_normals`            | [x] |
| C2  | `driver`       | no options; negative normal floats — sign bit set, so byte 3 is `>= 0x80`                                       | 4096, seeded            | `b_c2_negative_normals`            | [x] |
| C3  | `driver`       | no options; `+0.0` and `-0.0` (all-zero bytes vs. sign-bit-only)                                                | 2 exhaustive            | `b_c3_zeroes`                      | [x] |
| C4  | `driver`       | no options; subnormals of both signs (zero exponent, non-zero mantissa)                                         | 4096, seeded            | `b_c4_subnormals`                  | [x] |
| C5  | `driver`       | no options; `+inf` and `-inf`                                                                                   | 2 exhaustive            | `b_c5_infinities`                  | [x] |
| C6  | `driver`       | no options; quiet NaNs, both signs, random 22-bit payloads                                                      | 4096, seeded            | `b_c6_quiet_nans`                  | [x] |
| C7  | `driver`       | no options; signalling NaNs, both signs, random non-zero payloads                                               | 4096, seeded            | `b_c7_signalling_nans`             | [x] |
| C8  | `driver`       | no options; IEEE-754 extremes: `FLT_MAX`, `FLT_MIN`, `-FLT_MAX`, `-FLT_MIN`, `FLT_EPSILON`, smallest subnormal   | 12 exhaustive           | `b_c8_ieee_extremes`               | [x] |
| C9  | `driver`       | no options; values chosen so that **each** of the 4 bytes in turn is in `0x00..0x0f` — exercises `%02x` padding  | 4 x 256 exhaustive      | `b_c9_zero_padding_each_position`  | [x] |
| C10 | `driver`       | no options; values chosen so that **each** of the 4 bytes in turn is in `0x80..0xff` — exercises zero-extension  | 4 x 128 exhaustive      | `b_c10_high_byte_each_position`    | [x] |
| C11 | `driver`       | no options; every byte value `0x00..0xff` placed at every byte position (full per-position sweep)                | 4 x 256 exhaustive      | `b_c11_full_per_position_sweep`    | [x] |
| C12 | `driver`       | no options; uniformly random raw `u32` bit patterns transmuted to `float` (hits all classes without bias)        | 65536, seeded           | `b_c12_random_bit_patterns`        | [x] |
| C13 | `driver`       | no options; small integer-valued floats (`-1024.0 ..= 1024.0`), the common real-world shape                      | 2049 exhaustive         | `b_c13_integral_values`            | [x] |
| C14 | `driver`       | no options; exponent sweep — `2^e` for every representable `e`, both signs                                       | ~540 exhaustive         | `b_c14_exponent_sweep`             | [x] |
| C15 | `driver`       | no options; **many calls in sequence** in one capture — verifies statelessness and identical line framing         | 1024 per sequence, seeded | `b_c15_repeated_calls_sequence`   | [x] |
| C16 | `driver`       | no options; C and Rust `.so` **both loaded simultaneously** and called alternately on the same `stdout` stream    | 1024 alternations, seeded | `b_c16_interleaved_same_stream`   | [x] |

Every row asserts the captured `stdout` bytes of the C `.so` and the Rust `.so`
are **byte-for-byte identical**, using `dup`/`dup2` capture around each call so
the comparison includes the trailing newline and the exact digit casing.
