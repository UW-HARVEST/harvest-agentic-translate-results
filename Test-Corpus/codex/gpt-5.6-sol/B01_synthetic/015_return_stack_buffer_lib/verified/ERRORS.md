# Error Surface

The complete C source and public header were searched for `return`, `NULL`,
`assert`, conditionals, range/min/max checks, error macros, and enums. There are
no error codes, error enums, assertions, explicit ranges, or length parameters.
The sole explicit input rejection is the null-pointer guard below.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `printLine` | `line == NULL` at `driver.c:30` | Return `void` without writing any bytes to stdout |

Generic FFI boundary applicability:

- Null pointers: applicable only to `printLine`, covered by row 1.
- Zero or oversized lengths: not applicable; no exported function accepts a
  length.
- Out-of-range enums: not applicable; no exported function accepts an enum.
- `driver` accepts the full C `int` domain; zero and nonzero are both valid.
- A non-null pointer that is invalid or lacks a terminating NUL invokes C
  undefined behavior and is not an input the C implementation rejects.
