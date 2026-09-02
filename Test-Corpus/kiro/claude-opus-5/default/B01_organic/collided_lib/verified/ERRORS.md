# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`:

```sh
grep -n "return" c_src/src/lib.c
grep -nE "assert|NULL|ERROR|errno|-1|#if|#def|<=|>=" c_src/src/lib.c c_src/include/lib.h  # -> no matches
```

**Findings.** The library has **no** `RETURN_ERROR`-style macro, **no** `assert`,
**no** `errno` use, **no** `NULL` check, **no** `return -1`, **no** `return NULL`,
**no** error enum, and **no** explicit numeric range check or MIN/MAX constant.
It never allocates and never fails. Its *entire* rejection surface consists of
the three literal `return 0;` statements in `collided`, reached from the
`default:` label of a `switch` on a `C2_TYPE` tag.

The `0` returned by `c2CircletoCircle` / `c2CircletoAABB` / `c2AABBtoAABB` is
**not** an error — it is the valid boolean "no overlap" result, and is covered by
`CONFIGS.md` (Phase B) rather than here. Rows below are only the paths where the
C *rejects its input* instead of computing a geometric answer.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `collided` | `typeA` has no valid variant (any `int` ∉ {0,1}), `typeB` = `C2_TYPE_CIRCLE` (0) — `lib.c:96` outer `default:` | returns `0`; **A and B are never dereferenced** | `err_row01_typeA_invalid_typeB_circle` | [x] |
| 2 | `collided` | `typeA` invalid, `typeB` = `C2_TYPE_AABB` (1) — `lib.c:96` outer `default:` | returns `0`; no deref | `err_row02_typeA_invalid_typeB_aabb` | [x] |
| 3 | `collided` | `typeA` invalid **and** `typeB` invalid — `lib.c:96` outer `default:` | returns `0`; no deref | `err_row03_both_types_invalid` | [x] |
| 4 | `collided` | `typeA` = `C2_TYPE_CIRCLE` (0), `typeB` invalid — `lib.c:82` inner `default:` | returns `0`; **`A` is not dereferenced either** (the deref is inside the taken-case expression only) | `err_row04_typeA_circle_typeB_invalid` | [x] |
| 5 | `collided` | `typeA` = `C2_TYPE_AABB` (1), `typeB` invalid — `lib.c:92` inner `default:` | returns `0`; no deref | `err_row05_typeA_aabb_typeB_invalid` | [x] |

## Generic C-API boundaries also covered (not table rows)

The C code has no checks for these, so the requirement is that Rust reproduce
the *same* observable behaviour, not that it add validation.

| boundary | how the C behaves | covered by |
|----------|-------------------|-----------|
| `A`/`B` = `NULL` **with an invalid type tag** | reachable & well-defined: the `default:` arm returns `0` before any deref | rows 1–5 pass `null`, `0x1` (misaligned), and `usize::MAX` pointers |
| `A`/`B` = `NULL` with a *valid* type tag | C dereferences unconditionally ⇒ **undefined behaviour / SIGSEGV**. Not a defined result, so asserting equality is meaningless; deliberately NOT exercised. Documented so the omission is explicit. | n/a (by design) |
| out-of-range enum values across FFI | `C2_TYPE` is passed as a 4-byte `int` (verified in disassembly: `cmpl $0x0,-0xc(%rbp)` / `cmpl $0x1,...`). Every `int` other than 0/1 falls to `default:`. | rows 1–5 sweep `-2147483648, -1, 2, 3, 255, 256, 0x10000, 0x7FFFFFFF`, i.e. one step past each end of the valid range plus wide/random values |
| enum value 2 = "one past the last variant" | `default:` ⇒ `0` | rows 1–5 |
| zero / oversized lengths | no length or count parameter exists anywhere in the API | n/a |
| unaligned / garbage struct contents | all 2^32 bit patterns are valid `float`s to the C; there is no validation. Signalling NaNs, quiet NaNs, ±inf and subnormals are therefore *valid* inputs and belong to Phase B. | `CONFIGS.md` rows 1–24 |
