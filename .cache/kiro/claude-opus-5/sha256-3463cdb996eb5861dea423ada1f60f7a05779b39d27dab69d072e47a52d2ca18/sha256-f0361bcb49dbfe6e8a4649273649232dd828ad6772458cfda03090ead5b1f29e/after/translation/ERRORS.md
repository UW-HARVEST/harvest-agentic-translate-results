# ERRORS.md — Phase C error-surface table

Mechanically derived by grepping every rejection construct in `c_src/`:
`goto l_error`, `return (NULL)`, `return (-1)`, `return (0)`, `continue`
(silent rejection), `exit(EXIT_FAILURE)`, `perror`, `merror`, and every
explicit null / range / bound check. One row per distinct rejection branch.

`ALL` below = `CRALERT_MAIL_SET|EXEC_SET|READ_ALL|READ_FAILED|FP_SET` = `0x1F`.

| # | function | trigger (exact invalid input/condition) | expected C result | pass |
|---|----------|------------------------------------------|-------------------|-----|
| 1 | `os_strdup` | `str == NULL` (explicit `if (!str)`) | `fprintf(stderr,"NULL string passed to os_strdup")` then `exit(EXIT_FAILURE)` → exit code 1, no trailing newline | [x] `err01_os_strdup_null` |
| 2 | `os_calloc` | `calloc` returns NULL (`num=SIZE_MAX, size=SIZE_MAX`) | `fprintf(stderr,"Memory allocation failed in os_calloc")` then `exit(1)` | [x] `err02_os_calloc_oom` |
| 3 | `os_realloc` | `realloc` returns NULL (`ptr=NULL, new_size=SIZE_MAX`) | `fprintf(stderr,"Memory allocation failed in os_realloc")` then `exit(1)` | [x] `err03_os_realloc_oom` |
| 4 | `GetAlertData` | `_r==2` and a new `** Alert` line, but `fseek(fp,-strlen(str),SEEK_CUR)` fails (non-seekable `FILE*`, e.g. a pipe) | `goto l_error` → `FreeAlertData`, `clearerr`, return `NULL` | [x] `err04_pushback_fseek_fails_on_pipe` |
| 5 | `GetAlertData` | `_r==1` date line contains `':'` but no `' '` at/after it | `perror("date of location not NULL")` → `l_error` → `NULL` | [x] `err05_date_colon_without_space` |
| 6 | `GetAlertData` | `_r==1` date line contains **no** `':'` → `p` stays `NULL` → `!p` | `perror("date or location not NULL or p is NULL")` → `l_error` → `NULL` | [x] `err06_date_without_colon` |
| 7 | `GetAlertData` | `_r==1` and `al_data->date`/`->location` already non-NULL (same `!p` branch, first disjunct) | same `perror` → `l_error` → `NULL` (unreachable via the public API; `_r` never returns to 1) | [x] `err07_date_already_set_unreachable` |
| 8 | `GetAlertData` | `Rule: ` line where `strchr(p,' ')` chain yields `NULL` (e.g. `Rule: 1234` — no space after the id) | `goto l_error` → `NULL` | [x] `err08_rule_no_first_space` |
| 9 | `GetAlertData` | `Rule: ` line where the **second** `strchr(p,' ')` yields `NULL` (e.g. `Rule: 1234 level`) | `goto l_error` → `NULL` | [x] `err09_rule_no_second_space` |
| 10 | `GetAlertData` | `Rule: ` line with no `'` (apostrophe) after the level (e.g. `Rule: 1 level 5 -> no quote`) | `goto l_error` → `NULL` | [x] `err10_rule_no_quote` |
| 11 | `GetAlertData` | `Rule: ` line whose comment has an opening `'` but **no closing** `'` → `strrchr` NULL | `goto l_error` → `NULL` | [x] `err11_rule_unclosed_comment` |
| 12 | `GetAlertData` | `fgets` hits EOF while `_r != 2` (e.g. `_r==0`: file with no `** Alert` header) | falls through to `l_error` → `NULL` | [x] `err12_eof_r0` |
| 13 | `GetAlertData` | `fgets` hits EOF while `_r == 1` (header present, no date line follows) | falls through to `l_error` → `NULL` | [x] `err13_eof_r1` |
| 14 | `GetAlertData` | completely empty file (`fgets` NULL on first call, `_r==0`) | `l_error` → `NULL` | [x] `err14_empty_file` |
| 15 | `GetAlertData` | `FILE*` already at EOF / already in error state | `l_error` → `NULL` (and `clearerr` called) | [x] `err15_stream_already_exhausted` |
| 16 | `GetAlertData` | `** Alert` header with no `':'` after `str+9` → `strstr` NULL → `continue` (silent rejection, `_r` stays 0) | header ignored; alert never starts → eventually `NULL` | [x] `err16_header_no_colon` |
| 17 | `GetAlertData` | `** Alert` header where `strchr(p,' ')` after the id is NULL → `continue` | header ignored → `NULL` | [x] `err17_header_no_space_after_id` |
| 18 | `GetAlertData` | `flag & CRALERT_MAIL_SET` set but the token after the id is not `mail` → `continue` | header ignored → `NULL` | [x] `err18_mail_flag_rejects` |
| 19 | `GetAlertData` | `flag` containing bits with **no** defined `CRALERT_*` variant (e.g. `0xFFFF`, `-1`, `0x7FFFFFFF`, `INT_MIN`) — C `int` accepts any value | only `CRALERT_MAIL_SET` is tested; all other bits ignored. Must behave identically | [x] `err19_out_of_range_flags` |
| 20 | `Handle_Queue` (via `Init_FileQueue`) | `fopen(fileq->file_name,"r")` fails (`alerts.log` absent) | `return (0)`; `Init_FileQueue` returns `0` (**not** an error) | [x] `err20_init_missing_file` |
| 21 | `Handle_Queue` (via `Init_FileQueue`) | `flags & CRALERT_FP_SET` set, `fileq->fp == NULL`, `CRALERT_READ_ALL` clear → `if (!fileq->fp) return (0)` | `return (0)`; `Init_FileQueue` returns `0` | [x] `err21_fp_set_null_fp` |
| 22 | `Handle_Queue` | `fseek(fp,0,SEEK_END) < 0` (non-seekable fp supplied with `CRALERT_FP_SET`) | `merror(FSEEK_ERROR,…)` on stderr, `fclose`, `fp=NULL`, `return (-1)` → `Init_FileQueue` returns `-1` | [x] `err22_fseek_error` |
| 23 | `Handle_Queue` | `fstat(fileno(fp),…) < 0` (fp on a closed/invalid fd) | `merror(FSTAT_ERROR,…)`, `fclose`, `fp=NULL`, `return (-1)` | [x] `err23_fstat_error` |
| 24 | `Init_FileQueue` | `Handle_Queue(...) < 0` | `return (-1)` | [x] `err24_25_driver_init_failure` |
| 25 | `driver` | `Init_FileQueue(&fq,&time,flags) < 0` | `fprintf(stderr,"File queue initialization failed")` (no newline), `return NULL` | [x] `err24_25_driver_init_failure` |
| 26 | `Read_FileMon` | `fileq->fp == NULL` and `Handle_Queue(fileq,0) != 1` | `file_sleep()` (5 s `select`), `return (NULL)` | [x] `err26_read_filemon_queue_unavailable` |
| 27 | `Read_FileMon` | `fileq->fp == NULL` after the first `Handle_Queue` succeeded (`FP_SET` path) | `return (NULL)` immediately, no sleep | [x] `err27_second_null_fp_unreachable` |
| 28 | `Read_FileMon` | first `GetAlertData` NULL, then the re-`Handle_Queue(fileq,0) != 1` | `file_sleep()`, `return (NULL)` | [x] `err28_requeue_fails_midway` |
| 29 | `Read_FileMon` | `timeout` retries exhausted without an alert (`timeout == 0` → loop never runs) | `return (NULL)` | [x] `err29_34_timeout_exhausted` |
| 30 | `merror` | `err_template` producing > 255 bytes (long `file_name`) → `snprintf` truncation to `sizeof(buffer)==256` | stderr gets the 255-byte truncated text + `'\n'` | [x] `err30_merror_truncation` |
| 31 | `GetAlertData` filename fixup | line is exactly `Integrity checksum changed for: '` (33 bytes, empty remainder) → `al_data->filename[strlen(...) - 1]` with `strlen == 0` → index `(size_t)-1` | C writes one byte **before** the heap block (UB). Rust reproduces the same `wrapping_offset(-1)` write. Compared for identical observable `filename` (`""`) | [x] `err31_filename_underflow` |
| 32 | boundary | `GetAlertData` line longer than `OS_MAXSTR-1` (1023) → `fgets` splits it; the tail becomes a separate logical line | identical split/parse behaviour | [x] `err32_oversized_line_boundary` |
| 33 | boundary | `Init_FileQueue` / `Read_FileMon` `tm_mday`/`tm_year` extremes (`INT_MIN`, `INT_MAX`, `0`, `-1`) — **no** range check exists in C | stored verbatim; `year = tm_year + 1900` wraps on overflow | [x] `err33_tm_extremes` |
| 34 | boundary | `driver` `timeout == 0` and `timeout == 1` (`unsigned int` loop bound) | `0` → no retry loop; `1` → one retry + one 5 s sleep | [x] `err29_34_timeout_exhausted` |
| 35 | not-a-check | `__attribute__((nonnull))` on `Init_FileQueue`/`Read_FileMon`/`GetAlertData`/`FreeAlertData` is a **compiler hint only** — no runtime null check exists; passing NULL is UB in both | not testable as a *behaviour*; documented, deliberately not exercised | [x] `err35_nonnull_is_a_hint_only` |
| 36 | out-of-range index | `s_month[p->tm_mon]` with `tm_mon` outside `0..11` — C performs **no** validation, so it reads out of bounds of a `static const char *[12]` | UB with layout-dependent result; cannot be byte-matched across two different `.so` images. Documented; `0..11` is exercised exhaustively instead | [x] `err36_month_index_unchecked` |
