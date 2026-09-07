# Error surface

Mechanical scan:

```text
rg -n 'RETURN_ERROR|return\s+(-1|NULL)|return\b|assert\s*\(|if\s*\(|switch\s*\(|case\b|#ifdef|#if|NULL|MIN|MAX|<[=]?|>[=]?' ../c_src/include ../c_src/src ../c_src/CMakeLists.txt
```

The C source has no error-return statements, assertions, explicit range
checks, null checks, or min/max constants. Its sole rejection behavior is the
implicit no-op when the enum value matches no `switch` case.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `colourblind` | `Impairment` is any integer other than `0`, `1`, or `2` | returns `void` without reading or writing `R`, `G`, or `B` | [x] |

Generic FFI boundary probes required by Phase C, despite not being explicit C
rejection branches:

| # | function | boundary | expected C behavior on this build | verified |
|---|----------|----------|-----------------------------------|----------|
| G1 | `colourblind` | valid impairment with `R == NULL` | invalid dereference; child process terminates abnormally | [x] |
| G2 | `colourblind` | valid impairment with `G == NULL` | invalid dereference; child process terminates abnormally | [x] |
| G3 | `colourblind` | valid impairment with `B == NULL` | invalid dereference; child process terminates abnormally | [x] |

There are no length parameters, so zero-length and oversized-length probes do
not apply.
