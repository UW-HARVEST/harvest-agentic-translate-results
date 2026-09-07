# ERRORS.md — Phase C error-surface table

## Mechanical derivation

Every error/rejection construct was grepped for across the whole C source
(`c_src/src/lib.c`, `c_src/include/lib.h`). Result counts:

| grepped construct | occurrences |
|---|---|
| `RETURN_ERROR` | 0 |
| `return -1` | 0 |
| `return NULL` | 0 |
| `return 0;` | 0 |
| `assert` | 0 |
| `errno` | 0 |
| `enum` | 0 |
| `#if` / `#ifdef` | 0 |
| `switch` | 0 |
| `malloc` / `calloc` / `free` | 0 |
| `MIN` / `MAX` / `LIMIT` constants | 0 |
| pointer declarations (`*`) | 0 (the only `*` is float multiplication) |
| `if (…)` | 1 (`if (High < Low)` — a swap, not a rejection) |
| ternary `?:` | 3 (the per-channel transfer function) |

**The C API has no error-return channel at all.** `contrast_ratio` returns a
bare `float`, takes both arguments **by value**, and cannot fail: there is no
sentinel, no status code, no out-parameter, no allocation, and no pointer to
dereference. `unsigned char` channels make every one of the 2^24 values per
color a *valid* input, so there is no "out-of-range" value the C rejects.

Consequently the rejection surface is not made of error codes but of the
**degenerate IEEE-754 results** the C produces without guarding, plus the
**generic FFI boundary conditions** the prompt requires be covered anyway. Each
row below asserts C and Rust agree on the *exact* bit pattern / sentinel, not
merely that "both did something odd".

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | ✅ |
|---|----------|----------------------------------------------|-------------------|----|
| 1 | `contrast_ratio` | `B` = pure black `(0,0,0)`, `A` non-black → `Low == 0.0f`, `High > 0`; unguarded `High / Low` | `+INFINITY` (exact bits `0x7F800000`) | [x] |
| 2 | `contrast_ratio` | `A` = pure black `(0,0,0)`, `B` non-black → swap taken (`High=LumB`, `Low=LumA==0`) | `+INFINITY` (exact bits `0x7F800000`) | [x] |
| 3 | `contrast_ratio` | BOTH `A` and `B` pure black → `0.0f / 0.0f`, no guard | `NaN` (both sides NaN; `is_nan()` on each) | [x] |
| 4 | `contrast_ratio` | `A == B` (any equal colors, non-black) → `High == Low`, `if (High < Low)` false, no swap | exactly `1.0f` | [x] |
| 5 | `contrast_ratio` | `A == B == (0,0,0)` — the equal-colors path *and* the 0/0 path at once | `NaN`, **not** `1.0f` (C does not special-case equality) | [x] |
| 6 | `contrast_ratio` | Near-zero-but-nonzero `Low`: darkest non-black color `(1,1,1)` vs white — largest finite ratio | finite, no overflow to inf; exact bit match | [x] |
| 7 | `contrast_ratio` | Channel exactly at the transfer threshold, low side: byte `10` (`10/255 = 0.039215… <= 0.04045`) → **linear** branch `C/12.92` | linear branch taken; exact bit match | [x] |
| 8 | `contrast_ratio` | Channel one step past the threshold: byte `11` (`11/255 = 0.043137… > 0.04045`) → **pow** branch | pow branch taken; exact bit match | [x] |
| 9 | `contrast_ratio` | Channel `0` — minimum of the valid range, `0/12.92 == 0.0f` | linear branch, contributes `0.0` | [x] |
| 10 | `contrast_ratio` | Channel `255` — maximum of the valid range, `pow(1.0, 2.4) == 1.0` | pow branch, contributes full weight | [x] |
| 11 | `contrast_ratio` | **Out-of-range value across the FFI boundary**: the 3-byte struct is passed in one SysV INTEGER register; caller sets the 5 unused high bytes to `0xFF…`/garbage (the `enum`-with-no-valid-variant analogue for this ABI). Called as `extern "C" fn(u64, u64) -> f32`. | C ignores the padding bits and returns the same value as the clean call; Rust must ignore them identically | [x] |
| 12 | `contrast_ratio` | Same as #11 but with garbage high bits on the **second** argument register only | identical to clean call on both sides | [x] |
| 13 | `contrast_ratio` | Argument registers loaded with garbage high bits on **both** arguments, over randomized colors | identical to clean call on both sides, all inputs | [x] |
| 14 | `contrast_ratio` | Null pointer / zero length / oversized length | **NOT APPLICABLE, verified by grep**: the API takes no pointer and no length. Documented here so the boundary class is explicitly accounted for rather than silently skipped. | [x] |
| 15 | `contrast_ratio` | Sign of the result / negative zero: no input can make `Low` negative (all channels ≥ 0), so `-0.0` and negative ratios are unreachable | ratio is never negative; asserted over the exhaustive sweep | [x] |
