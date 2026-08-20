# ERRORS.md — Phase A: error-surface table

Derived **mechanically** from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Mechanical grep result (the anti-blind-spot step)

```
grep -nE 'return (-1|NULL|[A-Z_]+_ERROR)|RETURN_ERROR|assert|errno|abort|exit\(|goto|perror|E[A-Z]+' \
     c_src/src/lib.c c_src/include/lib.h
=> (NONE)
```

* error-return macros (`RETURN_ERROR`, …): **none**
* `return -1` / `return NULL` / error enums: **none** — all 5 `return`
  statements (lines 19, 25, 32, 45, 59) return a fully-formed value struct
* `assert`: **none**
* `errno` / `abort` / `exit` / `goto` / `perror`: **none**
* explicit range checks / null checks: **none**
* memory allocation (thus allocation failure): **none**
* pointer parameters in the *public* API: **none** (`tritanopia` takes and
  returns `cb_rgb_255` **by value**). The only pointers in the file are the
  three `float*` of `static void Tritanopia`, always called at line 57 with
  the addresses of live stack locals.

**This library has no error/rejection return path at all.** It is a total
function: every one of the 2^24 possible inputs produces a value.

Consequently the real "error surface" — the set of ways the C code resolves
*out-of-domain / implementation-defined / boundary* conditions, which is
exactly the class of behaviour a translation silently gets wrong — is the
three `(unsigned char)` narrowing casts and the six strict-`>` thresholds.
Those are enumerated below, one row per distinct condition, together with the
generic FFI boundaries that every C API has (documented as N/A **with
justification** where the API makes them unreachable).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✓ |
|---|----------|----------------------------------------------|-------------------|------|---|
| E1 | `cbDenorm` (lib.c:29, R) | narrowing cast `(unsigned char)v` where `v < 0` (UB / impl-defined). Reachable: post-matrix R is negative for **1 666 521** of 2^24 inputs; denorm arg reaches **-419.2283** (min). e.g. `(0,0,255)` | gcc x86-64 emits `cvttss2si`→`i32` then keeps the low byte: `trunc(-419.2283) = -419`, `-419 & 0xff = 93` → `R=93` | `err_e1_denorm_negative_wraps` | [x] |
| E2 | `cbDenorm` (lib.c:29, R) | narrowing cast `(unsigned char)v` where `v > 255` (UB / impl-defined). Reachable: **191 482** of 2^24 inputs; denorm arg reaches **269.2830** (max). e.g. `(255,255,0)` | `trunc(269.2830) = 269`, `269 & 0xff = 13` → `R=13` (wraps mod 256, does **not** clamp/saturate) | `err_e2_denorm_over_255_wraps` | [x] |
| E3 | `cbDenorm` (lib.c:29–31) | in-range cast `0 <= v <= 255`: C truncates **toward zero**, and the `+ 0.5f` makes it round-half-up. A `round()`/saturating cast would differ | `(unsigned char)trunc(v)` | `err_e3_denorm_in_range_truncates` | [x] |
| E4 | `cbDenorm` (lib.c:30, G) | upper boundary: G denorm arg attains exactly **255.5** (max, 1280 inputs). `trunc` keeps it legal — a *rounding* cast would overflow to 256 → 0 | `trunc(255.5) = 255` → `G=255` (not 0) | `err_e4_e5_g_b_upper_boundary` | [x] |
| E5 | `cbDenorm` (lib.c:31, B) | upper boundary: B denorm arg attains exactly **255.5** (max, 1280 inputs), same as E4 | `trunc(255.5) = 255` → `B=255` | `err_e4_e5_g_b_upper_boundary` | [x] |
| E6 | `cbDenorm` (lib.c:29–31) | cast of **NaN** → `cvttss2si` yields the "integer indefinite" value `0x80000000`, low byte `0`. **Unreachable from the public API** (proved: `pow` never receives a negative base, see E9; NaN count over all 2^24 inputs = 0 for all three channels) | would be `0`; must stay unreachable — no input may produce NaN in either impl | `err_e6_nan_unreachable` | [x] |
| E7 | `cbRemoveGammaRGB` (lib.c:13/15/17) | threshold is **strict** `>` `0.04045`, evaluated in `double`. Boundary: `u8=10` → `10/255 = 0.0392156877` → **NOT** `> 0.04045` → linear branch `c/12.92`; `u8=11` → `0.0431372561` → `>` → `pow` branch. Using `>=` or `float` compare would flip a channel | `10` → linear, `11` → `pow`; per-channel and independent | `err_e7_remove_gamma_threshold` | [x] |
| E8 | `cbApplyGammaRGB` (lib.c:36/39/42) | threshold is **strict** `>` `0.00313080495356037151702786377709`. Values `<=` it (incl. **all negative** values) take the linear `c*12.92` branch. Reachable: linear branch taken for **1 796 020** (R) / **88 320** (G) / **88 320** (B) of 2^24 | `<=` → `c*12.92` (may be negative → feeds E1); `>` → `1.055*pow(c,0.4166666666)-0.055` | `err_e8_apply_gamma_threshold` | [x] |
| E9 | `cbApplyGammaRGB` (lib.c:37/40/43) | `pow(base, 0.4166666666)` with `base < 0` would return **NaN**. **Unreachable**: the strict `>` of E8 guarantees `base > 0.0031308 > 0`. A translation that computed `pow` unconditionally (or used `abs`) would diverge | `pow` is never called with a negative base | `err_e9_pow_never_negative_base` | [x] |
| E10 | `tritanopia` | **null pointer** argument — **N/A, unreachable**: the public signature is `cb_rgb_255 tritanopia(cb_rgb_255)`, by value. There is no pointer, array, buffer, handle or context parameter anywhere in `include/lib.h`, so no null check exists or can be reached | n/a (no pointer parameter exists) | `err_e10_e12_no_pointer_or_length_parameters` | [x] |
| E11 | `tritanopia` | **out-of-range enum value across FFI** — **N/A, unreachable**: the API declares **no enum type**. `grep -c enum c_src` = 0. The only parameter type is a struct of three `unsigned char` | n/a (no enum exists) | `err_e11_out_of_range_register_patterns` | [x] |
| E12 | `tritanopia` | **zero / oversized length** — **N/A, unreachable**: there is no length, size, count, stride or capacity parameter, and no array/pointer input, so there is no length to be zero or oversized | n/a (no length parameter exists) | `err_e10_e12_no_pointer_or_length_parameters` | [x] |
| E13 | `tritanopia` | **"invalid" field value** — **N/A by construction**: the field type `unsigned char` is exactly `0..=255`, so *every* one of the 256^3 bit patterns is a valid input. There is no value "one step past a documented valid range" to test — the range is the full type | every input is valid; all 2^24 must match | `err_e13_every_bit_pattern_is_valid` + `row26_exhaustive_all_16m_inputs` | [x] |
| E14 | `tritanopia` (ABI) | 3-byte struct is passed in the **low 3 bytes of `RDI`**; bytes 3..7 of the register are **unspecified garbage**. Callee must ignore them. Passing `0xDEADBE_<rgb>` must give the same answer as passing `0x000000_<rgb>` | result depends only on the low 3 bytes; C and Rust must agree | `err_e14_upper_arg_register_garbage` | [x] |
| E15 | `tritanopia` (ABI) | 3-byte struct is returned in the **low 3 bytes of `RAX`**; bytes 3..7 are unspecified. A caller must not depend on them (so the differential assert compares only 3 bytes) | only the low 3 bytes are meaningful | `err_e15_return_low_three_bytes_only` | [x] |

