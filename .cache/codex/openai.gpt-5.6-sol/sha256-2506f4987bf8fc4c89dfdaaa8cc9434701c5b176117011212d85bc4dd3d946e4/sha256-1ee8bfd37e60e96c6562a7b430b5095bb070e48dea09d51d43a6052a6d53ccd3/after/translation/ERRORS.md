# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
`assert`, `if`, `switch`, preprocessor branches, null checks, enums, and
min/max constants in `../c_src/include` and `../c_src/src`.

The C implementation contains no explicit rejection, error return, assertion,
range check, null check, enum, or length parameter. The mandatory generic null
pointer boundary is still recorded because `static_alias` dereferences its
pointer unconditionally.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `static_alias` | `outer == NULL`; line 30 evaluates `*outer` without a null check | process terminates with `SIGSEGV`; there is no return value | [x] |

Generic boundary applicability:

- Zero, oversized, and one-past-range lengths: not applicable; neither API has
  a length parameter.
- Out-of-range enums: not applicable; the public API defines no enum.
- `driver` accepts every `int` value for both parameters. `iterations <= 0` is
  a valid no-op path, not an error.
