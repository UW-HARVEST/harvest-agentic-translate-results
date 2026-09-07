# Error-surface table

Mechanically derived from `c_src/include/shared.h`, `c_src/src/read-alert.c`,
`c_src/src/file-queue.c`, and `c_src/src/driver.c`. Conditions joined by `||`
in one C rejection branch remain one row; separately returning branches are
separate rows.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `os_calloc` | `calloc(num, size)` returns `NULL` | writes `Memory allocation failed in os_calloc` to stderr and exits with `EXIT_FAILURE` | [x] |
| 2 | `os_realloc` | `realloc(ptr, new_size)` returns `NULL` | writes `Memory allocation failed in os_realloc` to stderr and exits with `EXIT_FAILURE` | [x] |
| 3 | `os_strdup` | input string pointer is `NULL` | writes `NULL string passed to os_strdup` to stderr and exits with `EXIT_FAILURE` | [x] |
| 4 | `os_strdup` | `strdup(str)` returns `NULL` | writes `Memory allocation failed in os_strdup` to stderr and exits with `EXIT_FAILURE` | [x] |
| 5 | `GetAlertData` | a second `** Alert` is read after state 2 and the backward `fseek(fp, -strlen(str), SEEK_CUR)` fails | frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 6 | `GetAlertData` | the state-1 date/location line has a `:` but has no space at or after that colon | `perror`, frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 7 | `GetAlertData` | the state-1 date/location branch finds `al_data->date != NULL`, `al_data->location != NULL`, or no `:` (`p == NULL`) | `perror`, frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 8 | `GetAlertData` | a `Rule: ` line has no first space after the rule number, or no second space after it | frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 9 | `GetAlertData` | a `Rule: ` line has no opening single quote after the parsed level field | frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 10 | `GetAlertData` | a `Rule: ` line has an opening single quote but no closing single quote | frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 11 | `GetAlertData` | `fgets` terminates and the stream is not simultaneously at EOF and parser state 2 (empty/pre-alert-only input, header-only input, or stream I/O error) | frees the partial alert, clears stream error/EOF, returns `NULL` | [x] |
| 12 | `Handle_Queue` via `Init_FileQueue`/`Read_FileMon` | opening `fileq->file_name` fails when `CRALERT_FP_SET` is clear | internal result `0`; `Init_FileQueue` returns `0`, while `Read_FileMon` sleeps once and returns `NULL` | [x] |
| 13 | `Handle_Queue` via `Init_FileQueue` | `CRALERT_READ_ALL` is clear and the supplied `CRALERT_FP_SET` stream pointer is `NULL` | internal result `0`; `Init_FileQueue` returns `0` because it rejects only negative results | [x] |
| 14 | `Handle_Queue` via `Init_FileQueue` | `CRALERT_READ_ALL` is clear and `fseek(fp, 0, SEEK_END)` fails | logs `(1116)`, closes and nulls `fp`, returns `-1`; `Init_FileQueue` returns `-1` | [x] |
| 15 | `Handle_Queue` via `Init_FileQueue` | `fp` is non-null and `fstat(fileno(fp), ...)` fails | logs `(1118)`, closes and nulls `fp`, returns `-1`; `Init_FileQueue` returns `-1` | [x] |
| 16 | `Read_FileMon` | initial `fileq->fp == NULL` and reopening via `Handle_Queue(fileq, 0)` does not return `1` | sleeps once and returns `NULL` | [x] |
| 17 | `Read_FileMon` | `fileq->fp` is still `NULL` after the initial reopen block | returns `NULL` | [x] |
| 18 | `Read_FileMon` | the first parse returns `NULL`, then reopening the derived queue name via `Handle_Queue(fileq, 0)` does not return `1` | sleeps once and returns `NULL` | [x] |
| 19 | `Read_FileMon` | all `timeout` retry parses return `NULL` (including `timeout == 0`) | returns `NULL` after the retry loop | [x] |
| 20 | `driver` | `Init_FileQueue` returns a negative result | writes `File queue initialization failed` to stderr and returns `NULL` | [x] |

## Generic FFI boundaries

The public headers mark pointer arguments `nonnull`; the C implementation does
not convert null pointers into error codes. Differential tests must therefore
compare the actual process outcome for null `file_queue`, `tm`, `FILE`, and
`alert_data` pointers. They must also cover zero and oversized allocator
lengths, `timeout == 0` and `UINT_MAX`, month values immediately outside
`0..=11`, and flag values containing every documented bit plus unknown bits.
