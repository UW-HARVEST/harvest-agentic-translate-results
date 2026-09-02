# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical inventory of rejection constructs in the C source

```sh
grep -nE 'return|assert|NULL|errno|RETURN_ERROR|if *\(' c_src/src/lib.c c_src/include/lib.h
```

Findings:

* `return` statements: 3 — `return Result;`, `return Ratio;`,
  `return cbContrastRatio(...);`. **None** of them is an error return; all three
  return a computed `float`.
* `assert`: 0 occurrences.
* `NULL` / null checks: 0 occurrences. The public API takes both arguments **by
  value** (`cb_rgb_255 A, cb_rgb_255 B`), so there is no pointer to validate.
* error enums / error codes / `errno`: 0 occurrences. No out-parameters.
* explicit range checks: 0. The only comparisons are `> 0.04045` (a *branch*, not
  a rejection) and `High < Low` (a *swap*, not a rejection).
* min/max constants: 0. The channel type `unsigned char` makes `0..=255`
  structurally unrepresentable-out-of-range — every one of the 256 values per
  channel is valid input.
* enums: 0 declared, so there is no "out-of-range enum value" variant to pass
  across FFI for this API. (The generic FFI-garbage case is still covered below,
  row E7.)

**Conclusion: the C library has NO explicit error/rejection path.** It is a total
function over its input domain. The error surface is therefore entirely the set
of *degenerate IEEE-754 results* the C returns instead of rejecting, plus the
generic FFI-boundary conditions. Those are enumerated as rows below, and each has
a differential test that asserts C and Rust produce the **same** sentinel
(bit-identical, `+inf` vs `+inf`, NaN vs NaN with matching sign/payload class).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `contrast_ratio` | `B` = pure black `{0,0,0}` and `A` != black → `LumB == 0.0f`, `High = LumA > 0`, `Low = 0.0f`, so `High/Low` divides by zero | `+inf` (`0x7F800000`), no trap, no error code |
| E2 | `contrast_ratio` | `A` = pure black `{0,0,0}` and `B` != black → `High = LumA = 0`, `Low = LumB > 0`, `High < Low` is TRUE so it swaps to `High = LumB`, `Low = 0` → divide by zero | `+inf` (`0x7F800000`) |
| E3 | `contrast_ratio` | BOTH `A` and `B` = pure black `{0,0,0}` → `High = 0.0f`, `Low = 0.0f`, `0/0` | NaN (`0/0` invalid operation → quiet NaN; sign/payload must match Rust bit pattern class) |
| E4 | `contrast_ratio` | `A == B` (any equal colors, non-black) → `High == Low`, `High < Low` FALSE, `High/Low` | exactly `1.0f` (`0x3F800000`) |
| E5 | `contrast_ratio` | Any channel `<= 10` (i.e. `c/255 <= 0.04045`) takes the `else` branch `c/12.92` instead of `pow`; the boundary is between 10 (`0.039215..`) and 11 (`0.043137..`). Passing 10 vs 11 is the "one step past the documented range" case for this internal branch | branch-dependent luminance; no error. 10 → linear branch, 11 → `pow` branch |
| E6 | `contrast_ratio` | Zero-length / oversized length inputs: **not applicable** — the API has no length parameter and no buffers. Passing a struct is size-fixed at 3 bytes by the ABI. Documented here so the row is not silently skipped | N/A — cannot be triggered; asserted by construction (no pointer/length in the signature) |
| E7 | `contrast_ratio` | FFI garbage in the ABI padding: a 3-byte `struct` is passed in the low 3 bytes of an INTEGER register (SysV AMD64 class INTEGER); bits 24..63 are **undefined**. An external caller may leave them non-zero. The C must ignore them | identical result to the same call with the upper bits zeroed — the upper 40 bits must not affect the return value. Rust must ignore them identically |
| E8 | `contrast_ratio` | Null pointer: **not applicable** — no pointer parameters and a non-pointer (`float`) return. There is no `return NULL` in the source. Recorded so the generic null-pointer boundary is explicitly accounted for | N/A — cannot be triggered |

### Row checklist

