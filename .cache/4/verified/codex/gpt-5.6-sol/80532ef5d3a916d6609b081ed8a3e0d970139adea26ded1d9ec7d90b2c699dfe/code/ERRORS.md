# Error Surface

Mechanically derived from the four independently failing terms of the
`parse_val` condition in `c_src/src/main.c:68`. `parse_val` is static, so each
condition is observable through the exported `main`.

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `main` (`parse_val`) | `endp == str`: `strtol` consumes no decimal digit (including empty input, only whitespace, sign-only, or a non-digit prefix) | prints `An error occurred\n`; returns `0` | [x] |
| E2 | `main` (`parse_val`) | `errno != 0`: decimal magnitude is outside the C `long` range and `strtol` sets `ERANGE` | prints `An error occurred\n`; returns `0` | [x] |
| E3 | `main` (`parse_val`) | `errno == 0`, conversion consumed a digit, and parsed `long < INT_MIN` | prints `An error occurred\n`; returns `0` | [x] |
| E4 | `main` (`parse_val`) | `errno == 0`, conversion consumed a digit, and parsed `long > INT_MAX` | prints `An error occurred\n`; returns `0` | [x] |

Generic FFI boundary audit: neither exported function accepts a pointer,
length, or enum. Therefore null pointers, oversized lengths, and invalid enum
discriminants are not representable in this API. Empty input, zero, `INT_MIN`,
and `INT_MAX` are covered explicitly by E1 and the valid-configuration table.
