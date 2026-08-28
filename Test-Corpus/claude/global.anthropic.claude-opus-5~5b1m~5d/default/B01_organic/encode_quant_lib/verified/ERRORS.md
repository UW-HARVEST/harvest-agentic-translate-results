# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c` (62 lines) and `c_src/include/lib.h`.

## Mechanical grep of every rejection mechanism

| pattern grepped | count in C source | where |
|---|---|---|
| `return` statements | **1** | line 61 `return (uni);` |
| `RETURN_ERROR` / error macros | 0 | — |
| `return -1` / `return NULL` / negative sentinels | 0 | — |
| `assert` / `static_assert` | 0 | — |
| error `enum`s / status codes / `typedef`s | 0 | — |
| `errno` / `exit` / `abort` | 0 | — |
| pointer parameters, null checks | 0 (all 6 params are `int` by value; the 3 `*` hits are multiplications on lines 30/36/42) | — |
| explicit range/bounds checks | 0 | — |
| length/size parameters | 0 | — |
| `MIN`/`MAX` constants | 0 | — |
| `#if` / `#ifdef` gates | 0 | — |

**Conclusion: `encode_quant` has NO error-return path.** It is a pure,
total `int(int,int,int,int,int,int)` function with a single exit. Every one of
the 2^192 possible argument tuples is "accepted" and produces an `int`. There is
no invalid input that the C rejects, so there is no error code or sentinel to
match — the differential obligation for every row below is therefore
**"both libraries return the exact same `int`, and neither traps/aborts"**.

The 10 `if` conditions in the C are all *data-dependent branches*, not
rejections (they are covered by `CONFIGS.md`):

| line | condition | kind |
|---|---|---|
| 8  | `(uni ^ uni1) & (~7)` | branch (candidate clamp) |
| 10 | `(uni ^ uni2) & (~7)` | branch (candidate clamp) |
| 12 | `lsbit` | branch (mode select) |
| 13 | `lsbit == 4` | branch (mode select) |
| 20 | `lsbit & 1` | branch (mode select) |
| 31 | `uni & 8` | branch (sign of `diff`) |
| 37 | `uni1 & 8` | branch (sign of `diff`) |
| 43 | `uni2 & 8` | branch (sign of `diff`) |
| 57 | `d1 < d0` | branch (candidate selection) |
| 59 | `d2 < d0` | branch (candidate selection) |

## Error-surface table

