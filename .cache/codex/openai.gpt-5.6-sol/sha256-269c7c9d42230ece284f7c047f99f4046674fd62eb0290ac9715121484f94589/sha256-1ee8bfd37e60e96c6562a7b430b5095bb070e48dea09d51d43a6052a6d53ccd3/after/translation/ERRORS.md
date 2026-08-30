# Error Surface

The following source search was applied to all C source and headers:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+(-1|NULL)|assert[[:space:]]*\(|\
error|ERROR|if[[:space:]]*\(|switch[[:space:]]*\(|#[[:space:]]*if|\
NULL|MIN|MAX|enum' ../c_src/include ../c_src/src
```

It finds no error return, error enum, assertion, range check, null check,
minimum/maximum constant, conditional, switch, or conditional-compilation
branch. Consequently, the C implementation has no explicit rejection rows.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| - | - | No explicit rejection path exists in the C source. | - |

## Generic FFI Boundaries

`driver` has no pointer, length, or enum argument. Its unsigned and signed
arguments occupy their full C integer domains; values beyond the bitfield
widths are valid and truncate, so they are covered in `CONFIGS.md`. Its
`bool` domain is exactly `false` and `true`.

The dynamically exported (but header-private) `print_foo` accepts one pointer.
It immediately dereferences the pointer without checking it. A null pointer is
therefore not a C error return; it terminates the isolated caller with
`SIGSEGV` on the target platform. Phase C verifies that boundary in child
processes so it cannot terminate the test runner.

- [x] All zero explicit C rejection rows are covered.
- [x] The generic null-pointer boundary matches exactly (`SIGSEGV` for both).
- [x] No length or enum boundary exists in either exported signature.
