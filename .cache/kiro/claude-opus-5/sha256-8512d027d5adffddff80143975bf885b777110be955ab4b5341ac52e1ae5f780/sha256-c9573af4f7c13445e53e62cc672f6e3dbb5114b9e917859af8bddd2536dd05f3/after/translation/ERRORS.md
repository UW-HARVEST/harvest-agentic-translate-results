# ERRORS.md — Phase A: error-surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep result

```sh
grep -nE 'return[[:space:]]+(-1|NULL|[A-Z_]+ERR)|assert|RETURN_ERROR|errno|abort|exit\(|if[[:space:]]*\(|switch|#if|goto|malloc|free' c_src/src/lib.c
# -> 0 matches
```

The whole non-table body of `lib.c` is 9 lines:

```c
float half2float(uint16_t h) {
    union {
        float flt;
        uint32_t num;
    } out;
    int n = h >> 10;
    out.num = m__mantissa[(h & 0x3ff) + m__offset[n]] + m__exponent[n];
    return out.flt;
}
```

Consequently the C code contains:

- **0** error-return macros / `return -1` / `return NULL` / error enums
- **0** `assert`s
- **0** explicit range checks, null checks, or min/max guard constants
- **0** conditionals or `switch`es of any kind (the function is branch-free)
- **0** pointer parameters, **0** enum parameters, **0** length parameters,
  **0** allocations, **0** out-parameters

`half2float` is a total function over `uint16_t`: **every one of the 65536
possible inputs is valid and produces a defined `float`**. There is therefore no
"rejection" behaviour to mirror — the correct Rust behaviour is to *also* never
reject.

That absence is itself the property under test, so the table below has one row
per *latent* / *boundary* condition that a naive translation could plausibly
turn into a rejection (a Rust `panic!` on index-out-of-bounds, an arithmetic
overflow panic, a debug assertion, a truncation difference), plus the generic
FFI boundaries the instructions require. "Expected C result" is what the C
`.so` actually does, observed differentially.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] | test |
|---|----------|----------------------------------------------|-------------------|-----|------|
| 1 | `half2float` | `h = 0x0000` — minimum input; drives `m__mantissa` index to its lower bound `0` (`n=0`, `m__offset[0]=0`, `h&0x3ff=0`) | no error; returns `+0.0f` (bits `0x00000000`). Must NOT panic/abort. | [x] | `row01_row02_min_and_max_input_never_rejected` |
| 2 | `half2float` | `h = 0xFFFF` — maximum input; drives `n` to its upper bound `63` and `m__mantissa` index to its upper bound `2047` (`0x3ff + 0x400`) | no error; returns a negative NaN (bits `0xFFFFE000`). Must NOT panic/abort. | [x] | `row01_row02_min_and_max_input_never_rejected` |
| 3 | `half2float` | `h = 0x03FF` — largest `h` for which `m__offset[n] == 0`, i.e. last index of the *first* half of `m__mantissa` (index `1023`) | no error; returns largest positive subnormal half as `float`. Must NOT panic/abort. | [x] | `row03_row04_offset_transition_positive_side` |
| 4 | `half2float` | `h = 0x0400` — first `h` for which `m__offset[n] == 0x400`, i.e. first index of the *second* half of `m__mantissa` (index `1024`) | no error; returns smallest positive normal half as `float`. Must NOT panic/abort. | [x] | `row03_row04_offset_transition_positive_side` |
| 5 | `half2float` | `h = 0x83FF` — the *second* `m__offset` zero entry (`m__offset[32] == 0`), index `1023` again, reached via the negative half of the exponent table | no error; returns largest negative subnormal. Must NOT panic/abort. | [x] | `row05_row06_offset_transition_negative_side` |
| 6 | `half2float` | `h = 0x8400` — negative-side transition into the second half of `m__mantissa` (index `1024`, `n=33`) | no error; returns smallest negative normal. Must NOT panic/abort. | [x] | `row05_row06_offset_transition_negative_side` |
| 7 | `half2float` | `h = 0x7C00` — `n = 31`, the special `m__exponent[31] = 0x47800000` entry; `uint32_t` sum `0x38000000 + 0x47800000` (largest positive-side sum, potential overflow site) | no error; returns `+Inf` (bits `0x7F800000`). No `uint32_t` wrap; Rust must not panic on overflow. | [x] | `row07_irregular_exponent_31_no_overflow_panic` |
| 8 | `half2float` | `h = 0xFC00` — `n = 63`, the special `m__exponent[63] = 0xC7800000` entry; sum `0x38000000 + 0xC7800000` (largest sum overall, `0xFF800000`) | no error; returns `-Inf` (bits `0xFF800000`). No `uint32_t` wrap; Rust must not panic on overflow. | [x] | `row08_irregular_exponent_63_no_overflow_panic` |
| 9 | `half2float` | `h = 0x7FFF` / `h = 0xFFFF` — maximum sum on each sign: `m__exponent[31/63] + m__mantissa[2047]` (`0x387FE000`), the arithmetically largest addition the code ever performs | no error; returns NaN with bits `0x7FFFE000` / `0xFFFFE000`. Must not panic on overflow. | [x] | `row09_maximum_sums_exact_bits` |
| 10 | `half2float` | Signalling/quiet NaN payload inputs `h = 0x7C01`, `0x7DFF`, `0xFC01`, `0xFDFF` — payload must be carried through the table, not canonicalised | no error; returns NaN whose payload bits are `m__mantissa[...] + 0x47800000`, i.e. the exact bit pattern, not a canonical NaN. Rust must return the identical *bit pattern* (comparing `f32 == f32` would silently pass for any NaN; must compare `to_bits`). | [x] | `row10_nan_payloads_bit_exact_not_canonicalised` (exhaustive over all 2·1023 NaN encodings) |
| 11 | `half2float` | Argument register carries garbage in bits 16..31 (a caller that does not zero-extend the `uint16_t`, e.g. FFI signature declared `int`). C ABI leaves this unspecified. | Whatever the C `.so` does — Rust must do the same. Tested differentially with the argument declared `u32`/`i32` and high bits set. | [x] | `row11_argument_with_garbage_in_high_bits_matches_c` — **FOUND A REAL BUG, see below** |
| 11b | `half2float` | Garbage in bits 32..63 of the argument register (`rdi` vs `edi`) | C reads only the low 32 bits; upper bits irrelevant. Rust must match. | [x] | `row11b_argument_with_garbage_in_upper_64_bits_matches_c` |
| 12 | `half2float` | No pointer, length, enum, or count parameters exist, so the generic "null pointer", "zero length", "oversized length", and "out-of-range enum value" FFI boundaries are **structurally unreachable**. | N/A — verified by inspection of `lib.h`: the sole parameter is a scalar `uint16_t`, and the return is a scalar `float`. No row can be constructed. | [x] | `row12_no_pointer_length_or_enum_parameters_exist` (asserts the header still declares exactly `float half2float(uint16_t h);`, so this premise cannot silently rot) |
| 13 | `half2float` | Called repeatedly / from many threads with interleaved inputs (no internal mutable state to corrupt, but a translation could have introduced some, e.g. a lazily-initialised table) | no error; each call's result depends only on its own argument. Order- and thread-independent. | [x] | `row13_calls_are_pure_and_order_independent`, `row23_concurrent_invocation_is_consistent` |

