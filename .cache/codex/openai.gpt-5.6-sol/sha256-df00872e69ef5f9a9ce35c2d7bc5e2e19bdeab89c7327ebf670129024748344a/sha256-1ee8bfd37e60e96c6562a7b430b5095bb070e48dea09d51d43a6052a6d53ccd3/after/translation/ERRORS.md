# Error Surface

Mechanical search:

```sh
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|if[[:space:]]*\(|switch|default:' ../c_src
```

The C library has no error enum, error macro, assertion, allocation failure,
length argument, or checked min/max range. These are all explicit rejection
branches in `c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] E1 | `f2` | `typeA` is neither `C2_TYPE_CIRCLE` (0) nor `C2_TYPE_AABB` (1) | return `0` without dereferencing either pointer |
| [x] E2 | `f2` | `typeA == C2_TYPE_CIRCLE` and `typeB` is neither 0 nor 1 | return `0` without dereferencing either pointer |
| [x] E3 | `f2` | `typeA == C2_TYPE_AABB` and `typeB` is neither 0 nor 1 | return `0` without dereferencing either pointer |
| [x] E4 | `f3` | `v2 == 0` | return `0` |

Pointer note: `f2` with valid tags, `f4`, `f11`, `f12`, and `f13` dereference
their pointer arguments without null checks. A null pointer on those paths is
undefined behavior in C, not an error return or rejection, so there is no C
result to put in the table. E1-E3 exercise null pointers safely because the C
control flow returns before dereferencing them.
