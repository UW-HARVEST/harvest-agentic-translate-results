# Error-surface table

Mechanically derived from the four rejecting terms in
`src/driver.c:70`:

```c
endp != str && errno == 0 && tmp >= INT_MIN && tmp <= INT_MAX
```

`parse_val` is private, so each rejection is observable through `driver`,
which prints `An error occurred\n` and returns `void`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `driver` / `parse_val` | `endp == str`: `strtol` consumes no decimal digits | stdout is exactly `An error occurred\n`; returns `void` | [x] |
| 2 | `driver` / `parse_val` | `errno != 0`: decimal magnitude overflows or underflows C `long` | stdout is exactly `An error occurred\n`; returns `void` | [x] |
| 3 | `driver` / `parse_val` | `tmp < INT_MIN` while the value remains representable as C `long` | stdout is exactly `An error occurred\n`; returns `void` | [x] |
| 4 | `driver` / `parse_val` | `tmp > INT_MAX` while the value remains representable as C `long` | stdout is exactly `An error occurred\n`; returns `void` | [x] |

## Generic ABI boundaries

These are tested in Phase C even though they are not explicit rejection
branches:

| boundary | applicability / C behavior | status |
|----------|----------------------------|--------|
| Null `driver` pointer | Passed to `strtol`; on this platform both shared objects terminate by the same signal | [x] |
| Zero length | No length parameter exists; the equivalent empty C string is covered by row 1 | [x] |
| Oversized length | No length parameter exists; a 4096-digit numeric string is covered by row 2 | [x] |
| One past documented range | No documented enum/range parameter; `INT_MIN - 1` and `INT_MAX + 1` are covered by rows 3-4 | [x] |
| Out-of-range enum | No enum parameter exists in either exported function | [x] |
