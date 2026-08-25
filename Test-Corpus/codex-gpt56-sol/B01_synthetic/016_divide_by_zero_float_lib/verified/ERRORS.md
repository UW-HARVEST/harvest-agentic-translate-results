# Error Surface

Mechanically derived from the null check and guarded division branch in
`c_src/src/driver.c`. The C API returns `void`, so rejection is observable
through exact stdout bytes.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return without writing output | [x] |
| 2 | `good`, transitively `driver` | `!(fabs(data) > 0.000001)`, including finite `fabs(data) <= 0.000001` and NaN | Write `This would result in a divide by zero\n` instead of dividing; `good` first writes `50\n`, and `driver` continues its remaining calls | [x] |

No `RETURN_ERROR`, error return, error enum, `assert`, length parameter, or
public enum exists in the C source.
