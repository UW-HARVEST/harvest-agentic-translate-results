# Error Surface

Mechanical review covered every `return`, `if`, `switch`, comparison, enum,
pointer use, assertion, and error-like token in `c_src/src/lib.c` and
`c_src/include/lib.h`.

The C library defines no error enum, length argument, min/max input constant,
assertion, explicit null check, or explicit range check. Its sole defined input
rejection is the `default` arm of `c2Collided`. Null `A` or `B` with a valid
shape type is not a C rejection path: it is an unchecked null dereference and
therefore undefined behavior, so it has no defined C result to compare.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2Collided` | `typeB` is not `C2_TYPE_CIRCLE` (0), `C2_TYPE_AABB` (1), or `C2_TYPE_CAPSULE` (2); includes -1, 3, `INT_MIN`, and `INT_MAX`, with null and non-null `A`/`B` | returns `0` without dereferencing either pointer | [x] |
