# ERRORS.md — Phase A: error / rejection surface table

## Mechanical grep of the C source

```sh
$ grep -nE 'RETURN_ERROR|return *-1|return *NULL|assert|errno|abort|exit|E[A-Z]+|if *\(' c_src/src/lib.c c_src/include/lib.h
(no matches)
```

`c_src/src/lib.c` contains **no** error-return macro, **no** `return -1`, **no**
`return NULL`, **no** error enum, **no** `assert`, **no** explicit range check,
and **no** null check. The function has no pointer parameters and no enum
parameters, so there is no null-pointer surface and no out-of-range-enum
surface. Every `int` bit pattern and every `float` bit pattern is an accepted
input, and the function always returns a `float`.

The de-facto rejection surface is therefore the set of **degenerate / boundary /
undefined-behaviour conditions** the C hits, each of which produces a specific
observable result that the Rust must reproduce bit-for-bit. Every row below was
derived from the actual C constructs (`?:` clamp, `&`, `>>`, `do/while`,
`*=`) and its "expected C result" column was measured by calling the reference C
`.so`. The Rust must return the **same bit pattern** — that is the analogue of
"the same error code, not merely both failed somehow".

Legend for the derivation column: `e = min(exp_q2, 120)`,
`cnt = (e >> 2) & 31` (x86 `sar %cl`), `shifted = (1<<30) >> cnt`,
`product = g_expfrac[e & 3] * (float)shifted`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| E1 | `ldexp_q2` | **UB: negative shift count.** `exp_q2 < 0` makes `e >> 2 < 0`, so `1 << 30 >> (e>>2)` shifts by a negative count (C UB). gcc emits `sar %cl,%edx`; x86 masks `%cl` to 5 bits. | No trap, no error: returns `y * g_expfrac[e&3] * 2^(30 - ((e>>2)&31))`. Must NOT be "shift-count panic" and must NOT be a saturating/0 shift. e.g. `ldexp_q2(1.0, -5) = 0x301837f0` | [x] |
| E2 | `ldexp_q2` | **Shift count lands on 31** → `shifted == 0` → `product == 0`. Happens for `e>>2 == -1`, i.e. `exp_q2 ∈ {-1,-2,-3,-4}` (and every `exp_q2` with `(exp_q2>>2) ≡ 31 (mod 32)`, e.g. `-129..-132`). | Returns signed zero: `ldexp_q2(1.0,-1) = 0x00000000`, `ldexp_q2(-1.0,-4) = 0x80000000`, `ldexp_q2(1.0,-129) = 0x00000000` | [x] |
| E3 | `ldexp_q2` | **`0 * inf` invalid operation** — row E2's `product == 0` combined with `y = ±inf`. | Returns the x86 "real indefinite" QNaN `0xffc00000` for BOTH signs of infinity: `ldexp_q2(+inf,-1) = 0xffc00000`, `ldexp_q2(-inf,-2) = 0xffc00000` | [x] |
| E4 | `ldexp_q2` | **Negative index expression** `e & 3` with `e < 0`. In C this is a bit-mask on a two's-complement negative, so the index is still `0..3` — the array read is never out of bounds. A naive Rust `e % 4` or `e as usize & 3` would index OOB / wrap. | Never out of bounds; `-129 & 3 == 3`, `-4 & 3 == 0`, `INT_MIN & 3 == 0`. No crash, no OOB read. | [x] |
| E5 | `ldexp_q2` | **`INT_MIN` (`-2147483648`)** — most negative `int`. `e = INT_MIN`; `e >> 2 = -536870912`; `cnt = 0`; `exp_q2 -= e` must not overflow-trap. | Returns `y * g_expfrac[0] * 2^30` = `y`; `ldexp_q2(1.0, INT_MIN) = 0x3f800000`. Must not panic in a Rust debug build (`wrapping_sub` required). | [x] |
| E6 | `ldexp_q2` | **`INT_MIN + 1 .. INT_MIN + 3`** — the other residues at the extreme negative end. | `ldexp_q2(1.0, INT_MIN+1) = 0x3f5744fd`, `+2 = 0x3f3504f3`, `+3 = 0x3f1837f0` | [x] |
| E7 | `ldexp_q2` | **`INT_MAX` (`2147483647`)** — maximum loop trip count (17 895 697 iterations of `exp_q2 -= 120`). Tests that the loop terminates and that the final `exp_q2 -= e` with `e == exp_q2` reaches exactly 0. | Terminates; `ldexp_q2(1.0, INT_MAX) = 0x00000000` (underflowed to +0), `ldexp_q2(+inf, INT_MAX) = 0x7f800000` | [x] |
| E8 | `ldexp_q2` | **`exp_q2 == 0`** — the `do/while` body still runs once, so this is *not* an identity/early-return. | `product == 1.0` exactly, so the result equals `y` bit-for-bit for every `y` (including NaN payloads and signed zeros). `ldexp_q2(y,0) == y`. | [x] |
| E9 | `ldexp_q2` | **`exp_q2 < 0` never early-returns** — no `if (exp_q2 <= 0) return y;` guard exists. A translation that adds one diverges for all negative inputs. | `ldexp_q2(1.0, -1) = 0` ≠ `1.0`; `ldexp_q2(1.0, -5) = 0x301837f0` ≠ `1.0` | [x] |
| E10 | `ldexp_q2` | **Clamp boundary `exp_q2 == 120`** — the `?:` uses `>` not `>=`, so `exp_q2 == 120` takes the `120` branch; `exp_q2 == 119` takes the `exp_q2` branch. Off-by-one in the clamp changes the trip count. | `ldexp_q2(1.0,119) = 0x309837f0`, `ldexp_q2(1.0,120) = 0x30800000`, `ldexp_q2(1.0,121) = 0x305744fd` | [x] |
| E11 | `ldexp_q2` | **sNaN input** (`0x7f800001`) — a signalling NaN crosses the FFI boundary and is quieted by `mulss`. | Quieted, payload preserved: `ldexp_q2(0x7f800001, 5) = 0x7fc00001` | [x] |
| E12 | `ldexp_q2` | **qNaN payload/sign propagation** — `y` is the *source* operand of the final `mulss`, so the NaN operand is returned. | Payload and sign preserved: `ldexp_q2(0x7fc00001,5) = 0x7fc00001`, `ldexp_q2(0xffc0dead,7) = 0xffc0dead` | [x] |
| E13 | `ldexp_q2` | **Signed zero input** — `±0 * product` must preserve the sign of zero. | `ldexp_q2(+0.0,3) = 0x00000000`, `ldexp_q2(-0.0,3) = 0x80000000` | [x] |
| E14 | `ldexp_q2` | **Gradual underflow / flush to zero** — subnormal `y` scaled by `product < 1`, result rounds to zero or stays subnormal (round-to-nearest-even). | `ldexp_q2(0x00000001, 1) = 0x00000001` (rounds up to min subnormal), `ldexp_q2(0x00000001, 4) = 0x00000000` (0.5·min ties to even → +0) | [x] |
| E15 | `ldexp_q2` | **Overflow is impossible** — `product` is at most `g_expfrac[0] * 2^30 == 1.0` exactly, so `|result| <= |y|` always. A translation that mis-scales (e.g. wrong shift direction) would overflow to `inf` here. | `ldexp_q2(FLT_MAX, -128) = 0x7f7fffff` (unchanged, product == 1.0), never `inf` | [x] |
| E16 | `ldexp_q2` | **Shift count wraps back to 0** — `(e>>2) & 31 == 0` for `e ∈ [-128,-125]`, so a deeply negative `exp_q2` produces `product == g_expfrac[e&3] * 2^30`, i.e. *amplification* rather than the expected decay. | `ldexp_q2(1.0,-128) = 0x3f800000`, `ldexp_q2(1.0,-125) = 0x3f1837f0`, `ldexp_q2(1.0,-124) = 0x3f000000` | [x] |
| E17 | `ldexp_q2` | **Generic FFI boundary sweep** — no null-pointer or enum surface exists, so the equivalent "one step past the valid range" sweep is *every* `int` boundary: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `119`, `120`, `121`, `INT_MAX-1`, `INT_MAX`, plus every `float` class (`±0`, subnormal, normal, `±FLT_MAX`, `±inf`, qNaN, sNaN). | All return a defined `float`; C and Rust must agree bit-for-bit on every one. | [x] |

All 17 rows have a passing differential test in `tests/error_paths.rs`.