## Row 11 — the divergence this table actually caught

This is the one place the Rust did **not** match the C, and it is exactly the
class of bug the instructions warn about: an input with no valid interpretation
crossing the FFI boundary.

The original Rust signature was:

```rust
pub extern "C" fn half2float(h: c_ushort) -> c_float
```

With a `c_ushort` parameter LLVM is entitled to assume the caller already
zero-extended the value, so bits 16..31 of the argument register flowed straight
into `h >> 10`. That produced `n > 63`, a `m__mantissa` index far past the end
(observed: `index out of bounds: the len is 2048 but the index is 29299`), and —
because `Cargo.toml` sets `panic = "abort"` for the release profile — a
**`SIGABRT` that killed the whole process**.

The C does not do that. At every optimisation level GCC truncates the argument
to 16 bits before using it:

```text
-O0:  mov %edi,%eax ; mov %ax,-0x14(%rbp) ; movzwl -0x14(%rbp),%eax
-O1:  mov %edi,%eax ; shr $0xa,%ax ; and $0x3ff,%edi ; and $0x3f,%eax
-O2/-O3/-Os:  mov %edi,%eax ; and $0x3ff,%edi ; shr $0xa,%ax
```

`shr $0xa,%ax` is a 16-bit shift, so `n` is always in `0..=63` and the C simply
computes `half2float(arg & 0xFFFF)` for any register contents.

Fix applied to the Rust (`src/lib.rs`): take the wide value and truncate
explicitly, reproducing the C exactly.

```rust
pub extern "C" fn half2float(h: c_uint) -> c_float {
    let h = h as u16;
    ...
}
```

`uint16_t` and `unsigned int` parameters occupy the same argument register on
the SysV x86-64 and AArch64 C ABIs, so this is ABI-compatible for well-behaved
callers and now bit-identical to the C for ill-behaved ones. Verified against
the C built at `-O0`, `-O1`, `-O2`, `-O3` and `-Os`, and with the Rust `.so`
built in both the debug profile (bounds and overflow checks **on**) and the
release profile.

## Notes on rows 7–9 (the one real arithmetic hazard)

The C addition `m__mantissa[...] + m__exponent[n]` is `uint32_t` arithmetic and
would wrap silently. Rust's `+` panics on overflow in debug builds. The maximum
possible sum is `m__mantissa[2047] + m__exponent[63] = 0x387FE000 + 0xC7800000
= 0xFFFFE000`, which does **not** overflow, so C and Rust agree either way; the
Rust translation nevertheless uses `wrapping_add`, which is the exact C
semantics unconditionally. Rows 7–9 pin this down empirically.

## Completion status

All 14 rows have a passing differential test in `tests/differential.rs`
(module `phase_c_error_surface`, plus `row23_...` in `phase_b_...` for row 13).
Verified under both the debug and release Rust profiles and against the C built
at every optimisation level: `./check_features.sh`.