Because the C declares no rejections, the rows below enumerate the *implicit*
rejection surface every C API of this shape still has: the generic FFI
boundaries mandated by Phase C (null pointers, zero/oversized lengths, one-step
-past-range values, and out-of-range enum values crossing the FFI boundary), plus
every place the C performs an operation that is undefined/trapping in C or
panicking in Rust and where the two could therefore diverge instead of agreeing.
"Expected C result" is the ground truth the Rust must reproduce byte-identically.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `encode_quant` | **Out-of-range "enum" value for the `lsbit` mode selector**: `lsbit` is used as a 3-way mode switch (`0` / `4` / odd / other-even) but is typed `int`, so any `int` is a real input. `lsbit = 2, 3, 5, 6, 7, 8, 9, 100, 12345` — values with no "documented" variant. | No rejection: `lsbit==4` → dither branch; else odd → set bit0; else → clear bit0. Returns a normal `int`. Must match exactly. |
| 2 | `encode_quant` | **Negative out-of-range enum values** for `lsbit`: `-1, -2, -3, -4, -5, -8, -100`. `lsbit & 1` on a negative `int` relies on two's complement (`-1 & 1 == 1`, `-2 & 1 == 0`). | No rejection: negative-odd → set-bit0 branch, negative-even → clear-bit0 branch. Must match exactly. |
| 3 | `encode_quant` | **Extreme enum values** for `lsbit`: `INT_MIN` (even → clear branch), `INT_MAX` (odd → set branch), `INT_MIN+1`, `INT_MAX-1`, and `4` one step away on both sides (`3`, `5`). | No rejection; branch chosen purely by the `==4` / `&1` tests. Must match exactly. |
| 4 | `encode_quant` | **Signed integer overflow, `uni + 1` (line 6)** with `uni == INT_MAX` — UB in C. | C (gcc, no `-fwrapv`) wraps to `INT_MIN`; the line-8 guard then detects the changed high bits and restores `uni1 = uni`. Rust must use wrapping and produce the same `int`, not panic. |
| 5 | `encode_quant` | **Signed integer overflow, `uni - 1` (line 7)** with `uni == INT_MIN` — UB in C. | Wraps to `INT_MAX`; line-10 guard restores `uni2 = uni`. Rust must wrap, not panic. |
| 6 | `encode_quant` | **Signed overflow in `(2 * (uni & 7) + 1) * step` (lines 30/36/42)**: multiplier is `1,3,5,7,9,11,13,15`; any `step > INT_MAX/15` overflows, e.g. `step = INT_MAX`, `step = 0x1000_0000`, `step = INT_MIN`. | Wraps (two's complement) before the `/ 8`. Rust must wrap, not panic. |
| 7 | `encode_quant` | **`diff = -diff` (lines 32/38/44) with `diff == INT_MIN`** — would be UB in C. **Proven UNREACHABLE:** the multiplier `2*(uni&7)+1` is odd, so the wrapped product `P` ranges over all of `int`, but `diff = P / 8` is then bounded to `[-2^28, 2^28-1]`. `INT_MIN` is outside that range, so the negation can never overflow. Reachable extremes are `diff = -2^28` and `diff = 2^28-1`. | N/A by construction — no negation overflow exists in either library. The test instead drives `diff` to both reachable extremes (`±2^28`) with `uni & 8` set so the negation executes, and asserts C/Rust agree. |
| 8 | `encode_quant` | **Signed overflow in `pred + diff` (lines 33/39/45)**: `pred = INT_MAX, diff > 0` or `pred = INT_MIN, diff < 0`. | Wraps. Rust must wrap, not panic. |
| 9 | `encode_quant` | **Signed overflow in `tgt - p` / `tgt2 - p` (lines 34/40/46/48/51/54)**: e.g. `tgt = INT_MAX`, `p = INT_MIN`. | Wraps. Rust must wrap, not panic. |
| 10 | `encode_quant` | **Signed overflow in `d0 += d3 >> 5` (lines 50/53/56)**: `d0` near `INT_MAX` plus a large `d3 >> 5`. | Wraps, so a "distortion" can become negative and flip the line-57/59 comparisons. Rust must wrap and select the same candidate. |
| 11 | `encode_quant` | **Right shift of a negative value, `d >> 31` (lines 35/41/47/49/52/55)** — implementation-defined in C. | gcc emits an *arithmetic* shift → `-1` for negatives, so `d ^ (d >> 31)` is the branchless absolute value (`INT_MIN` maps to `INT_MAX`). Rust `i32 >> ` is arithmetic; must match. |
| 12 | `encode_quant` | **Right shift of a negative value, `d3 >> 5` (lines 50/53/56)** — implementation-defined in C. (`d3` is non-negative after row 11's abs, so this exercises the non-negative path; the negative path is exercised by `(uni >> 1)`/`(uni >> 2)` below.) | Arithmetic shift, rounding toward −∞ for negatives. Must match. |
| 13 | `encode_quant` | **Right shift of a negative value in the `lsbit == 4` dither, `(uni >> 1) & (uni >> 2) & 1` (lines 17–19)** with `uni`/`uni1`/`uni2` negative (e.g. `uni = -1`, `INT_MIN`, `-7`). | Arithmetic shift; `-1 >> 1 == -1` so the OR-ed bit is 1 for `uni = -1`. Must match. |
| 14 | `encode_quant` | **`step == 0`** (degenerate "zero length"): all three `diff` values become `0`. | No rejection; all three candidates collapse to `p = pred`, so `d1 == d0`, `d2 == d0`, both `<` tests are false, and the (possibly lsbit-modified) `uni` is returned unchanged. |
| 15 | `encode_quant` | **Negative `step`** (a "length" that a real API would reject): `step = -1, -8, -1000, INT_MIN`. | No rejection; `diff` becomes negative and the `uni & 8` sign flip inverts, `/ 8` truncates toward zero for negative numerators. Must match exactly. |
| 16 | `encode_quant` | **Oversized `step`** (`INT_MAX`, `INT_MAX-1`, `0x7FFF_FFF8`): overflow per row 6 plus truncating division of a negative product. | No rejection; wrapped product then `/ 8` truncating **toward zero** (not floor). Must match exactly. |
| 17 | `encode_quant` | **All six arguments simultaneously at the signed extremes** — the cross-product of `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}^6` (117 649 tuples), i.e. every "one step past the valid range" combination at once. | No rejection; every tuple returns an `int`. Must match exactly for all of them. |
| 18 | `encode_quant` | **Null pointers / oversized lengths**: *not applicable and proven so* — `nm`/the header show the ABI is 6 by-value `int`s and no pointer or length parameter exists, so there is no pointer to null and no buffer to oversize. The nearest reachable analogue is passing `0` and `INT_MIN`/`INT_MAX` in every slot, which rows 14–17 cover. | N/A by construction; documented rather than invented. |
| 19 | `encode_quant` | **Division `/ 8` (lines 30/36/42) can never trap**: the divisor is the literal `8`, so neither divide-by-zero nor the `INT_MIN / -1` overflow is reachable. | N/A by construction — no `SIGFPE` path exists in either library. Asserted by testing `step = INT_MIN` (row 16) without a crash. |
| 20 | `encode_quant` | **Extra/garbage argument bits across the FFI boundary**: passing values whose upper bits are set so a wrong `int`/`long`/`unsigned` widening in the Rust wrapper would show up (`0xFFFF_FFFF`, `0x8000_0000` as `u32`-derived `c_int`). | No rejection; both must sign-extend/truncate identically, i.e. the Rust `extern "C" fn(c_int,...) -> c_int` must have the identical calling convention. |

All 20 rows have a dedicated differential test in
`translation/tests/phase_c_errors.rs`; see that file's checklist and the
`ERRORS.md` checklist at the bottom of this document.

## Checklist (Phase C)

- [x] Row 1 — `lsbit` out-of-range positive enum values
- [x] Row 2 — `lsbit` negative enum values
- [x] Row 3 — `lsbit` extreme enum values / one step past `4`
- [x] Row 4 — `uni + 1` overflow at `INT_MAX`
- [x] Row 5 — `uni - 1` overflow at `INT_MIN`
- [x] Row 6 — `(2*(uni&7)+1)*step` overflow
- [x] Row 7 — `-diff` overflow proven unreachable; both reachable `diff` extremes (`±2^28`) tested through the negation branch
- [x] Row 8 — `pred + diff` overflow
- [x] Row 9 — `tgt - p` / `tgt2 - p` overflow
- [x] Row 10 — `d += d3 >> 5` overflow
- [x] Row 11 — `d >> 31` on negative values
- [x] Row 12 — `d3 >> 5` shift semantics
- [x] Row 13 — `(uni >> 1) & (uni >> 2)` on negative values
- [x] Row 14 — `step == 0`
- [x] Row 15 — negative `step`
- [x] Row 16 — oversized `step`
- [x] Row 17 — full extremes cross-product (117 649 tuples)
- [x] Row 18 — null-pointer/length surface proven N/A
- [x] Row 19 — division trap surface proven N/A
- [x] Row 20 — argument widening / calling convention
