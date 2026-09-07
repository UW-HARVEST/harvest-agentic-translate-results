# Divergences found and fixed

The Rust translation already had the right control flow, symbol set and data
tables. Verification found **five classes of real divergence**, all in the
*operand roles* of scalar SSE float arithmetic, which decide **which NaN payload
survives** an operation. `xxxss dst, src` returns the quieted `dst` when `dst`
is NaN, otherwise the quieted `src` — so `addss(a, b)` and `addss(b, a)` differ
whenever both operands are NaN with different payloads.

The correct roles were derived **mechanically from the compiled C**
(`objdump -d c_src/build/libharvest-work-NQTXuV.so`), not guessed. Example of
the evidence used, for `c2Dot`:

```asm
movss  a.x,%xmm1 ; movss b.x,%xmm0 ; mulss %xmm0,%xmm1   ; xmm1 = mulss(a.x, b.x)
movss  a.y,%xmm2 ; movss b.y,%xmm0 ; mulss %xmm2,%xmm0   ; xmm0 = mulss(b.y, a.y)
addss  %xmm1,%xmm0                                       ; addss(b.y*a.y, a.x*b.x)
```

| # | function | before (wrong) | after (matches C) | how it was caught |
|---|----------|----------------|-------------------|-------------------|
| 1 | `c2Dot` / `lm_dot2` | `addss(a.x*b.x, a.y*b.y)` | `addss(b.y*a.y, a.x*b.x)` — the `addss` destination is the *second* product | `cfg08_09_c2Sub_c2Dot`: C returned `0x7fffffff`, Rust `0x7fc00001` |
| 2 | `c2CircletoCircle` | `r2 = addss(A.r, B.r)` | `r2 = addss(B.r, A.r)` | disassembly (`movss A.r,%xmm1 ; movss B.r,%xmm0 ; addss %xmm1,%xmm0`) |
| 3 | `f9` | 5 hand-written dot products with inconsistent operand order; `mulss(dot11, dot00)`, `mulss(dot12, dot01)`, `mulss(dot12, dot00)`, `mulss(dot02, dot01)` | a single `lm_dot2` helper with the C's roles; `mulss(dot00, dot11)`, `mulss(dot01, dot12)`, `mulss(dot00, dot12)`, `mulss(dot01, dot02)` | disassembly of `f9` + `cfg46_47_f9_float_zoo` |
| 4 | `f11` | `m = mulss(1.0, l - 0.5*c)` — kept the source's `1.0f *` | `m = subss(l, mulss(c, 0.5))` — **GCC folds the `1.0f *` away**, so the C does *not* quiet a NaN there | disassembly: only one `mulss` before the store to `m` |
| 4b | `f11` branches 3–6 | `addss(m, x)` / `addss(m, c)` | `addss(x, m)` / `addss(c, m)` — C's `dst` is always the left operand of the source-level `+` | disassembly of each branch body |
| 5 | `f13` | `mulss(h, 60.0)`, `addss(h, 360.0)` | `mulss(60.0, h)`, `addss(360.0, h)` — the constant is the `dst` | disassembly (`movss h,%xmm1 ; movss 60.0,%xmm0 ; mulss %xmm1,%xmm0`) |

## Non-divergences confirmed (things that looked risky but already matched)

- `c2Sub` / `lm_sub2`: `subss(a.x, b.x)` — already correct.
- `f11`'s `c` and `x`, and all of `f12`'s `p`/`q`/`t`: operand roles already
  matched the emitted code exactly.
- `f12`'s `switch (i)` is compiled as an **unsigned** `cmpl $0x4 ; ja default`,
  so negative `i` reaches `default:`; the Rust `match` on `i32` is equivalent.
- `f3` never actually executes `INT_MIN / -1` (every such path is guarded), so
  the `wrapping_div` calls can not diverge from the C's `idiv`.
- `fmodf` (`f11`) and `floorf` (`f12`): Rust's `%` and `.floor()` agree with
  glibc bit-for-bit across the whole tested space.
- `c_cast_f32_to_i32`: the `cvttss2si` "integer indefinite" emulation
  (`i32::MIN` for NaN and out-of-range) matches on both boundaries.

## Test-side correction

`ERRORS.md` row 11's first draft asserted `f3`'s correction produces a true
mathematical floor (`trunc - 1`). The C actually computes
`q + (v2 > 0 ? -1 : 1)`, which **adds** one when `v2 < 0` — e.g.
`f3(-200, -199) == 2`. The Rust already matched the C; the *test's* invariant was
wrong and was corrected to the C's behaviour. The C is ground truth.
