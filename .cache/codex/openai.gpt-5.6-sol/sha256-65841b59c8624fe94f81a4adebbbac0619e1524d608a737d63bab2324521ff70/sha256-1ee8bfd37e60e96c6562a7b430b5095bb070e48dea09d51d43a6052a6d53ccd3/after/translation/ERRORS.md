# Error Surface

The following mechanical search was applied to all files under `c_src/include`
and `c_src/src`:

```text
RETURN_ERROR
return -1
return NULL
assert(...)
error
min/max
NULL comparisons
if/switch branches
#if/#ifdef branches
```

It found no rejection statements, error enums, assertions, null checks, range
checks, min/max constants, conditional branches, or compile-time branches.
`md5_digest` returns `void` and assumes both pointers satisfy the contract in
`lib.h`; invalid pointers invoke undefined behavior rather than a defined C
error result.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

Distinct defined C rejection paths: **0**

Generic length and enum boundaries do not apply: the API accepts no length or
enum parameters. Null and invalid pointers are outside the C function's defined
input domain, so they cannot be asserted as portable differential results.

## Generic Boundary Coverage

The applicable null cases are isolated in subprocesses. The C and Rust calls
produce identical process termination on this target.

| boundary | applicability and result | status |
|----------|--------------------------|--------|
| Null `m` | Outside defined pointer contract; C and Rust process rejection matches | [x] |
| Null `out` | Outside defined pointer contract; C and Rust process rejection matches | [x] |
| Zero/oversized length | Not applicable; no length parameter | [x] |
| Out-of-range enum | Not applicable; no enum parameter | [x] |
