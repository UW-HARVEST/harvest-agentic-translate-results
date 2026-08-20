# CONFIGS.md — Configuration-surface table (Phase A, gate for Phase B)

## Mechanical derivation of the axes

There is exactly one public entry point (`c_src/include/lib.h`):

```c
float half2float(uint16_t h);
```

`half2float` is **branch-free** (`grep -cE '\b(if|else|switch|while|for|goto)\b'`
→ 0) and takes **no options, modes or flags** — there is no context struct, no
setter, no global, no `#ifdef`. So the runtime-option axis is empty and the
whole configuration surface comes from the **input shape**: which rows of the
three lookup tables the single input `h` selects.

```c
int n = h >> 10;                                            /* axis A */
out.num = m__mantissa[(h & 0x3ff) + m__offset[n]]           /* axis B, axis C */
        + m__exponent[n];                                   /* axis D */
```

| axis | derived from | distinct states the C code actually treats differently |
|------|--------------|--------------------------------------------------------|
| **A** — table row `n` | `int n = h >> 10` | `0 … 63` (top 6 bits of `h`: sign bit + 5 exponent bits) |
| **B** — mantissa field | `h & 0x3ff` | `0` vs `1…1022` vs `1023` (low 10 bits) |
| **C** — mantissa-table region | `m__offset[n]`, whose only two values are `0x0000` (**only** at `n = 0` and `n = 32`) and `0x0400` (all other 62 rows) | region `[0..1023]` vs region `[1024..2047]` |
| **D** — exponent addend | `m__exponent[n]` | `0x00000000` (`n=0`); linear `n·0x00800000` (`n=1..30`); **`0x47800000` (`n=31`, the inf/NaN row — *not* the linear value)**; `0x80000000` (`n=32`); `0x80000000 + n'·0x00800000` (`n=33..62`); **`0xC7800000` (`n=63`)** |
| **E** — result class | consequence of A–D | `+0`, `−0`, positive subnormal-source, negative subnormal-source, positive normal, negative normal, `+inf`, `−inf`, `+NaN` (2046 payloads), `−NaN` |
| **F** — `uint32_t` addition | `mant + exp` | never wraps (verified: max sum `0xFFFFE000`, 0 overflows over the whole 65 536-value domain) — but must still be reproduced with wrapping semantics |
| **G** — return path | `union { float flt; uint32_t num; }` | raw-bit identity, incl. NaN payload & sign bit (must be compared as `u32` bits, never with float `==`) |
| **H** — FFI/ABI shape | the exported `T half2float` symbol | called through `extern "C" fn(u16) -> f32` loaded from the `.so` (return in `xmm0`) |

Aliasing trap that the rows are designed to catch: `m__mantissa[512] ==
m__mantissa[1024] == 0x38000000`, so a wrong `m__offset` lookup still yields the
right answer for *some* inputs. Rows therefore exercise both index regions at
values where the two regions differ (`m__mantissa[513] = 0x38004000` vs
`m__mantissa[1025] = 0x38002000`).

## Configuration-surface table

