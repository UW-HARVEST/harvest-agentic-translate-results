# Error Surface

Derived mechanically from every allocation result, explicit `NULL` return, and
public-API boundary in `../c_src/src/lib.c`. The function has no enum or numeric
length parameter, no documented numeric range, and no assertions.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `searchAndReplace` | `strstr(orig, search) == NULL` and the internal `strdup(orig)` allocation fails | returns `NULL` (the unchecked `strdup` result) | [x] |
| 2 | `searchAndReplace` | first match starts after byte zero (`inx_start > 0`) and the prefix `malloc(inx_start + 1)` fails | returns `NULL` | [x] |
| 3 | `searchAndReplace` | the `realloc` used to append a replacement fails | returns `NULL` | [x] |
| 4 | `searchAndReplace` | a later non-adjacent match exists (`inx_start2 > from`) and the `realloc` used to append the intervening gap fails | returns `NULL` | [x] |
| 5 | `searchAndReplace` | bytes remain after the final match (`from < orig_len && from > 0`) and the final suffix `realloc` fails | returns `NULL` | [x] |
| 6 | `searchAndReplace` | `orig == NULL` | process terminates with `SIGSEGV` while evaluating `strlen(orig)` | [x] |
| 7 | `searchAndReplace` | `search == NULL` | process terminates with `SIGSEGV` while evaluating `strlen(search)` | [x] |
| 8 | `searchAndReplace` | `value == NULL` | process terminates with `SIGSEGV` while evaluating `strlen(value)` | [x] |
| 9 | `searchAndReplace` | `search` is the empty C string (zero search length) | does not return: `strstr(..., "")` repeatedly finds the same position and the loop never advances | [x] |

Generic boundary accounting:

- Zero original length is valid and is covered in `CONFIGS.md`.
- Zero replacement length is valid and is covered in `CONFIGS.md`.
- Zero search length is row 9 above.
- There is no caller-supplied length, enum, or documented numeric range, so
  oversized lengths, one-past-range values, and invalid enum discriminants do
  not exist at this FFI boundary.
