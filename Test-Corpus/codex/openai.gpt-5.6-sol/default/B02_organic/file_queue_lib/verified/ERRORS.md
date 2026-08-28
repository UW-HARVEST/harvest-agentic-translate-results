# Error Surface

Derived mechanically from `return NULL`, negative returns, `goto l_error`,
allocation checks, nonnull declarations, and explicit failure checks in
`c_src/include/` and `c_src/src/`. There are no C enums or `assert` calls.
Flags are integer bitmasks, so unknown and out-of-range flag values are valid
ignored-bit configurations covered in `CONFIGS.md`.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|----------------------------------------------|-------------------|---|
| 1 | `os_calloc` | `calloc(num, size) == NULL` (forced with an overflowing/oversized allocation) | writes `Memory allocation failed in os_calloc`, exits `EXIT_FAILURE` | [x] |
| 2 | `os_realloc` | `realloc(ptr, new_size) == NULL` (forced with `new_size == SIZE_MAX`) | writes `Memory allocation failed in os_realloc`, exits `EXIT_FAILURE` | [x] |
| 3 | `os_strdup` | `str == NULL` | writes `NULL string passed to os_strdup`, exits `EXIT_FAILURE` | [x] |
| 4 | `os_strdup` | underlying `strdup(str) == NULL` | writes `Memory allocation failed in os_strdup`, exits `EXIT_FAILURE` | [x] |
| 5 | `GetAlertData` | a second `** Alert` starts after a complete header, but rewinding that line with `fseek` fails | frees partial alert, clears stream error, returns `NULL` | [x] |
| 6 | `GetAlertData` | first line after accepted alert marker contains `:` but no space at/after it | frees partial alert, clears stream error, returns `NULL` | [x] |
| 7 | `GetAlertData` | first line after accepted alert marker contains no `:` (`p == NULL` in date/location check) | frees partial alert, clears stream error, returns `NULL` | [x] |
| 8 | `GetAlertData` | `Rule: ` line lacks either of the two required spaces after the rule number (`p == NULL`) | frees partial alert, clears stream error, returns `NULL` | [x] |
| 9 | `GetAlertData` | `Rule: ` line has required spaces but no opening `'` in/after the level field | frees partial alert, clears stream error, returns `NULL` | [x] |
| 10 | `GetAlertData` | `Rule: ` line has an opening `'` but no closing `'` | frees partial alert, clears stream error, returns `NULL` | [x] |
| 11 | `GetAlertData` | EOF/read failure occurs before state 2 (no accepted alert plus date/location pair) | frees partial alert, clears stream error, returns `NULL` | [x] |
| 12 | `Init_FileQueue` | supplied/opened stream cannot seek to end while `CRALERT_READ_ALL` is clear | closes stream, sets `fp = NULL`, returns `-1` | [x] |
| 13 | `Init_FileQueue` | `fstat(fileno(fp))` fails | closes stream, sets `fp = NULL`, returns `-1` | [x] |
| 14 | `Read_FileMon` | `fp == NULL` and reopening `alerts.log` does not return queue status 1 | sleeps once, returns `NULL` | [x] |
| 15 | `Read_FileMon` | initial `GetAlertData` returns `NULL`, then reopening `alerts.log` does not return queue status 1 | sleeps once, returns `NULL` | [x] |
| 16 | `Read_FileMon` | no event is obtained before `i == timeout` (including `timeout == 0`) | returns `NULL` | [x] |
| 17 | `driver` | `Init_FileQueue` returns negative (for example `alerts.log` is an unseekable directory stream) | writes `File queue initialization failed`, returns `NULL` | [x] |
| 18 | `FreeAlertData` | `al_data == NULL`, violating its C `nonnull` contract | process terminates from invalid pointer dereference | [x] |
| 19 | `GetAlertData` | `fp == NULL`, violating its C `nonnull` contract | process terminates from invalid stream use | [x] |
| 20 | `Init_FileQueue` | `fileq == NULL` or `p == NULL`, violating its C `nonnull` contract | process terminates from invalid pointer dereference | [x] |
| 21 | `Read_FileMon` | `fileq == NULL`, or `p == NULL` on the refresh path, violating its C `nonnull` contract | process terminates from invalid pointer dereference | [x] |
| 22 | `merror` | `err_template == NULL` | process terminates while `snprintf` dereferences the invalid format pointer | [x] |

`tm_mon` is used directly as an array index with no C range check. Values
outside 0 through 11 invoke undefined behavior in the reference and therefore
do not define an error result that a differential test can require.
