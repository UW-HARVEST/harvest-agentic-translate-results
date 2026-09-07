# CONFIGS.md — Phase A configuration-surface table

Mirror of `ERRORS.md` for **valid** inputs. Axes derived mechanically from what
the C actually branches on.

## Axes enumerated from the C source

1. **Runtime options / modes / flags:** *none.* The public header exposes one
   function taking one scalar; there is no init function, no context struct, no
   setter, no global, no environment variable read, and no `#ifdef` other than
   the header include guard (see the grep table in `ERRORS.md`). So this axis
   has exactly one value: "no options".
2. **Public entry points:** the full set is `{ driver }` — this *is* the
   lowest-level public entry point; there is no convenience wrapper layer. The
   only lower-level function, `print_hex`, is `static` and deliberately not
   exported by either `.so` (see `SYMBOLS.md`), so it cannot be — and must not
   be — driven directly.
3. **Input shapes the code special-cases:**
   - *byte value class per object byte*: `0x00` (prints `00`), `0x01..0x0f`
     (`%02x` must zero-pad to two digits), `0x10..0x7f`, `0x80..0xff` (must not
     sign-extend to 8 digits).
   - *byte position* 0,1,2,3 within the `int` — combined with host **byte
     order**, this is what decides the digit order in the output.
   - *count*: `len` is fixed at `sizeof(int)` = 4, so the loop-trip-count axis
     has a single value; the "empty / one / many" axis instead applies to the
     number of successive `driver` calls sharing the stdout buffer.
   - *sign / magnitude* of `x`: zero, small positive, large positive,
     `INT_MAX`, small negative, large negative, `INT_MIN`.

## Configuration table (cross-product, pruned to what the C distinguishes)

| #   | entry point(s)     | configuration (options set + input shape)                                                                             | [x] |
|-----|--------------------|-----------------------------------------------------------------------------------------------------------------------|-----|
| C1  | `driver`           | no options; `x = 0` — all four object bytes `0x00`                                                                     | [x] |
| C2  | `driver`           | no options; `x = 1` — low byte `0x01` needs `%02x` zero-padding, other three bytes `0x00`                              | [x] |
| C3  | `driver`           | no options; `x = 0x01020304` — four distinct ascending bytes, pins host **byte order** in the output                   | [x] |
| C4  | `driver`           | no options; `x = 0x04030201` — reversed byte order of C3, confirms the order is data-driven not hard-coded             | [x] |
| C5  | `driver`           | no options; every byte in `0x01..0x0f` (`0x0f0e0d0c`) — zero-padding in **all four** positions simultaneously          | [x] |
| C6  | `driver`           | no options; every byte in `0x80..0xff` (`0x80818283`, `0xffffffff`) — high-bit set in all positions, no sign extension  | [x] |
| C7  | `driver`           | no options; exactly one byte high-bit-set, swept across all 4 positions (`0x80`,`0x8000`,`0x800000`,`0x80000000`)      | [x] |
| C8  | `driver`           | no options; exactly one byte `< 0x10` and non-zero, swept across all 4 positions (`0x01`,`0x0100`,`0x010000`,`0x01000000`) | [x] |
| C9  | `driver`           | no options; `x = INT_MAX`, `INT_MAX-1`, `INT_MIN`, `INT_MIN+1` — value-range boundaries                                 | [x] |
| C10 | `driver`           | no options; exhaustive sweep of the low byte `0x00..0xff` with upper bytes zero (all 256 per-byte `%02x` renderings)    | [x] |
| C11 | `driver`           | no options; exhaustive sweep of one byte `0x00..0xff` in **each** of the 4 positions (1024 cases)                       | [x] |
| C12 | `driver`           | no options; randomized full-range `i32`, seeded PRNG, 20000 iterations                                                  | [x] |
| C13 | `driver`           | no options; randomized small positive `0..=255`, seeded, 2000 iterations                                                | [x] |
| C14 | `driver`           | no options; randomized negative `i32::MIN..0`, seeded, 2000 iterations                                                  | [x] |
| C15 | `driver`           | no options; randomized values whose bytes are drawn only from `{0x00,0x0f,0x80,0xff}`, seeded, 2000 iterations          | [x] |
| C16 | `driver`           | no options; **many** successive calls in a single stdout capture (2000 randomized values, one capture) — buffering, line separation, no extra bytes | [x] |
| C17 | `driver`           | no options; **one** call in a capture (baseline for C16: exactly `2*sizeof(int)` digits + `\n`, nothing else)            | [x] |
| C18 | `driver`           | no options; interleaved C-then-Rust and Rust-then-C call order in one capture — both `.so`s share the glibc stdout `FILE`, so ordering/flush behaviour must be identical | [x] |
| C19 | `driver`           | no options; return-value/ABI shape — `void` return, no registers or errno clobbered across the call (call twice, compare) | [x] |

Rows C10–C15 are property-style with a fixed seed (`SEED = 0x5EED_1234_ABCD_EF01`)
so failures reproduce exactly.

## Feature combinations

`Cargo.toml` declares no `[features]`, so the cross-product of feature
combinations is the single default build; all rows above are verified under it,
and `scripts/check_features.sh` confirms `--no-default-features` and
`--all-features` resolve to that same build.
