# Error Surface

Mechanical source scan covered `include/lib.h` and `src/lib.c` for error-return
macros/statements, `assert`, `if`, `switch`, null checks, explicit range checks,
enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no explicit rejection or error paths in the C source.

`hdr_bitrate` unconditionally reads `h[1]` and `h[2]`, then indexes fixed
arrays. A null/short/invalid pointer, layer selector 0, or bitrate index 15
therefore invokes C undefined behavior rather than returning an error or
sentinel. The API has no length or enum parameter, so zero/oversized lengths
and out-of-range enum values are inapplicable. Undefined behavior is not
invented as an error-surface row because the C source does not define a result
to match.

The generic boundary probes are isolated in subprocesses. In this build, null
pointers terminate both shared-library callers with status 139; layer selector
0 and bitrate index 15 both return `0` from both shared libraries. These are
observations of this build, not extra C error contracts.

Completion: [x] every explicit C rejection row is covered (zero rows).
