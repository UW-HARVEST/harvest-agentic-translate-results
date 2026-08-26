# Error Surface

The public API has no returned error code. `driver` rejects input by writing
`An error occurred\n` to stdout and returning without changing the global
house. These rows split every false term in the acceptance condition at
`c_src/src/driver.c:70`; the two `errno` rows distinguish the two observable
`strtol` range directions.

| # | function | trigger (the exact invalid input/condition) | expected C result | covered |
|---|----------|---------------------------------------------|-------------------|---------|
| 1 | `driver` | `endp == str`: no base-10 conversion, including empty, whitespace-only, lone-sign, or nonnumeric input | Writes exactly `An error occurred\n`; no house update | [x] |
| 2 | `driver` | `errno != 0`: positive base-10 value overflows `long` | Writes exactly `An error occurred\n`; no house update | [x] |
| 3 | `driver` | `errno != 0`: negative base-10 value underflows `long` | Writes exactly `An error occurred\n`; no house update | [x] |
| 4 | `driver` | `errno == 0`, conversion consumed input, and `tmp < INT_MIN` | Writes exactly `An error occurred\n`; no house update | [x] |
| 5 | `driver` | `errno == 0`, conversion consumed input, and `tmp > INT_MAX` | Writes exactly `An error occurred\n`; no house update | [x] |

## Generic FFI Boundaries

- `driver(NULL)` is not rejected by C; it invokes undefined behavior through
  `strtol`. Differential coverage must isolate this call in child processes.
- There are no pointer-length pairs, enum parameters, or documented numeric
  option ranges in this API.
- Zero is valid for both `run(0)` and `driver("0")`.
- One-past-range decimal values are covered by rows 4 and 5.
- Oversized decimal strings are covered by rows 2 and 3.
