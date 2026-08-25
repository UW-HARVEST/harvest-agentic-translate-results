# Error Surface

Mechanical source scan:

```text
rg -n 'RETURN_ERROR|return[[:space:]]+-1|return[[:space:]]+NULL|assert|\
error|enum|MIN|MAX|if[[:space:]]*\(' c_src/src c_src/include
```

`tool_basename` contains no explicit rejection, range check, error return,
assertion, enum, length argument, or min/max constant. Therefore the C source
contributes no explicit error-surface rows.

The mandatory generic FFI boundary is tracked separately:

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| G1 | `tool_basename` | `path == NULL` | isolated child process terminates with `SIGSEGV` (signal 11) | [x] |

Length, range, and enum boundary cases are not applicable because the API has
no length, numeric range, or enum parameter. A zero-length C string is valid
and is covered in `CONFIGS.md`.
