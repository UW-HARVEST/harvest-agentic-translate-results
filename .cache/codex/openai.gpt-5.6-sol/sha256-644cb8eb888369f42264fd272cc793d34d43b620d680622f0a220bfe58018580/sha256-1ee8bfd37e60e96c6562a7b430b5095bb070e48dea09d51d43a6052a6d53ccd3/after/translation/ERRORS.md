# Error Surface

Mechanical searches covered `c_src/include/lib.h` and `c_src/src/lib.c` for
error returns, `return -1`, `return NULL`, assertions, null checks, range
checks, enums, and min/max constants.

The API returns `void` and contains no checked rejection, error return,
assertion, null check, length, enum, or documented valid range. Consequently,
there are no source-derived error rows.

The generic FFI boundaries requested by the verification protocol are tracked
below. Null pointers are not rejected by C; dereferencing them terminates the
calling process in this build, so these cases must run in child processes.

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `hsl_to_rgb` | `dest == NULL`, with non-null `src` and `src[1] == 0` | process terminates with `SIGSEGV` while writing `dest[0]` | [x] |
| 2 | `hsl_to_rgb` | `src == NULL`, with non-null `dest` | process terminates with `SIGSEGV` while reading `src[0]` | [x] |

Not applicable to this API: zero or oversized lengths (no length parameter),
one-past-range values (no documented input range is enforced), and invalid enum
discriminants (no enum parameter).
