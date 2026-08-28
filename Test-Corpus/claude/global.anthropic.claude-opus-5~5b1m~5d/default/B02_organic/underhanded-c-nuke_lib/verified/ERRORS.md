# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/`. The exhaustive greps:

```
$ grep -n "return"  c_src/src/*.c c_src/include/*.h
spectral_contrast.c:7 :    return sum;
spectral_contrast.c:19:    return dot_product(a, b, length);
match.c:8             :    return sum;
match.c:37            :    if(total(test, bins) < threshold * total(reference, bins)) return 0;
match.c:40            :    return spectral_contrast(t, r, bins) >= threshold;

$ grep -n "assert\|NULL\|errno\|ERROR\|#if\|#ifdef\|switch" c_src/src/*.c c_src/include/*.h
(no matches)

$ grep -n "if\s*(" c_src/src/*.c c_src/include/*.h
match.c:37            :    if(total(test, bins) < threshold * total(reference, bins)) return 0;

$ grep -n "#define" c_src/src/*.c c_src/include/*.h
match.h:1             : #define N_SMOOTH 16
```

So: **there is exactly one `if` in the whole library, no `assert`, no `NULL`
check, no range check, no error enum, no `errno` use, and no sentinel return.**
The library never *reports* an error. Its entire rejection surface is
(a) the one semantic `return 0` gate in `match`, and (b) implicit
domain/undefined-behaviour boundaries. Both classes are enumerated below; every
row has a differential test in `tests/errors.rs`.

Legend for "expected C result": `= v` means it returns `v`; `SIGSEGV` means the
built `.so` deterministically crashes (verified by running it in a child
process, exit status 139).

