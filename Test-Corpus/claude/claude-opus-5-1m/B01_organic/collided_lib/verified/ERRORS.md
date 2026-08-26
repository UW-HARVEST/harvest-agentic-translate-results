# ERRORS.md — Error / rejection surface table (Phase A → gate for Phase C)

Mechanically derived by grepping `c_src/src/lib.c` and `c_src/include/lib.h`
for every rejection path. The greps used:

```sh
grep -n 'return\|assert\|NULL\|default:\|case ' c_src/src/lib.c
grep -n '' c_src/include/lib.h
```

Findings: the library contains **no** `assert`, **no** `RETURN_ERROR`-style
macro, **no** `-1` / `NULL` error return, **no** allocation, and **no** explicit
range/limit constants. The *only* rejection mechanism in the whole library is
the `default:` arm of the three `switch` statements in `collided`, each of which
returns `0`. Everything else in the library is total over its input domain
(pure float arithmetic and comparisons), so its "error" behaviour is the
IEEE-754 result (NaN / ±Inf) rather than a rejection.

`C2_TYPE` is a C enum, so any `int` bit pattern is a legal argument at the ABI
level; values with no valid variant must hit the `default:` arms. GCC gives
this enum an `unsigned int` underlying type (all enumerators are non-negative),
so a negative argument arrives as a large unsigned value — either way it
matches none of the `case` labels and falls through to `default:`.

## Rejection rows

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|--------------------------------------------|-------------------|------|
| E1 | `collided` (`lib.c:95-96`, outer `default:`) | `typeA` is any value other than `0`/`1` (e.g. `2`, `3`, `-1`, `INT_MAX`, `INT_MIN`, `0x100000000`-truncated), `typeB` valid | returns `0`; **neither** `A` nor `B` is dereferenced | `err_e1_outer_default_bad_typea` |
| E2 | `collided` (`lib.c:81-82`, inner `default:` under `case C2_TYPE_CIRCLE`) | `typeA == C2_TYPE_CIRCLE` (`0`) and `typeB` is any value other than `0`/`1` | returns `0`; `B` is **not** dereferenced (`A` is not read either) | `err_e2_circle_bad_typeb` |
| E3 | `collided` (`lib.c:91-92`, inner `default:` under `case C2_TYPE_AABB`) | `typeA == C2_TYPE_AABB` (`1`) and `typeB` is any value other than `0`/`1` | returns `0`; `B` is **not** dereferenced (`A` is not read either) | `err_e3_aabb_bad_typeb` |
| E4 | `collided` (E1 combined with E2/E3 domain) | **both** `typeA` and `typeB` invalid | returns `0` (outer `default:` wins, inner switch never entered) | `err_e4_both_types_invalid` |
| E5 | `collided` | `A == NULL` / `B == NULL` **together with an invalid type tag** — the only null case the C code defines, because the invalid tag short-circuits before any dereference | returns `0`, no crash | `err_e5_null_pointers_with_invalid_type` |
| E6 | `collided` | out-of-range enum values passed across FFI, swept exhaustively over `-8..=8` plus `i32::MIN`/`i32::MAX`/`u32::MAX as i32` for both `typeA` and `typeB` (full cross product) | `0` unless *both* tags are in `{0,1}` | `err_e6_enum_cross_product_sweep` |

### Non-rejections deliberately recorded (so they are not mistaken for errors)

| # | function | condition | C behaviour (NOT an error — must be matched exactly) | test |
|---|----------|-----------|------------------------------------------------------|------|
| N1 | `c2CircletoCircle` | negative radius `A.r`/`B.r` | no validation; `r2 = (A.r+B.r)^2` squares away the sign, so a negative sum still yields a positive threshold | `nonerr_n1_negative_radii` |
| N2 | `c2CircletoAABB` | inverted AABB (`min > max`) | no validation; `c2Clampv` = `max(lo, min(a, hi))` returns `lo` for an inverted box | `nonerr_n2_inverted_aabb` |
| N3 | `c2AABBtoAABB` | inverted / degenerate boxes (`min == max`, `min > max`) | no validation; the four `<` comparisons are evaluated as-is | `nonerr_n3_inverted_degenerate_aabb` |
| N4 | any float entry point | NaN operand | comparisons with NaN are false: `c2Maxv`/`c2Minv` select `b`, and every `d2 < r2` test returns `0` | `nonerr_n4_nan_operands` |
| N5 | any float entry point | ±Inf operands, `Inf - Inf`, overflow to Inf in `c2Dot` | plain IEEE-754 results, no clamping or checks | `nonerr_n5_infinities` |
| N6 | `collided` | zero-size / oversized "length" — the API takes **no** length argument, so there is no length to validate; the shape size is implied by the type tag | n/a (documented absence) | — |
| N7 | `collided` | unaligned `A`/`B` for a *valid* tag | C does a normal (unaligned-tolerant on x86-64) load; must not change results | `cfg_c14_collided_unaligned_pointers` |

## Phase C completion status

Test file: `tests/error_paths.rs` (run against both `.so`s via `libloading`).

| row | test | status |
|-----|------|--------|
| E1 | `err_e1_outer_default_bad_typea` | [x] passing |
| E2 | `err_e2_circle_bad_typeb` | [x] passing |
| E3 | `err_e3_aabb_bad_typeb` | [x] passing |
| E4 | `err_e4_both_types_invalid` | [x] passing |
| E5 | `err_e5_null_pointers_with_invalid_type` | [x] passing |
| E6 | `err_e6_enum_cross_product_sweep` | [x] passing |
| N1 | `nonerr_n1_negative_radii` | [x] passing |
| N2 | `nonerr_n2_inverted_aabb` | [x] passing |
| N3 | `nonerr_n3_inverted_degenerate_aabb` | [x] passing |
| N4 | `nonerr_n4_nan_operands` | [x] passing |
| N5 | `nonerr_n5_infinities` | [x] passing |
| N6 | n/a — the API has no length parameter to validate | [x] documented |
| N7 | `cfg_c14_collided_unaligned_pointers` (in `tests/valid_paths.rs`) | [x] passing |

Each row asserts the *exact* returned sentinel from both libraries (`0`), not
merely that both "failed somehow", and additionally asserts that the C result
really is `0` so the test cannot pass by two libraries agreeing on a wrong value.

**All rows checked — 0 unchecked rows remain.**
