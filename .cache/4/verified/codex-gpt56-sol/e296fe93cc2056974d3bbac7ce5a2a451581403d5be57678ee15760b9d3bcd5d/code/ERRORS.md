# Error Surface

Rows are derived from explicit failure/filter branches in `c_src/include/shared.h`
and `c_src/src/*.c`. Allocation failures terminate the process, parser failures
free the partial result and clear the stream error, and queue failures preserve
the C return sentinel shown below.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| E01 | `os_calloc` | `calloc(num, size)` returns `NULL` | prints `Memory allocation failed in os_calloc` and exits with `EXIT_FAILURE` | [x] |
| E02 | `os_realloc` | `realloc(ptr, new_size)` returns `NULL` | prints `Memory allocation failed in os_realloc` and exits with `EXIT_FAILURE` | [x] |
| E03 | `os_strdup` | input string pointer is `NULL` | prints `NULL string passed to os_strdup` and exits with `EXIT_FAILURE` | [x] |
| E04 | `os_strdup` | non-null input is valid but `strdup` returns `NULL` | prints `Memory allocation failed in os_strdup` and exits with `EXIT_FAILURE` | [x] |
| E05 | `GetAlertData` | a second `** Alert` starts after state 2, but rewinding that line with `fseek` fails | returns `NULL`, frees partial data, and clears stream error | [x] |
| E06 | `GetAlertData` | candidate `** Alert` line has no `:` after the prefix | rejects that candidate with `continue`; returns the next valid alert or `NULL` at incomplete EOF | [x] |
| E07 | `GetAlertData` | candidate header has `:` but no space after the alert id | rejects that candidate with `continue`; returns the next valid alert or `NULL` at incomplete EOF | [x] |
| E08 | `GetAlertData` | `CRALERT_MAIL_SET` is set and the text after the id's first space does not begin with `mail` | rejects that candidate with `continue`; returns the next matching alert or `NULL` | [x] |
| E09 | `GetAlertData` | state-1 date/location line contains `:` but no following space | prints `date of location not NULL`, then returns `NULL` and clears stream error | [x] |
| E10 | `GetAlertData` | state-1 date/location split leaves `p == NULL`, or date/location was already non-null | prints `date or location not NULL or p is NULL`, then returns `NULL` and clears stream error | [x] |
| E11 | `GetAlertData` | `Rule: ` line lacks the first or second required space before the level | returns `NULL`, frees partial data, and clears stream error | [x] |
| E12 | `GetAlertData` | `Rule: ` line has no opening `'` after the parsed level | returns `NULL`, frees partial data, and clears stream error | [x] |
| E13 | `GetAlertData` | rule comment has no closing `'` | returns `NULL`, frees partial data, and clears stream error | [x] |
| E14 | `GetAlertData` | input ends or `fgets` fails before parser state 2 is reached | returns `NULL`, frees partial data, and clears stream error | [x] |
| E15 | `Init_FileQueue` | non-`FP_SET` fixed path `alerts.log` cannot be opened | returns `0` with `fp == NULL` (the unavailable queue is not propagated as `-1`) | [x] |
| E16 | `Init_FileQueue` | `FP_SET` is set, `READ_ALL` is clear, and supplied `fp == NULL` | returns `0` with `fp == NULL` (internal queue result `0`) | [x] |
| E17 | `Init_FileQueue` | seek-to-end `fseek(fp, 0, SEEK_END)` fails | logs `(1116)`, closes and nulls `fp`, and returns `-1` | [x] |
| E18 | `Init_FileQueue` | `fstat(fileno(fp), ...)` fails | logs `(1118)`, closes and nulls `fp`, and returns `-1` | [x] |
| E19 | `Read_FileMon` | initial `fp == NULL` and reopening the queue does not return `1` | sleeps once and returns `NULL` | [x] |
| E20 | `Read_FileMon` | first parser call returns `NULL`, then queue reopening does not return `1` | sleeps once and returns `NULL` | [x] |
| E21 | `Read_FileMon` | no alert is obtained in `timeout` attempts after a successful reopen | returns `NULL` after exactly `timeout` sleep iterations | [x] |
| E22 | `driver` | `Init_FileQueue` returns `-1` | prints `File queue initialization failed` and returns `NULL` | [x] |

The headers annotate `GetAlertData`, `FreeAlertData`, `Init_FileQueue`, and
`Read_FileMon` arguments as `nonnull`. Null pointers to those parameters are
outside the C contract and cause undefined behavior rather than a defined C
error result; boundary tests compare their process-level behavior separately.

