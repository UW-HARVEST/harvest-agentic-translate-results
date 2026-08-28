# Error Surface

The following mechanical source scan was used:

```sh
rg -n 'RETURN_ERROR|return\s+-1|return\s+NULL|assert\s*\(|NULL|enum|\
#if|#ifdef|#ifndef|\b(min|max|MIN|MAX)\b|\bif\s*\(|\bswitch\s*\(' \
  ../c_src/include ../c_src/src
```

The C source has no error-return macro, `return -1`, `return NULL`, assertion,
explicit range check, null check, error enum, length parameter, or min/max
constant. Its `if` statements only classify strings and its `switch` maps
multiplier levels, with `0xDEAD` as a normal default result.

## Source-Defined Rejections

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| - | - | No source-defined rejection paths | - |

## Generic FFI Boundaries

The mandatory generic checks add the only pointer boundary. It is not a C
rejection path: passing null to `strcmp` is undefined behavior. On the target
x86-64/glibc platform, it deterministically terminates the process with
`SIGSEGV` (shell status 139), so it must be tested in isolated child processes.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|
| G1 | `classify_mode` | `mode == NULL` | process terminates with `SIGSEGV` | [x] |

There are no length-taking APIs or enum-taking APIs, so zero/oversized lengths
and out-of-range enum discriminants are not applicable. Scalar zero, extreme,
and out-of-switch-range values return ordinary results and are covered by
`CONFIGS.md`.