Cross-product of axes A×B×C×D, pruned to the combinations the C distinguishes.
Every row is driven through the `.so` export of **both** libraries and compared
as raw `u32` bits. Rows marked *randomized* use ≥ 4 096 pseudo-random inputs
from a fixed-seed (`0x1234_5678_9ABC_DEF0`) SplitMix64 generator, in addition to
the row's boundary values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| C1 | `half2float` | **A** `n = 0`, **B** mantissa `0`, **C** offset `0x0000` → index `0`, **D** exponent `0x00000000`. Single input `h = 0x0000` (`+0.0`). | [x] |
| C2 | `half2float` | **A** `n = 0`, **B** mantissa `1…1023` (positive half-subnormals), **C** offset `0x0000` → index `1…1023`, **D** exponent `0`. All 1 023 inputs exhaustively. | [x] |
| C3 | `half2float` | **A** `n = 32` (sign bit set, exp field 0), **B** mantissa `0`, **C** index `0`, **D** exponent `0x80000000`. Single input `h = 0x8000` (`−0.0`) — must differ from C1 only in the sign bit. | [x] |
| C4 | `half2float` | **A** `n = 32`, **B** mantissa `1…1023` (negative half-subnormals), **C** offset `0x0000` → index `1…1023`, **D** exponent `0x80000000`. All 1 023 inputs exhaustively. | [x] |
| C5 | `half2float` | **A** `n ∈ 1…30` (positive normals), **B** mantissa `0`, **C** offset `0x0400` → index **`1024`** (region boundary), **D** linear exponent. All 30 inputs. | [x] |
| C6 | `half2float` | **A** `n ∈ 1…30`, **B** mantissa `1…1022`, **C** offset `0x0400` → index `1025…2046`, **D** linear exponent. *Randomized* over the 30×1022 space. | [x] |
| C7 | `half2float` | **A** `n ∈ 1…30`, **B** mantissa `1023`, **C** index **`2047`** (last table element), **D** linear exponent. All 30 inputs. | [x] |
| C8 | `half2float` | **A** `n = 31` (**special exponent row `0x47800000`**), **B** mantissa `0`, **C** index `1024`, **D** ⇒ `0x7F800000`. Single input `h = 0x7C00` (`+inf`). | [x] |
| C9 | `half2float` | **A** `n = 31`, **B** mantissa `1…1023` (**+NaN payloads**, axis G), **C** index `1025…2047`. All 1 023 inputs exhaustively, compared as bits. | [x] |
| C10 | `half2float` | **A** `n ∈ 33…62` (negative normals), **B** mantissa `0`, **C** index `1024`, **D** sign-bit-set linear exponent. All 30 inputs. | [x] |
| C11 | `half2float` | **A** `n ∈ 33…62`, **B** mantissa `1…1022`, **C** index `1025…2046`. *Randomized* over the 30×1022 space. | [x] |
| C12 | `half2float` | **A** `n ∈ 33…62`, **B** mantissa `1023`, **C** index `2047`. All 30 inputs. | [x] |
| C13 | `half2float` | **A** `n = 63` (**special exponent row `0xC7800000`**), **B** mantissa `0`, **C** index `1024`, **D** ⇒ `0xFF800000`. Single input `h = 0xFC00` (`−inf`). | [x] |
| C14 | `half2float` | **A** `n = 63`, **B** mantissa `1…1023` (**−NaN payloads**), **C** index `1025…2047`, up to `h = 0xFFFF` (max sum `0xFFFFE000`, axis F). All 1 023 inputs exhaustively, compared as bits. | [x] |
| C15 | `half2float` | **C** region-aliasing discriminator: inputs whose mantissa index lands in `[512..1023]` (offset `0x0000`, step `0x4000`) paired with inputs landing in `[1024..2047]` (offset `0x0400`, step `0x2000`), i.e. `h ∈ {0x0200..0x03FF} ∪ {0x8200..0x83FF}` vs `n ∉ {0,32}`. Catches a swapped/constant `m__offset`. All 1 024 offset-0 high-mantissa inputs. | [x] |
| C16 | `half2float` | **A** every one of the 64 rows × **B** mantissa `{0, 1, 511, 512, 1022, 1023}` — the full pruned A×B cross-product at boundary mantissas (384 inputs). | [x] |
| C17 | `half2float` | **E** result-class sweep: for each of the 10 result classes (`+0`, `−0`, ±subnormal-source, ±normal, ±inf, ±NaN) assert the C and Rust bit patterns agree **and** land in the expected class, so a class mix-up cannot hide behind bit equality. | [x] |
| C18 | `half2float` | **H** ABI/statelessness: the **exhaustive** domain — all 65 536 `uint16_t` values through both `.so` exports, compared as `u32` bits. This is the complete valid-input space of the library. | [x] |
| C19 | `half2float` | **H/G** call-order independence & purity: the exhaustive domain replayed in a fixed-seed shuffled order, then each `h` called 3× interleaved between the two libraries; every call for a given `h` must return identical bits (the C tables are non-`const` `static`, so a hidden mutation would show up here). | [x] |
| C20 | `half2float` | **H** *randomized* property sweep: 200 000 fixed-seed random `u16` values (with replacement) driven alternately C-then-Rust and Rust-then-C, guarding against order-dependent lazy-init differences. | [x] |

**Rows: 20. Unchecked rows: 0.**

## Build-configuration coverage

`Cargo.toml` declares no `[features]`; `c_src/CMakeLists.txt` declares no
options or `-D` defines. The single valid combination is the empty/default one,
so every row above is verified under **all** (= 1) feature combinations:

| # | feature combination | rows verified |
|---|---------------------|---------------|
| 1 | *(none / default)* — `--no-default-features` | C1 … C20 |