| #  | function | trigger (exact invalid input / condition) | expected C result |
|----|----------|-------------------------------------------|-------------------|
| E1  | `match` | `total(test,bins) < threshold * total(reference,bins)` — the one explicit rejection (`match.c:37`). Energy gate fails. | `= 0`, **before** any preprocessing (short-circuit) |
| E2  | `match` | E1 with `threshold` so large the product overflows to `+inf` (`total(ref) > 0`, `threshold = DBL_MAX`) | `= 0` |
| E3  | `match` | E1 where `threshold * total(reference,bins)` is `NaN` (e.g. `threshold = +inf`, `total(reference) = 0`) → `<` is an *ordered* comparison so it is **false**: the gate does **not** reject | falls through to the contrast test; `comisd`/`setae` with a NaN operand ⇒ `= 0` |
| E4  | `match` | `threshold = NaN` (any data) | gate false ⇒ falls through; final `contrast >= NaN` false ⇒ `= 0` |
| E5  | `match` | `total(test,bins)` is `NaN` (test contains a NaN element) | gate `NaN < x` false ⇒ falls through, no early `return 0` |
| E6  | `match` | second rejection: `spectral_contrast(t,r,bins) >= threshold` is false (`match.c:40`) — contrast below cut-off, or contrast is `NaN` | `= 0` |
| E7  | `match` | contrast is `NaN` because a preprocessed buffer normalises to `0/0` (all-equal input ⇒ `differentiate` zeroes everything ⇒ magnitude `0`) | `= 0` |
| E8  | `match` | `bins == 0`. `float_t t[0], r[0]` are zero-length VLAs; `differentiate(v,0)` executes the unguarded `v[length-1] = 0` ⇒ `v[-1] = 0`, which at the VLA's address is exactly `preprocess`'s saved return address ⇒ `ret` to `0x0`. | **SIGSEGV** (verified `-O0` *and* `-O2`) |
| E9  | `match` | `bins < 0` (`-1`, `-2`, `-5`, `INT_MIN`). Gate is false (both totals are `0`), then `preprocess` calls `memcpy(v, source, (size_t)(bins*8))` with a wrapped-around huge size. | **SIGSEGV** (verified `-O0` *and* `-O2`) |
| E10 | `match` | `test == NULL` (with `bins >= 1`) | **SIGSEGV** (`total` dereferences it immediately) |
| E11 | `match` | `reference == NULL` (with `bins >= 1`) | **SIGSEGV** |
| E12 | `match` | `bins` larger than the caller's buffers (out-of-range length, e.g. buffer of 8, `bins = 1<<20`) | out-of-bounds read; no check exists. Undefined; not asserted, only that neither library *checks* |
| E13 | `match` | `bins` so large the two VLAs exceed the stack (`bins = 1<<28` ⇒ 2 × 2 GiB) | **SIGSEGV** (stack overflow) |
| E14 | `spectral_contrast` | `length == 0` | every loop body is skipped; `dot_product` returns its initial `sum` ⇒ `= +0.0` (bits `0x0000000000000000`), buffers untouched |
| E15 | `spectral_contrast` | `length < 0` (`-1`, `-7`, `INT_MIN`) — loop guards are `i < length` with `i` starting at `0`, so still no iterations | `= +0.0`, buffers untouched (**no** crash — differs from `match`, which has the VLA/`memcpy`) |
| E16 | `spectral_contrast` | `a == NULL`, `length >= 1` | **SIGSEGV** |
| E17 | `spectral_contrast` | `b == NULL`, `length >= 1` | **SIGSEGV** |
| E18 | `spectral_contrast` | `a == NULL`, `b == NULL`, `length <= 0` | no dereference ⇒ `= +0.0` (must **not** crash) |
| E19 | `spectral_contrast` | zero magnitude: `a` all `+0.0` ⇒ `normalize` divides by `sqrt(0) == 0` ⇒ `0.0/0.0` | `a[i]` becomes the x86 default QNaN `0xFFC00000` (sign **set**); return `0xFFF8000000000000` |
| E20 | `spectral_contrast` | zero magnitude on `b` only (`a` non-zero) | `b[i] = ±inf`/QNaN per element sign; return is `NaN` or `±inf` — must match bit-for-bit |
| E21 | `spectral_contrast` | `a` all `-0.0` (magnitude `sqrt(+0)` is `+0`, elements are `-0.0`) ⇒ `-0.0/0.0` | `= 0xFFC00000` per element (QNaN), same as E19 by IEEE |
| E22 | `spectral_contrast` | non-zero but tiny magnitude: all elements subnormal ⇒ `dot_product` underflows to `+0.0` ⇒ division by zero again | `±inf` elements, return `NaN`/`inf` — bit-exact |
| E23 | `spectral_contrast` | overflow: elements `~3.4e38` ⇒ `a[i]*a[i]` overflows f32 to `+inf` ⇒ magnitude `+inf` ⇒ `x/inf == 0` | elements become `±0.0`, return `+0.0`/`-0.0`/`NaN` — bit-exact |
| E24 | `spectral_contrast` | `a` contains `+inf` / `-inf` | magnitude `+inf`, `inf/inf` ⇒ QNaN `0xFFC00000`; finite/`inf` ⇒ `±0.0` |
| E25 | `spectral_contrast` | `a` contains a **quiet** NaN | NaN is *quieted-through*: element keeps its payload; return carries a NaN payload determined by the SSE destination-operand rules (`mulss dst = b[i]`, `addsd dst = product`) |
| E26 | `spectral_contrast` | `a` contains a **signaling** NaN (`0x7F800001`, `0x7FA00001`) | the sNaN is quieted in place (mantissa MSB forced on) by the `divsd` in `normalize`; no trap |
| E27 | `spectral_contrast` | **both** `a[i]` and `b[i]` are NaN with different payloads (the operand-order-sensitive case) | `mulss` destination is `b[i]` ⇒ **`b`'s** payload survives (this is `-O0`-specific; `-O2` picks `a`'s) |
| E28 | `spectral_contrast` | NaN accumulated into `sum`, then a *second* NaN product | `addsd`'s destination is the *product* ⇒ the **later** product's payload survives |
| E29 | `spectral_contrast` | aliasing `a == b` | `normalize` runs **twice** on the same buffer; second run divides by the new (≈1) magnitude. Well-defined in C; must be reproduced (Rust must not create two `&mut`) |
| E30 | `spectral_contrast` | partial overlap, `b == a + 1` | well-defined in C; must be reproduced |
| E31 | `match` | aliasing `test == reference` | `t` and `r` are distinct VLAs, so no aliasing inside; result should be `1` for well-conditioned data |
| E32 | `match` | `bins == 1` — `differentiate` zeroes the single element, `spectral_contrast` then reads the **low 4 bytes** of that `0.0` double ⇒ `0.0f` ⇒ magnitude 0 ⇒ `0/0` | contrast `NaN` ⇒ `= 0` for every `threshold` |
| E33 | `match`, `spectral_contrast` | `threshold`/`length` "one past the valid range": `threshold = -0.0`, `+0.0`, `DBL_MIN`, `-DBL_MAX`; `length = INT_MAX`, `INT_MIN`, `INT_MIN + 1` | no range check exists; the safe subset (`length <= 0`) must return `+0.0` from `spectral_contrast`, and `threshold` extremes must agree bit-for-bit |
| E34 | (enum surface) | The C API declares **no enum, no flags and no mode parameter** — the only non-pointer parameters are `int bins`/`int length` and `double threshold`, so "out-of-range enum value" degenerates to out-of-range `int`/`double`, covered by E8/E9/E13/E14/E15/E33. | — |

## Crashing rows

E8–E13, E16, E17 (plus `length = INT_MAX` from E33) make the *C* library die
with `SIGSEGV`. They are therefore exercised out-of-process: `ub_crash_matrix`
re-invokes the test binary (`std::process::Command` on `current_exe()`) with
`DIFF_CASE`/`DIFF_LIB` set, so the `#[ignore]`d `ub_crash_child` helper performs
exactly one call against exactly one `.so`, and the parent inspects the child's
`ExitStatus` (`.signal()` / `.code()` / the printed `RESULT` line).

