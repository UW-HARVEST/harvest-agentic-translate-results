# Error surface

Mechanical scans used:

```text
rg -n 'RETURN_ERROR|return -1|return NULL|assert|if|switch|default|return 0' ../c_src
```

The C source has no `assert`, error enum, error-return macro, `-1`/`NULL`
error return, explicit numeric range rejection, or public length rejection.
The four explicit rejection branches are the invalid-enum defaults below.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|-|
| 1 | `c2Collided` | `typeA` is not `C2_TYPE_CIRCLE`, `C2_TYPE_AABB`, or `C2_TYPE_CAPSULE` | returns `0` without dereferencing either shape | [x] |
| 2 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is outside the enum | returns `0` without dereferencing either shape | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is outside the enum | returns `0` without dereferencing either shape | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is outside the enum | returns `0` without dereferencing either shape | [x] |

`c2MakeProxy` also accepts an out-of-range enum, but does not reject it: its
switch has no `default`, so it returns `void` and leaves the proxy untouched.
That behavior is covered as a valid observable configuration in `CONFIGS.md`.

Unchecked pointer preconditions (`c2BBVerts`, valid-type `c2MakeProxy`,
`c2GJKSimplexMetric`, `c22`, `c23`, `c2D`, `c2Witness`, `c2L`, and
valid-type `c2Collided`) have C undefined behavior for null pointers; the C
code defines no error result to compare. `c2GJK` explicitly accepts null
transform, output, iteration, and cache pointers, which are valid
configurations covered in `CONFIGS.md`.
