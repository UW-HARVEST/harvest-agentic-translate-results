# ERRORS.md — Phase A: error / rejection surface table

## How this was derived (mechanically)

```
$ grep -n 'return\|assert\|if\|NULL\|ERROR\|errno\|<\|>' c_src/src/*.c c_src/include/*.h
```

Findings, exhaustively:

* **No** `assert` anywhere (`grep -c assert c_src -r` -> 0).
* **No** `NULL` comparison anywhere (`grep -c NULL c_src -r` -> 0).
* **No** error enum, no `errno`, no `RETURN_ERROR`-style macro, no `-1` /
  `NULL` sentinel return. `grep -c 'return -1\|return NULL' c_src -r` -> 0.
* **No** `#if` / `#ifdef` conditional code.
* The only named constant is `#define N_SMOOTH 16` (a divisor and a window
  width, not a limit that is validated).
* `match` has **exactly one** `if` statement, and it is the library's only
  explicit rejection path: `return 0`.
* Everything else that "rejects" input does so implicitly, through loop
  guards (`i < length`), unguarded division, and unguarded pointer arithmetic.

So the error surface is: **one explicit rejection**, **one implicit rejection**
(the final ordered comparison), and a set of unchecked-boundary behaviours that
the C exhibits rather than diagnoses. Every one of those is a row below,
because each is a real input an external caller can supply across the FFI
boundary, and the Rust must behave identically.

`float_t` note that governs several rows: `match.c` sees `typedef double
float_t` from `match.h`; `spectral_contrast.c` includes only `<math.h>` and
never `match.h`, so there `float_t` is glibc's `typedef float float_t`
(verified on this host: `sizeof(float_t) == 4`, `FLT_EVAL_METHOD == 0`).
`spectral_contrast` therefore indexes its arguments as `float`, including when
`match` calls it with `double` buffers. That is the ground truth.

## The table

