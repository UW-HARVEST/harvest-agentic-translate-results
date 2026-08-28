# Error surface

The C source has no `assert`, error macro, error enum, `return -1`, or
`return NULL`. Its explicit input rejection/default paths are below. Conditions
that dereference a null pointer or index outside supplied storage are C
undefined behavior, not rejection paths, and therefore have no deterministic C
result to reproduce.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2MakeProxy` | `type` is not 0, 1, or 2 | no write to `*p` | [x] |
| 2 | `c2Collided` | `typeA` is not 0, 1, or 2 | returns `0` without dereferencing either shape | [x] |
| 3 | `c2Collided` | `typeA == 0` and `typeB` is not 0, 1, or 2 | returns `0` without dereferencing either shape | [x] |
| 4 | `c2Collided` | `typeA == 1` and `typeB` is not 0, 1, or 2 | returns `0` without dereferencing either shape | [x] |
| 5 | `c2Collided` | `typeA == 2` and `typeB` is not 0, 1, or 2 | returns `0` without dereferencing either shape | [x] |

## Boundary behavior

These are valid sentinel/boundary paths rather than rejection paths and are
covered by the configuration tests:

- `c2GJK`: null `ax_ptr`/`bx_ptr` select identity transforms; null
  `outA`/`outB`/`iterations` suppress writes; null `cache` disables caching.
- `c2GJK`: `use_radius == 0` disables radii; every other integer enables them.
- `c2Support`: `count <= 1` returns index 0 after reading `verts[0]`.
- `c2GJKSimplexMetric`, `c2D`, `c2Witness`, and `c2L`: unsupported `count`
  values take their documented C `default` branches.
- `c2Collided`: out-of-range enum values include `-1`, `3`, `INT_MIN`, and
  `INT_MAX`.
