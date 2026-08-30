# ERRORS.md — Phase C error-surface table

Derived mechanically from the C source, not from docs or assumptions.

## Mechanical grep evidence

```
grep -nE 'return|assert|NULL|errno|RETURN_ERROR|exit|abort|==|!=' c_src/src/driver.c
  -> src/driver.c:26:#include <stdio.h>
  -> src/driver.c:27:#include <string.h>
  -> src/driver.c:30:    for (int i = 0; i < len; i++)     # loop bound, not a rejection

grep -nE '#(if|ifdef|ifndef|else|elif)' c_src/src/driver.c   -> (none)
grep -nE '\b(if|switch|case|goto)\b'    c_src/src/driver.c   -> (none)
```

Findings, enumerated exhaustively:

- **No** `return` statement anywhere (both functions are `void`).
- **No** error-return macro, no error enum, no sentinel value, no `errno` use.
- **No** `assert`, no `abort`, no `exit`.
- **No** null-pointer check, **no** range check, **no** min/max constant.
- The only conditional in the entire library is the `i < len` loop bound at
  `src/driver.c:30`, which is iteration control, not input rejection.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| — | — | *(none — the library has no reachable rejection path; see below)* | — | n/a |

The table is **empty by derivation, not by omission**. `driver`'s only parameter
is a by-value `int`. Every one of the 2^32 possible `int` bit patterns is a
*valid* input that the C accepts unconditionally: it `memcpy`s the 4 bytes and
prints them. There is no value of `x` for which the C code rejects, errors,
asserts, or behaves differently in kind. Consequently every input belongs in
`CONFIGS.md` (the valid-path table), and there are zero error-path rows.

## Generic FFI boundaries still covered (per Phase C instructions)

These are exercised in `tests/differential.rs` even though the C has no explicit
check for them, because the task requires covering the generic boundaries every
C API has:

| # | boundary | why it is / is not applicable here | test |
|---|----------|-------------------------------------|------|
| G1 | `INT_MIN` (`-2147483648`) — one step past the negative end of the range | valid input; sign bit set, no `memcpy`/format divergence allowed | `boundary_extremes` |
| G2 | `INT_MAX` (`2147483647`) — the positive end of the range | valid input | `boundary_extremes` |
| G3 | `0` / zero-length-ish input | `len` is hardcoded `sizeof(int)`, never 0; `x == 0` prints `00000000` | `boundary_extremes` |
| G4 | `-1` (all bits set) | worst case for sign-extension bugs in `%02x` promotion | `boundary_extremes` |
| G5 | out-of-range **enum** value across FFI | **N/A** — the public API declares no enum and no pointer parameter. Documented rather than skipped silently. | n/a |
| G6 | **null pointer** argument | **N/A** — `driver(int)` takes no pointer. The only pointer in the library, `print_hex`'s `p`, is `static` (not in the ABI) and is always fed the address of a live local `raw[4]`, so a caller cannot supply null. | n/a |
| G7 | oversized / negative `len` | **N/A at the ABI** — `len` is not caller-controlled; `driver` always passes `sizeof(raw) == 4`. | n/a |
| G8 | signed-char sign extension | `raw` is `char` (signed on x86-64) but cast to `unsigned char*` before printing; bytes >= 0x80 must print as `80`..`ff`, never `ffffff80` | `high_bytes_no_sign_extension` |

- [x] Every row in this table has a passing differential test (or is a
      documented, justified N/A with the reason grounded in the C signature).