**All 15 rows have a passing differential test.** 12 rows (E1–E9, E13–E15)
test a directly reachable condition. The 3 rows E10–E12 are
**unreachable-by-signature** (no pointer, no enum, no length parameter exists),
so they are justified structurally rather than guessed at, and are still
covered by tests that assert the by-value ABI and that arbitrary out-of-range
register payloads are ignored. E6 and E9 are tested as *negative* properties
(the condition must never arise), which is the only correct way to test an
unreachable branch.

## Note on UB (E1/E2) — why the Rust must replicate, not "fix", it

`(unsigned char)v` for `v` outside `0..255` is undefined behaviour in C, but
the C code is the ground truth and is compiled by a real compiler, so it has a
concrete observable behaviour that ~11% of all inputs depend on
(1 666 521 + 191 482 = **1 857 995** of 16 777 216 inputs, i.e. 11.07%, hit
E1 or E2 on the R channel). Rust's `as` cast **saturates**, which would give
`0` instead of `93` and `255` instead of `13`. `src/lib.rs` therefore
implements `f32_to_uchar` to reproduce `cvttss2si` + low-byte truncation. This
is correct and must not be "simplified" to `as u8`.

---

## Verification results (final)

All 15 rows verified. Phase C tests live in `tests/phase_c_errors.rs` and are
run by `./run_all.sh` in every configuration:

```
running 13 tests
test err_e1_denorm_negative_wraps ... ok          # E1
test err_e2_denorm_over_255_wraps ... ok          # E2
test err_e3_denorm_in_range_truncates ... ok      # E3
test err_e4_e5_g_b_upper_boundary ... ok          # E4 + E5
test err_e6_nan_unreachable ... ok                # E6
test err_e7_remove_gamma_threshold ... ok         # E7
test err_e8_apply_gamma_threshold ... ok          # E8
test err_e9_pow_never_negative_base ... ok        # E9
test err_e10_e12_no_pointer_or_length_parameters ... ok   # E10 + E12
test err_e11_out_of_range_register_patterns ... ok        # E11
test err_e13_every_bit_pattern_is_valid ... ok    # E13
test err_e14_upper_arg_register_garbage ... ok    # E14
test err_e15_return_low_three_bytes_only ... ok   # E15
test result: ok. 13 passed; 0 failed
```

Each test asserts the **same concrete value** from both `.so`s, and where a
plausible mistranslation would give a specific wrong answer the test also
asserts the result is *not* that value, so it cannot pass vacuously:

| row | C's answer | wrong answer explicitly excluded |
|---|---|---|
| E1 | `R = 93` for `(0,0,255)` | `0` (Rust's saturating `as u8`), `255` |
| E2 | `R = 13` for `(255,255,0)` | `255` (saturation) |
| E4/E5 | `G = B = 255` at the `255.5` boundary | `0` (overflow from a rounding cast) |
| E9 | the linear-branch wraparound `trunc(d) & 0xff` | `0` (the `pow(negative) = NaN` signature) |

### A note on E9's discriminator

The first version of the E9 test asserted "R must never be 0" for
negative-base inputs. That was **wrong**: a *slightly* negative value (e.g.
denorm argument `-0.4`) legitimately truncates to `0`, so the test failed on 18
of 5184 legitimate cases. It was corrected to restrict attention to strongly
negative arguments (`< -1.0`) and to compare against the exact wraparound
prediction `trunc(d) & 0xff`. The C was never changed — the *test's* premise
was at fault.

### Mutation-testing evidence

`./mutation_test.sh` proves these error-path tests are not vacuous. Injecting
a saturating cast (the single most likely mistranslation of E1/E2) is caught by
18 tests; changing `trunc` to `round` is caught by 28. Conversely, changing the
unreachable NaN sentinel (E6) correctly survives, which independently confirms
the "unreachable" claim in that row.
