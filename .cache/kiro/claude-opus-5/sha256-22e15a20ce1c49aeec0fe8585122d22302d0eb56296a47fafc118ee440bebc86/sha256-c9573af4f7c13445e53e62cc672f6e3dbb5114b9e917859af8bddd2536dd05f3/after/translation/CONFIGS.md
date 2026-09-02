# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/driver.c` + `c_src/include/driver.h`, the
mirror of `ERRORS.md` for **valid** inputs.

## Axis enumeration (from the source, not from guesses)

### Axis 1 — entry point (the FULL set of exported entry points)

`nm -D --defined-only c_src/build/libdriver.so` exports exactly two:

* `printHexCharLine(char)` — the **low-level** entry point. It is absent from
  `driver.h` but is non-`static`, so it is dynamically callable and must be
  exercised directly, not just through its caller.
* `driver(char)` — the one-shot **convenience wrapper**; its whole body is
  `char result = data + 1; printHexCharLine(result);`.

### Axis 2 — runtime options / modes / flags

```sh
grep -n "if\|switch\|#ifdef\|#if" c_src/src/driver.c c_src/include/driver.h
```

Zero hits other than the `#ifndef DRIVER_H_` header guard. There are **no**
runtime options, no modes, no flags, no globals, no init/config function, and no
conditional compilation. This axis has exactly one level: *(none)*. So the
cross-product is Axis 1 × Axis 3.

### Axis 3 — input shape the code special-cases

The parameter is a single by-value `char`, so there is no size, width, count,
element type, format, byte order, or emptiness axis. The shape axis is the
*value* axis, and the value sub-ranges the behaviour actually distinguishes are
determined by two ABI facts the C code depends on:

1. On the target ABI (x86-64 Linux) `char` is **signed**. Passing it to the
   variadic `printf` applies the integer promotions, **sign-extending** to `int`.
   `%02x` then reinterprets that `int` as `unsigned int`. Therefore the sign bit
   of the argument selects between two visibly different output widths:
   * `0x00..=0x7F` (non-negative) → 2 hex digits, e.g. `41`
   * `0x80..=0xFF` (negative)     → 8 hex digits, e.g. `ffffff80`
2. `%02x` has a minimum field width of 2, so values `0x00..=0x0F` are
   zero-padded (`00`..`0f`) while `0x10..=0x7F` are not.
3. In `driver`, `data + 1` is computed in promoted `int` and truncated back to
   `char`, so `data == 0x7F` crosses into the negative sub-range and
   `data == 0xFF` wraps to `0x00`. The `+1` therefore **shifts** every boundary
   in axis 3 by one relative to `printHexCharLine`.

## Configuration rows

Rows are the pruned cross-product of Axis 1 × Axis 3 — one row per combination
the C actually treats differently. Every row is driven with **many randomized
inputs** drawn from that row's sub-range using a fixed-seed xorshift PRNG (seed
`0x2025_0901_DEADBEEF`), except the exhaustive rows which enumerate their whole
domain.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| C1 | `printHexCharLine` | no options (none exist); input shape = zero-padded low nibble range `0x00..=0x0F` → output is 2 digits with a leading `0` | `configs_c1_print_low_nibble_padded` | [x] |
| C2 | `printHexCharLine` | no options; input shape = non-negative unpadded range `0x10..=0x7F` → output is exactly 2 digits, no padding | `configs_c2_print_positive_unpadded` | [x] |
| C3 | `printHexCharLine` | no options; input shape = **negative** range `0x80..=0xFF` → sign-extension makes output 8 digits (`ffffff80`..`ffffffff`) | `configs_c3_print_negative_sign_extended` | [x] |
| C4 | `printHexCharLine` | no options; input shape = the four sub-range boundary values `0x00, 0x0F, 0x7F, 0x80, 0xFF` | `configs_c4_print_boundaries` | [x] |
| C5 | `printHexCharLine` | no options; input shape = **exhaustive** over the entire 256-value domain (the whole valid input space, so no value-dependent path can hide) | `configs_c5_print_exhaustive_all_256` | [x] |
| C6 | `driver` | no options; input shape = `0x00..=0x0E` → `result` stays in the padded low-nibble range | `configs_c6_driver_low_nibble_padded` | [x] |
| C7 | `driver` | no options; input shape = `0x0F..=0x7E` → `result` lands in the non-negative unpadded range | `configs_c7_driver_positive_unpadded` | [x] |
| C8 | `driver` | no options; input shape = `0x7F..=0xFE` → `result` lands in the **negative** sign-extended range (this is the `+1` overflow / boundary-shift row) | `configs_c8_driver_negative_sign_extended` | [x] |
| C9 | `driver` | no options; input shape = `0xFF` → `result` **wraps** to `0x00`, output `00` | `configs_c9_driver_wraparound` | [x] |
| C10 | `driver` | no options; input shape = **exhaustive** over the entire 256-value domain | `configs_c10_driver_exhaustive_all_256` | [x] |
| C11 | `driver` **composed with** `printHexCharLine` | no options; asserts the composed pipeline identity — `driver(v)` output equals `printHexCharLine(v+1 truncated)` output — checked across randomized `v` on **both** libraries, so a divergence in the wrapper's arithmetic cannot hide behind a correct low-level function | `configs_c11_composition_identity` | [x] |
| C12 | both, interleaved | no options; input shape = a long randomized call sequence alternating entry points, all output captured in one stream → exercises stdout buffering/ordering of the composed pipeline rather than one call in isolation | `configs_c12_interleaved_stream` | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so there is exactly one
feature combination (default == `--no-default-features`). All rows above are run
under it, in **both** the debug and release profiles, by
`translation/run_all_features.sh`.