Legend for the last column: `[x]` = a differential test exists in
`tests/errors.rs` and **passes** against the canonical C `.so`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `match` | `total(test,bins) < threshold * total(reference,bins)` — the energy gate at `match.c:37`, the library's only explicit `return 0`. Reached with `test` energy strictly below `threshold *` `reference` energy. | returns `0` (int), **before** any preprocessing; `test`/`reference` untouched. Asserted to be exactly `0`, not merely "equal". | `e1_energy_gate_returns_zero` | [x] |
| E2 | `match` | gate operand is NaN, so the `<` is unordered: `total(test,bins)` is NaN, or `threshold` is NaN, or the product is NaN. `comisd -0x78(%rbp),%xmm0` + `jbe` ⇒ unordered sets CF=ZF=1 ⇒ the branch IS taken ⇒ the `return 0` is **skipped**. | gate does **not** reject; falls through to preprocessing; then `contrast >= NaN` is false ⇒ final result `0` | `e2_gate_unordered_does_not_reject` | [x] |
| E3 | `match` | `threshold * total(reference,bins)` = `±inf * ±0` (invalid op) ⇒ product is the x86 indefinite QNaN `0xFFF8000000000000` | gate unordered ⇒ falls through (same as E2) | `e3_gate_inf_times_zero` | [x] |
| E4 | `match` | final `spectral_contrast(t,r,bins) >= threshold` is false or unordered (contrast NaN, or `threshold` NaN). `comisd -0x70(%rbp),%xmm3` + `setae`: unordered ⇒ CF=1 ⇒ `0`. | returns `0` | `e4_final_comparison_rejects` | [x] |
| E5 | `match` | `bins == 0`. Both VLAs are zero-length and land at `%rsp`; `total` loops 0×; `memcpy(v,src,0)` is a no-op; `smoothen` loops 0×; then **`differentiate` executes its unguarded trailing store as `v[-1] = 0`** (`match.c:25`) — i.e. it zeroes `%rsp - 8`, **the slot holding `preprocess`'s return address into `match`**. | **SIGSEGV, always**, for every `threshold` (the gate can never fire first: it needs `0.0 < threshold*0.0`, which is `0.0` or a QNaN). **MEASURED, not assumed:** standalone C driver exits 139. Rust guards the store and returns instead — there is no C value to match. | `e5_match_bins_zero_crashes_in_c` | [x] |
| E6 | `spectral_contrast` | `length == 0` | all three loops run 0×; `sqrt` never called; returns **exactly `+0.0`** (`0x0000000000000000`); both arrays untouched | `e6_spectral_length_zero` | [x] |
| E7 | `spectral_contrast` | `length < 0` (`-1`, `-2`, `-3`, `-16`, `-17`, `-1000`, `-65536`, `INT_MIN`, `INT_MIN+1`) — no validation; every loop guard `i < length` is false at `i = 0` | returns **exactly `+0.0`**; both arrays untouched; **no** crash | `e7_spectral_negative_length` | [x] |
| E8 | `match` | `bins < 0` — no validation. Gate passes (both totals are `0.0`), then `preprocess` calls `memcpy(v, source, length * sizeof(*v))` where the negative `int` product converts to a **~2^64 `size_t`**. | **SIGSEGV.** UB in the C; asserted as a crash (signal 11) in a child process, and asserted that Rust does not crash. | `e8_match_negative_bins_crashes_in_c` | [x] |
| E9 | `spectral_contrast` / `normalize` | zero magnitude: `dot_product(v,v,length) == 0` ⇒ `magnitude == 0.0` ⇒ `v[i] /= 0.0` with **no guard** (`spectral_contrast.c:13`). Reached by an all-`+0.0`/`-0.0` array, by subnormals whose squares underflow to `0`, and through `match` on all-zero spectra. | `0.0/0.0` ⇒ f32 indefinite QNaN `0xFFC00000` (asserted exactly); `x/0.0`, `x != 0` ⇒ `±inf`. `dot_product` of those ⇒ NaN. No crash. | `e9_zero_magnitude_division` | [x] |
| E10 | `spectral_contrast` / `normalize` | `sqrt` of a non-finite magnitude. `a[i]*a[i] >= 0` so `sqrt` never sees a negative finite value, but the f32 reinterpretation readily produces NaN and `+inf` sums. | `sqrt(NaN)` ⇒ quieted NaN, `sqrt(+inf)` ⇒ `+inf` ⇒ `x/inf` ⇒ `±0`; propagates; no crash | `e10_sqrt_of_nan_magnitude` | [x] |
| E11 | `spectral_contrast`, `match` | operand bytes decode to an **f32 signalling NaN** (`0x7F800001`, `0x7FBFFFFF`, `0xFF800001`, `0xFFBFFFFF`, `0xFFA00001`, …), and f64 sNaNs (`0x7FF0000000000001`, …). No check; SSE exceptions are masked. | the sNaN is quieted by `mulss` / `cvtss2sd` / `divsd` and propagated with the payload rules encoded in `src/lib.rs`; no trap | `e11_signalling_nan_inputs` | [x] |
| E12 | `spectral_contrast`, `match` | fully aliased arguments `spectral_contrast(v, v, length)` / `match(v, v, bins, th)` — permitted, there is no `restrict`, so `normalize` runs **twice** on the same buffer | second `normalize` divides the already-normalized data by ≈`1.0`; buffer mutated twice; must match byte-for-byte | `e12_fully_aliased_arguments` | [x] |
| E13 | `spectral_contrast` | partially overlapping arguments: `b = a + k` and `a = b + k` for `k` ∈ {1,2,3,5} | well-defined in C: `normalize(a)` completes before `normalize(b)` starts, so the overlap is observable and must match byte-for-byte | `e13_partially_overlapping_arguments` | [x] |
| E14 | `match`, `spectral_contrast` | **null pointers.** `spectral_contrast(NULL,NULL,len)` for `len <= 0` never dereferences. `spectral_contrast(NULL,NULL,len)` for `len > 0`, and `match(NULL,NULL,bins,th)` for **any** `bins` (including `0`, via E5), do. | `spectral_contrast(NULL,NULL,len<=0)` ⇒ exactly `+0.0`, both. `spectral_contrast(NULL,NULL,len>0)` ⇒ SIGSEGV in **both**, asserted equal. `match(NULL,NULL,bins,th)` ⇒ SIGSEGV in C for `bins` ∈ {`-1`,`0`,`1`,`16`}; for `bins >= 1` Rust faults identically, asserted equal. | `e14_null_pointers` | [x] |
| E15 | both | out-of-range value across the FFI boundary. **The API declares no `enum`**, so the exhaustive integer surface is `int bins` / `int length`; the whole `int` range is accepted without validation. Boundaries: `INT_MIN`, `INT_MIN+1`, `-2`, `-1`, `0`, `1`, `INT_MAX`, and a stack-exhausting `4_000_000`. | `spectral_contrast`: `INT_MIN..=0` ⇒ exactly `+0.0` (both); `1` ⇒ values compared over 50 random inputs; `INT_MAX` ⇒ both fault, asserted equal. `match`: `1` ⇒ compared over 50 random inputs; `INT_MIN`/`INT_MIN+1`/`-1`/`0` ⇒ SIGSEGV in C (asserted); `4_000_000` ⇒ C exhausts the 8 MB stack with its two 32 MB VLAs and dies (asserted), Rust heap-allocates and survives (documented divergence on an input C cannot handle). | `e15_integer_boundaries` | [x] |
| E16 | `match` | `bins == 1`: `differentiate`'s loop body never runs (`1 - 1 == 0`), so the trailing `v[0] = 0` makes **both** preprocessed buffers all-zero ⇒ E9. `spectral_contrast` then reads **1 f32** out of the 2 f32 slots of the single `double`. | contrast is NaN ⇒ `>= threshold` false ⇒ returns `0` for **every** threshold; asserted to be exactly `0` as well as equal, across all 21 threshold classes and 200 random raw-bit inputs | `e16_match_bins_one_zeroed_by_differentiate` | [x] |
| E17 | both | `bins` / `length` odd, so `spectral_contrast` reads `bins` f32 slots = `bins/2` whole doubles plus the low half of one more — never past the end of the `bins`-double buffer, since `bins <= 2*bins`. This is the exact boundary the `float_t` confusion sits on. | in bounds; no crash; value and both buffers must match | `e17_odd_bins_f32_view_boundary` | [x] |
| E18 | `smoothen` | window clamped at the array end (`i + j < length`) while the divisor stays `N_SMOOTH` = 16, unguarded — the last `min(15, length-1)` outputs are attenuated by `(length-i)/16`. For `length < 16` **every** element is attenuated. | attenuated tail, bit-exact. **Correction to a natural but wrong assumption:** this means a *constant* spectrum does NOT preprocess to all zeros (e.g. `[7.5, 7.5]` → `[0.9375, 0.46875]` → `[-0.46875, 0]` → `[-0.029296875, 0]`), so it does *not* reach the zero-magnitude path; only an all-zero spectrum does. | `e18_smoothen_attenuated_tail` | [x] |

## Non-rows (checked and confirmed absent)

`grep` found no: allocation-failure path (no `malloc`), no integer-overflow
check, no length maximum, no alignment check, no `restrict` qualifier, no
`const` qualifier, no return-code enum, no output parameter, no logging. The
library validates nothing. Rows E5–E17 are precisely the boundaries a caller
can hit *because* nothing is validated.
