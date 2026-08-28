# Error Surface

The following source scan was applied to `../c_src/include/` and
`../c_src/src/`:

```text
rg -n 'return\s+(-1|NULL)|RETURN_ERROR|assert\s*\(|if\s*\(|switch\s*\(|#if|#ifdef|NULL|ERROR|MIN|MAX|enum'
```

It found no rejection or error path. `flip_horizontal` returns `void` and the C
implementation has no error return, assertion, explicit null check, range
check, enum, or min/max constant.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|

There are therefore zero C rejection rows to check. Boundary calls that remain
defined without dereferencing pixel storage (zero dimensions, negative
dimensions that suppress the loops, extreme width with zero height, and a null
pixel pointer with height less than two) are covered by differential tests.
A null image pointer and dimensions that make the C pointer arithmetic leave
the supplied allocation invoke undefined behavior; they do not produce a C
error result and cannot be compared as error paths.

Phase C status: **complete** (zero rejection rows; defined boundary cases pass).