For every null-pointer and oversized-length row the assertion is that **both**
libraries fault. For `match` with `bins <= 0` (E8/E9) the C faults on undefined
behaviour while the Rust returns `0`; that divergence is recorded and explained
rather than "fixed" by making Rust fault too — deliberately faulting would be
strictly worse for every caller and would make the library unusable. See the
recorded matrix at the bottom of this file.

## Row → test mapping (all rows checked off)

| row | test in `tests/errors.rs` | [x] |
|-----|---------------------------|-----|
| E1  | `e1_energy_gate_rejects` | [x] |
| E2  | `e2_energy_gate_product_overflows` | [x] |
| E3  | `e3_gate_product_is_nan_falls_through` | [x] |
| E4  | `e4_threshold_nan` | [x] |
| E5  | `e5_total_test_is_nan` | [x] |
| E6  | `e6_contrast_cutoff_rejects` | [x] |
| E7  | `e7_zero_magnitude_gives_nan_contrast` | [x] |
| E8  | `ub_crash_matrix` / case `e8_match_bins_zero` | [x] |
| E9  | `ub_crash_matrix` / cases `e9_match_bins_neg1`, `…neg2`, `…neg5`, `…int_min` | [x] |
| E10 | `ub_crash_matrix` / case `e10_match_null_test` | [x] |
| E11 | `ub_crash_matrix` / case `e11_match_null_reference` | [x] |
| E12 | `ub_crash_matrix` / case `e12_match_bins_past_buffer` | [x] |
| E13 | `ub_crash_matrix` / case `e13_match_bins_huge` | [x] |
| E14 | `e14_sc_length_zero` | [x] |
| E15 | `e15_sc_negative_length` | [x] |
| E16 | `ub_crash_matrix` / case `e16_sc_null_a` | [x] |
| E17 | `ub_crash_matrix` / case `e17_sc_null_b` | [x] |
| E18 | `e18_sc_null_pointers_with_nonpositive_length` | [x] |
| E19 | `e19_sc_zero_magnitude_on_a` | [x] |
| E20 | `e20_sc_zero_magnitude_on_b_only` | [x] |
| E21 | `e21_sc_negative_zero_elements` | [x] |
| E22 | `e22_sc_subnormal_underflowing_magnitude` | [x] |
| E23 | `e23_sc_overflowing_magnitude` | [x] |
| E24 | `e24_sc_infinities` | [x] |
| E25 | `e25_sc_quiet_nan_passthrough` | [x] |
| E26 | `e26_sc_signalling_nan_is_quieted` | [x] |
| E27 | `e27_sc_nan_payload_precedence_in_mulss` | [x] |
| E28 | `e28_sc_nan_payload_precedence_in_addsd` | [x] |
| E29 | `e29_sc_fully_aliased` | [x] |
| E30 | `e30_sc_partial_overlap` | [x] |
| E31 | `e31_match_aliased_arguments` | [x] |
| E32 | `e32_match_bins_one_always_zero` | [x] |
| E33 | `e33_boundary_scalars` + `ub_crash_matrix` / case `e33_sc_length_int_max` | [x] |
| E34 | `e34_no_enum_surface_int_domain` | [x] |

## Observed UB / crash matrix (recorded by `ub_crash_matrix`)

```
e8_match_bins_zero                 C=Signal(11)  Rust=Returned("0")
e9_match_bins_neg1                 C=Signal(11)  Rust=Returned("0")
e9_match_bins_neg2                 C=Signal(11)  Rust=Returned("0")
e9_match_bins_neg5                 C=Signal(11)  Rust=Returned("0")
e9_match_bins_int_min              C=Signal(11)  Rust=Returned("0")
e10_match_null_test                C=Signal(11)  Rust=Signal(11)
e11_match_null_reference           C=Signal(11)  Rust=Signal(11)
e12_match_bins_past_buffer         C=Signal(11)  Rust=Signal(11)
e13_match_bins_huge                C=Signal(11)  Rust=Signal(11)
e16_sc_null_a                      C=Signal(11)  Rust=Signal(11)
e17_sc_null_b                      C=Signal(11)  Rust=Signal(11)
e33_sc_length_int_max              C=Signal(11)  Rust=Signal(11)
```

Every null-pointer / oversized-length row faults on both sides. The only
recorded divergence is `match` with `bins <= 0` (E8/E9): the C `.so` faults on
undefined behaviour, the Rust returns `0`. Deliberately faulting in Rust to
"match" would be strictly worse for every caller and would make the library
unusable, so the behaviour is documented here instead of imitated. No in-process
differential row uses `bins <= 0`, since the C cannot survive it.
