# Error Surface

The C source has no explicit error return, assertion, null/range check,
pointer/length parameter, or enum parameter. Its only input rejection occurs
inside `scanf("%f", &x)`. `main` ignores the conversion count, so the
initialized positive zero is printed in both rejection cases and `main`
returns zero.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `main` | The next non-whitespace input byte cannot begin a `%f` conversion, so `scanf` returns `0`. | Print native bytes of `0.0f` as lowercase hex plus newline; return `0`; leave the mismatching byte unread. [x] |
| 2 | `main` | EOF occurs before the first `%f` conversion, so `scanf` returns `EOF`. | Print native bytes of `0.0f` as lowercase hex plus newline; return `0`. [x] |

Generic FFI boundaries are not applicable to `driver(float)` or `main(void)`:
neither accepts pointers, lengths, or enums.