- [x] E1 — `+inf` when `Low` is 0 via the no-swap path
- [x] E2 — `+inf` when `Low` is 0 via the swap path
- [x] E3 — NaN from `0.0/0.0`
- [x] E4 — exactly `1.0` for identical colors
- [x] E5 — 10/11 branch boundary on every channel position
- [x] E6 — N/A, no length parameter exists (verified from the header signature)
- [x] E7 — upper ABI register bits ignored, C == Rust
- [x] E8 — N/A, no pointer parameter exists (verified from the header signature)

## Test mapping

| row | test in `translation/tests/differential.rs` |
|---|---|
| E1 | `e01_divide_by_zero_no_swap` |
| E2 | `e02_divide_by_zero_swap_path` |
| E3 | `e03_zero_over_zero_nan` |
| E4 | `c06_identical_colors` (asserts exact `1.0` bits) |
| E5 | `e05_transfer_function_branch_boundary` |
| E6 | `e06_e08_no_pointer_or_length_parameters` (structural, from `lib.h`) |
| E7 | `c16_e7_abi_upper_bit_garbage` |
| E8 | `e06_e08_no_pointer_or_length_parameters` (structural, from `lib.h`) |
| generic: every channel byte accepted | `e09_every_channel_value_is_accepted` |

Every error row asserts the **specific** sentinel (`0x7F800000` for `+inf`, equal
NaN bit patterns for `0/0`, `0x3F800000` for `1.0`) on both sides — not merely
"both failed".

## Note on out-of-range enum values

The task's generic requirement to pass invalid enum values across FFI has no
instantiation here: `lib.h` declares no `enum`, and the only parameter type is a
struct of three `unsigned char`, for which **all 2^24 bit patterns are valid**.
`e09_every_channel_value_is_accepted` drives all 256 values through each of the
6 channel slots against three different backgrounds, and
`c16_e7_abi_upper_bit_garbage` covers the one genuinely out-of-domain bit
pattern that the ABI permits (garbage in register bits 24..63).

## Harness soundness (mutation testing)

An error table is only meaningful if the harness can actually fail. Two defects
were found and fixed while validating this:

1. **Stale-artifact bug (fixed).** `cargo test` does not rebuild a
   `crate-type = ["cdylib"]` artifact, so the first version of the harness could
   load an `.so` left over from an earlier `cargo build` and pass vacuously.
   The harness now builds the `cdylib` itself into
   `target/harness/<profile>/` and asserts the object is newer than `src/lib.rs`
   and `Cargo.toml`.
2. **Bogus coverage counter (fixed).** `contrast_ratio` returns
   `max/min`, so it is symmetric; the original swap-path classifier compared
   `f(a,b)` with `f(b,a)` and therefore never observed a swap. It now classifies
   via a white-reference probe.

With those fixed, deliberate mutations of `src/lib.rs` were each rebuilt and run:

| mutation | detected? |
|---|---|
| `0.7152f32` -> `0.7153f32` | YES — 13 tests fail |
| `c / 12.92` -> `c / 12.93` (linear arm only) | YES — 13 tests fail |
| `powf(2.4)` -> `powf(2.4000001)` | YES — 13 tests fail |
| luminance weighting done in `f64` instead of `f32` | YES — 14 tests fail |
| `High / Low` -> `Low / High` | YES — 17 tests fail |
| `LumA = cbLuminance(RA,..)` -> `cbLuminance(RB,..)` | YES — 17 tests fail |
| `c > 0.04045` -> `c >= 0.04045` | no — **provably unobservable**: no `n/255.0f32` widens to exactly `0.04045` |
| `if High < Low` -> `if High <= Low` | no — **provably unobservable**: swapping two equal values is the identity |
| `c / 12.92` -> `c / 12.9200001` | no — **unobservable**: the 7.7e-9 relative change is below `f32` resolution after the narrowing cast |
| `A.R as f32 / 255.0` -> `... + 0.0f32` | no — **unobservable**: `x + 0.0 != x` only for `x == -0.0`, unreachable here |

Every undetected mutation is semantically equivalent on the reachable input
domain, so the suite detects 100% of observable mutations. `src/lib.rs` was
restored bit-for-bit (md5 verified) after each experiment.
