# ERRORS.md — Phase C error / rejection surface table

Mechanically derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical scan of the C source

Every rejection idiom was grepped for across all of `c_src`:

| pattern searched | matches |
|---|---|
| `RETURN_ERROR` / `*_ERROR` / error macros | 0 |
| `return -1` / `return NULL` / `return 0;` as sentinel | 0 |
| `errno` / `set_error` / error enums / `typedef enum` | 0 |
| `assert` / `static_assert` / `abort` / `exit` | 0 |
| `if (` / `switch` (any conditional at all) | 0 |
| `goto` / early `return` other than the single tail `return` | 0 |
| null-pointer checks (`== NULL`, `!ptr`) | 0 (no pointer parameters exist) |
| explicit range / bounds checks (`<`, `>`, `<=`, `>=`) | 0 |
| `MIN` / `MAX` / limit constants | 0 |
| `#ifdef` / `#if` conditional compilation | 0 |
| malloc/free (allocation-failure paths) | 0 |

Command used:

```
grep -nE 'RETURN_ERROR|return .*(-1|NULL)|errno|assert|abort|exit\(|if *\(|switch *\(|goto|== *NULL|#if|MIN|MAX|alloc' c_src/src/lib.c c_src/include/lib.h
# -> no matches
```

`float2half` is total and branch-free:

```c
uint16_t float2half(float flt) {
    union { float flt; uint32_t num; } in;
    uint32_t n, j;
    in.flt = flt;
    n = in.num;
    j = (n >> 23) & 0x1ff;                        /* masked: always 0..511 */
    return (uint16_t)((uint32_t)m__base[j] + ((n & 0x007fffff) >> m__shift[j]));
}
```

**There is no error surface intrinsic to the C code**: the API has one scalar,
by-value `float` parameter, no pointers, no lengths, no enums, no flags, no
out-of-band error channel, and a `uint16_t` return type whose entire value range
is a legitimate result. Every one of the 2^32 possible `float` bit patterns is a
valid input that the function must map to a defined `uint16_t` — there is no
input it can reject.

Two safety properties are guaranteed by construction rather than by a check, and
both are still tested below because they are exactly where a Rust translation can
diverge (Rust panics where C is silent):

- `j` is masked with `& 0x1ff`, so the `m__base[j]` / `m__shift[j]` accesses can
  never go out of bounds — in Rust an unmasked index would panic instead.
- every `m__shift` entry is `0x0d`..`0x18` (13..24), all `< 32`, so the
  `>> m__shift[j]` is never a UB/panicking over-wide shift on a 32-bit value —
  in Rust an over-wide shift would panic in debug and be masked in release.

## Error-surface table

One row per distinct rejection the C actually performs, plus the generic FFI
boundary conditions that must be verified even though the table has no
intrinsic rejections. "expected C result" is the concrete value/behaviour the C
`.so` produces, and each row's test asserts the Rust `.so` produces *that same
value*, not merely "also didn't crash".

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|---------------------------------------------|-------------------|------|-----|
| E1 | `float2half` | *(no rejection branch exists in the C source)* — table intentionally has no intrinsic-error rows; documented here so the absence is explicit and auditable | n/a — function is total, cannot fail | `no_intrinsic_error_paths_documented` | [x] |
| E2 | `float2half` | quiet NaN, positive: bits `0x7fc00000` | defined `uint16_t`, no trap | `errors_special_values` | [x] |
| E3 | `float2half` | quiet NaN, negative: bits `0xffc00000` | defined `uint16_t`, no trap | `errors_special_values` | [x] |
| E4 | `float2half` | signalling NaN, positive: bits `0x7f800001` (payload must survive the FFI unchanged, no quieting) | defined `uint16_t`, no trap | `errors_special_values` | [x] |
| E5 | `float2half` | signalling NaN, negative: bits `0xff800001` | defined `uint16_t`, no trap | `errors_special_values` | [x] |
| E6 | `float2half` | NaN with every payload bit set: `0x7fffffff` / `0xffffffff` (max mantissa, exercises the `uint16_t` truncation of `base + mantissa`) | defined `uint16_t`, wraps/truncates rather than saturating | `errors_special_values` | [x] |
| E7 | `float2half` | `+inf` (`0x7f800000`) | `0x7c00` | `errors_special_values` | [x] |
| E8 | `float2half` | `-inf` (`0xff800000`) | `0xfc00` | `errors_special_values` | [x] |
| E9 | `float2half` | `+0.0` (`0x00000000`) | `0x0000` | `errors_special_values` | [x] |
| E10 | `float2half` | `-0.0` (`0x80000000`) | `0x8000` | `errors_special_values` | [x] |
| E11 | `float2half` | value one step past the largest finite half (overflow to infinity): `65520.0`, `65536.0`, `f32::MAX` | `0x7c00` (`0xfc00` negated) | `errors_range_boundaries` | [x] |
| E12 | `float2half` | value one step below the smallest half subnormal (underflow to zero): `f32::MIN_POSITIVE`, `2^-25`, `f32` smallest subnormal `0x00000001` | `0x0000` (`0x8000` negated) | `errors_range_boundaries` | [x] |
| E13 | `float2half` | exact boundary between half-subnormal and half-normal exponent buckets: `j` in `{101,102,112,113}`, i.e. bits around `0x33000000`/`0x38800000` | per-table value | `errors_range_boundaries` | [x] |
| E14 | `float2half` | index-mask boundary: every `j` where `m__shift[j]` changes value (all 28 run boundaries), and the two `j` values `255` and `511` that hold an anomalous `0x0d` shift inside an otherwise `0x18` region (these are the `+inf`/`-inf`/NaN buckets, so a NaN mantissa is *shifted in* there rather than discarded) | per-table value | `errors_shift_table_anomalies` | [x] |
| E15 | `float2half` | inputs that drive the **maximum** possible `base + mantissa` sum, to probe the `(uint16_t)` narrowing cast. Mechanically checked: `max over j of (m__base[j] + (0x7fffff >> m__shift[j])) == 0xffff` exactly, attained at `j = 511` (`base = 0xfc00`, `shift = 13`, mantissa term `0x3ff`), so the cast never actually discards bits and never wraps. The row is kept and tested because a Rust translation using `u16` arithmetic instead of C's `uint32_t` intermediate would sit exactly on that `0xffff` edge and could overflow-panic. | `0xffff` at `j = 511`; no wraparound anywhere | `errors_truncating_cast` | [x] |
| E16 | `float2half` | out-of-range "enum"-style abuse: the parameter is a `float`, so the analogue is a bit pattern that is not any canonical float — all 2^9 possible `j` buckets reached with a non-canonical mantissa, incl. pseudo-denormals | defined `uint16_t` for all 512 buckets | `errors_all_512_buckets_reachable` | [x] |
| E17 | `float2half` | ABI edge: reading the return value's upper 16 bits — C returns `uint16_t` in `ax`, upper bits of `eax` unspecified; Rust must return the same 16-bit value | low 16 bits equal | `errors_special_values` | [x] |

Rows E1 is a documentation row (asserted by re-running the grep in the test);
E2–E17 are executable differential tests. There are no null-pointer,
zero-length, oversized-length, or enum-variant rows because the API surface has
no pointer, length, or enum parameter — this is asserted mechanically in
`no_intrinsic_error_paths_documented` by checking the header's single
declaration.
