# Error Surface

Mechanical scan:

```sh
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|switch|default:|if[[:space:]]*\(' \
  c_src/include c_src/src
```

The C source has no `RETURN_ERROR`, `return -1`, `return NULL`, `assert`,
explicit null rejection, or public min/max range rejection. Required pointer
arguments are dereferenced and are therefore C undefined behavior when null;
they are not error results. Optional pointers in `c2GJK` are valid
configurations and are listed in `CONFIGS.md`.

The complete explicit rejection surface is the out-of-range `C2_TYPE` switch
handling below.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2MakeProxy` | `type` is not `C2_TYPE_CIRCLE` (0), `C2_TYPE_AABB` (1), or `C2_TYPE_CAPSULE` (2) | returns `void`; does not read `shape` or write `p` | [x] |
| 2 | `c2Collided` | `typeA` is not 0, 1, or 2 (regardless of `typeB`) | returns `0` without reading either shape | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` is not 0, 1, or 2 | returns `0` without reading either shape | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` is not 0, 1, or 2 | returns `0` without reading either shape | [x] |
| 5 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` is not 0, 1, or 2 | returns `0` without reading either shape | [x] |

Out-of-range simplex counts are not rejected: `c2GJKSimplexMetric`, `c2D`,
`c2Witness`, and `c2L` deliberately take their `default` result branches.
Those branches are valid-path configurations in `CONFIGS.md`.

