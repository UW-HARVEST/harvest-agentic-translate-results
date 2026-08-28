# Error Surface

Mechanical scans covered all files under `../c_src/include` and
`../c_src/src` for error-return statements/macros, assertions, conditional
branches, null checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | Covered |
|---|----------|----------------------------------------------|-------------------|---------|

There are no explicit rejection paths in the C source. `next_double` accepts a
pointer to a `cn_rnd_t` and dereferences it without a null check. A null or
otherwise invalid pointer therefore has undefined behavior rather than a
defined C error result. The API has no length or enum parameters, so zero or
oversized lengths and out-of-range enum values are not applicable.

The generic null-pointer boundary is covered by
`generic_null_pointer_behavior_matches`, which invokes each shared object's
export in a separate subprocess and compares its termination signal.