## Divergence found and fixed

Verification found one real behavioural divergence. It is recorded here because
it is a *valid-input* ABI issue, not an error path.

**Symptom.** `printHexCharLine(0x100)` printed `00` from the C `.so` and `100`
from the Rust `.so`. Caught by the Phase C row
`errors_out_of_range_int_passed_as_char_arg`, which passes full-width `int`
values through the ABI-compatible widened signature `void f(int)`.

**Root cause (read from the machine code, C is ground truth).** GCC compiles
`void printHexCharLine(char charHex)` to

```text
mov    %edi,%eax
mov    %al,-0x4(%rbp)      ; spill only the LOW BYTE  -> truncation
movsbl -0x4(%rbp),%eax     ; reload sign-extended     -> promotion
```

so the callee **re-narrows** the incoming argument register to 8 bits. The Rust
`extern "C" fn(c_char)` compiled to a bare `mov %edi,%esi`, because the SysV
AMD64 ABI permits the callee to assume the caller already narrowed. For any
caller that leaves non-zero high bytes in `%edi` the two libraries disagreed.
`driver` was already correct — it compiled to `inc %dil; movsbl %dil,%esi`, which
only ever touches the low byte, matching GCC's `mov %al` / `movzbl` pair.

**Fix (`src/lib.rs`).** Declare both exports as taking `c_int` — ABI-identical to
the `char` declaration in `driver.h`, since the argument travels in the same
register — and truncate explicitly:

```rust
let narrowed: c_char = charHex as c_char;   // reproduces `mov %al`
let promoted:  c_int = narrowed as c_int;   // reproduces `movsbl`
```

`printHexCharLine` now compiles to `movsbl %dil,%esi`, the exact net effect of
GCC's three instructions. Behaviour for a well-behaved caller passing a real
`char` is unchanged.

## Suite sensitivity

`translation/mutation_check.sh` compiles seven deliberately wrong variants of the
C source in `/tmp` (`c_src` is never modified) and points the suite at each one.
All seven are detected:

| mutant | wrong behaviour | rows that failed |
|--------|-----------------|------------------|
| m1 | `data + 2` instead of `data + 1` | 12 |
| m2 | `%02X` instead of `%02x` | 16 |
| m3 | parameter is `unsigned char` (no sign-extension) | 12 |
| m4 | parameter is `int` (no narrowing) — the bug fixed above | 1 |
| m5 | `%x` instead of `%02x` (no field width) | 13 |
| m6 | newline dropped from the format string | 17 |
| m7 | `data - 1` instead of `data + 1` | 12 |

m4 is detected by exactly one row —
`errors_out_of_range_int_passed_as_char_arg` — which is why that row is
load-bearing rather than decorative.

