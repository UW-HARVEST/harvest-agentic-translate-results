# ERRORS.md — error-surface table

Every distinct rejection / error exit in the C sources, derived mechanically
from `grep -n "return\|goto\|exit(\|perror\|merror\|continue" c_src/src/*.c
c_src/include/*.h` and from every explicit range/NULL check and min/max
constant. One row per distinct rejection branch.

Constants that participate in rejections: `OS_MAXSTR 1024` (`fgets` size),
`MAX_FQUEUE 256` (`snprintf` bound on `file_name`), `LOG_LIMIT 100`
(`log_size < LOG_LIMIT`, always true because the increment is commented out),
`FQ_TIMEOUT 5` (`file_sleep` seconds), `EXIT_FAILURE 1`.

Flags: `CRALERT_MAIL_SET 0x001`, `CRALERT_EXEC_SET 0x002`,
`CRALERT_READ_ALL 0x004`, `CRALERT_READ_FAILED 0x008`, `CRALERT_FP_SET 0x010`.

| # | function | trigger (exact invalid input/condition) | expected C result | test |
|---|----------|------------------------------------------|-------------------|------|
| 1 | `os_calloc` (shared.h:15) | `calloc(num,size)` returns NULL — `os_calloc(SIZE_MAX, 1)` | `fprintf(stderr,"Memory allocation failed in os_calloc")` then `exit(1)`; process terminates with status 1 | `err_01_os_calloc_alloc_failure_exits_1` |
| 2 | `os_realloc` (shared.h:24) | `realloc(ptr,new_size)` returns NULL — `os_realloc(NULL, SIZE_MAX)` | `fprintf(stderr,"Memory allocation failed in os_realloc")` then `exit(1)` | `err_02_os_realloc_alloc_failure_exits_1` |
| 3 | `os_strdup` (shared.h:32) | `str == NULL` | `fprintf(stderr,"NULL string passed to os_strdup")` then `exit(1)` | `err_03_os_strdup_null_exits_1` |
| 4 | `os_strdup` (shared.h:37) | `strdup(str)` returns NULL (OOM) | `fprintf(stderr,"Memory allocation failed in os_strdup")` then `exit(1)` | not forceable without an allocator hook; branch is identical in shape to #1/#2 which are covered. Documented, not executed. |
| 5 | `GetAlertData` (read-alert.c:111) | second `** Alert` header seen while `_r == 2` **and** `fseek(fp,-strlen(str),SEEK_CUR)` fails — stream is a non-seekable pipe (`ESPIPE`) | `goto l_error` → `FreeAlertData` + `clearerr` → returns `NULL` | `err_05_second_header_on_pipe_fseek_fails` |
| 6 | `GetAlertData` (read-alert.c:121) | header line where `strstr(str+9, ":")` is NULL, e.g. `"** Alert 1234 mail"` (no colon at/after offset 9) | `continue` — header rejected, `_r` stays 0, `alertid` not set; if that is the whole file → final `feof && _r==2` false → `NULL` | `err_06_header_without_colon` |
| 7 | `GetAlertData` (read-alert.c:132) | header line with a colon but no space at/after `str+9`, e.g. `"** Alert:x\n"` | `continue`; `alertid` **is** already assigned (`os_realloc`+`strncpy`), `_r` stays 0 → eventually `NULL` | `err_07_header_colon_no_space` |
| 8 | `GetAlertData` (read-alert.c:139) | `flag & CRALERT_MAIL_SET` and the word after the first space is not `"mail"` | `continue` — whole alert skipped, `_r` stays 0 → `NULL` | `err_08_mail_set_but_not_mail` |
| 9 | `GetAlertData` (read-alert.c:166) | body line arrives with `_r < 1` (any line before the first `** Alert` header) | `continue` — line ignored | `err_09_body_before_header` |
| 10 | `GetAlertData` (read-alert.c:185) | `_r == 1` date/location line that contains `':'` but no `' '` at/after that colon, e.g. `"2006 Apr 13 16:15:17\n"` | `perror("date of location not NULL")` then `goto l_error` → `NULL` | `err_10_dateline_colon_without_space` |
| 11 | `GetAlertData` (read-alert.c:191) | `_r == 1` date/location line with **no** colon at all → `p == NULL` (`!p`) | `perror("date or location not NULL or p is NULL")` then `goto l_error` → `NULL` | `err_11_dateline_no_colon` |
| 11b | `GetAlertData` (read-alert.c:191) | same statement's `al_data->date \|\| al_data->location` sub-conditions | **unreachable**: `date`/`location` are only assigned immediately before `_r = 2`, and `_r` never returns to 1 (an `** Alert` line at `_r == 2` returns or errors). Documented as dead; `!p` (row 11) is the only live trigger. | — |
| 12 | `GetAlertData` (read-alert.c:218) | `Rule: ` line where, after `p = str+6`, the two successive `strchr(p,' ')` do not both succeed, e.g. `"Rule: 1000\n"` or `"Rule: 1000 x\n"` | `goto l_error` → `NULL` | `err_12_rule_missing_second_space` |
| 13 | `GetAlertData` (read-alert.c:227) | `Rule: ` line with both spaces but no `'\''` at/after that point, e.g. `"Rule: 1000 level 7 no quote\n"` | `goto l_error` → `NULL` | `err_13_rule_missing_open_quote` |
| 14 | `GetAlertData` (read-alert.c:239) | `Rule: ` line whose text after the first `'\''` has **no further** `'\''` (`strrchr(comment,'\'')==NULL`), e.g. `"Rule: 1 level 7 -> 'unterminated\n"` | `goto l_error` → `NULL` | `err_14_rule_missing_close_quote` |
| 15 | `GetAlertData` (read-alert.c:305) | loop ends (`fgets` NULL) with `_r != 2` — empty file, file with no header, header only, header+rejected date line | `feof && _r==2` false → `l_error` → `FreeAlertData` + `clearerr` → `NULL` | `err_15_eof_with_r_not_2` |
| 16 | `GetAlertData` (read-alert.c:104/305) | `fgets` returns NULL **without** EOF — read error: stream opened write-only (`fopen(...,"w")`), so `fgets` fails with `EBADF` and `feof` is 0 | `feof(fp)` false → `l_error` → `NULL` (and `clearerr` clears the error flag) | `err_16_read_error_not_eof` |
| 17 | `GetAlertData` (header, `nonnull`) | `fp == NULL` | declared `__attribute__((nonnull))`; `fgets(str,1024,NULL)` dereferences NULL → **SIGSEGV**. Undefined behaviour, no defined result to match. | documented, not executed (would kill the harness) |
| 18 | `FreeAlertData` (header, `nonnull`) | `al_data == NULL` | `nonnull`; `os_free(al_data->alertid)` dereferences NULL → **SIGSEGV**. UB. | documented, not executed |
| 19 | `Handle_Queue` (file-queue.c:79) via `Init_FileQueue` | `!(flags & CRALERT_FP_SET)` and `fopen(file_name,"r")` fails — no `alerts.log` in CWD | `return 0`; `Init_FileQueue` tests `< 0` so it still **returns 0 (success)** with `fileq->fp == NULL` | `err_19_init_missing_alerts_log_returns_0` |
| 20 | `Handle_Queue` (file-queue.c:86) via `Init_FileQueue` | `flags & CRALERT_FP_SET` (so no `fopen`) and caller-supplied `fileq->fp == NULL`, and `!(flags & CRALERT_READ_ALL)` | `return 0` → `Init_FileQueue` returns 0, `last_change` **not** updated | `err_20_fp_set_with_null_fp` |
| 21 | `Handle_Queue` (file-queue.c:89–93) via `Init_FileQueue` | `fseek(fp,0,SEEK_END) < 0` — `CRALERT_FP_SET` with a non-seekable pipe `FILE*`, `CRALERT_READ_ALL` clear | `merror(FSEEK_ERROR,file_name,errno,strerror(errno))` to stderr, `fclose(fp)`, `fp=NULL`, `return -1` → `Init_FileQueue` returns **-1** | `err_21_fseek_error_returns_minus1` |
| 22 | `Handle_Queue` (file-queue.c:99–103) via `Init_FileQueue` | `fstat(fileno(fp),...) < 0` — `CRALERT_FP_SET \| CRALERT_READ_ALL` with an `fmemopen` stream (`fileno` = -1 → `EBADF`) | `merror(FSTAT_ERROR,...)`, `fclose(fp)`, `fp=NULL`, `return -1` → `Init_FileQueue` returns **-1** | `err_22_fstat_error_returns_minus1` |
| 23 | `Init_FileQueue` (file-queue.c:125) | `p->tm_mon` outside `0..=11` indexes `static const char *s_month[12]` out of bounds | Out-of-bounds read of a `.data.rel.ro` array → `strncpy` from an arbitrary pointer. **Undefined behaviour.** Probed empirically on the reference build: `tm_mon` in `-3..=-1` and every value `>= 12` **faults (SIGSEGV)**; a few values in `-20..=-4` survive and copy junk into `mon`. The translation therefore reproduces the *unchecked* index (see the fix note below) instead of range-checking, so that it faults on the same inputs. | `err_23_tm_mon_out_of_range` (in-range: full struct parity for all 12 months; out-of-range: asserts C **and** Rust both `SIGSEGV` for every large-magnitude value) |
| 24 | `Init_FileQueue` (file-queue.c:123) | `p->tm_year + 1900` signed-overflow, e.g. `tm_year = INT_MAX` | signed overflow UB; gcc emits a wrapping `add`. Rust uses `wrapping_add` → identical bit pattern | `err_24_tm_year_overflow` |
| 25 | `driver` (driver.c:15–18) | `Init_FileQueue(&fq,&time,flags) < 0` (i.e. row 21/22 conditions reachable via `driver`) | `fprintf(stderr,"File queue initialization failed")`, `return NULL` | `err_25_driver_init_failure` — note: `driver` builds its own `fq` zeroed, so `CRALERT_FP_SET` gives `fp==NULL` → row 20 (`return 0`), so `Init_FileQueue` can never fail from `driver`; asserted that both return the *same* result for all flag values |
| 26 | `Read_FileMon` (file-queue.c:150–153) | `fileq->fp == NULL` and `Handle_Queue(fileq,0) != 1` (no `alerts.log`) | `file_sleep()` (5 s `select`) then `return NULL` | `err_26_readfilemon_no_queue` |
| 27 | `Read_FileMon` (file-queue.c:156–158) | `fileq->fp == NULL` after the retry succeeded returning 1 | `return NULL`. **Unreachable**: `Handle_Queue` only returns 1 with a non-NULL `fp` when it opened one, and returns 0 when `fp` is NULL. Documented as dead. | — |
| 28 | `Read_FileMon` (file-queue.c:171–174) | first `GetAlertData` returned NULL, then `Handle_Queue(fileq,0) != 1` — `alerts.log` deleted/renamed between the two calls | `file_sleep()` then `return NULL` | `err_28_readfilemon_file_vanishes` |
| 29 | `Read_FileMon` (file-queue.c:177–188) | `timeout` retries all yield NULL from `GetAlertData` | loop ends → `return NULL` (with `timeout` × 5 s of `file_sleep`) | `err_29_readfilemon_timeout_expires` (`timeout = 1`) |
| 30 | `GetAlertData` (read-alert.c:288–291) | syscheck alert whose `Integrity checksum changed for: '` line has an **empty** remainder → `strdup("")` then `filename[strlen-1]` = `filename[-1]` | out-of-bounds write one byte *before* the heap block. UB, replicated verbatim by the Rust (`wrapping_offset(len-1)`). Exercised but only the returned `filename` value is compared. | `err_30_syscheck_empty_filename` |
| 31 | `merror` (file-queue.c:24) | `err_template == NULL` | `snprintf(buf,256,NULL,...)` → UB (glibc prints `(null)`); probed and compared byte-for-byte via captured stderr where defined | `err_31_merror_null_template` |
| 32 | `merror` (file-queue.c:26) | formatted output longer than `sizeof(buffer)==256` — `file_name`/`err_msg` long enough to overflow | `snprintf` truncates at 255 chars + NUL; `fprintf(stderr,"%s\n",buffer)` prints the truncation | `err_32_merror_truncation` |
| 33 | `GetAlertData` | `flag` values with no valid flag bits / out-of-range enum-like ints: `-1`, `INT_MIN`, `INT_MAX`, `0x20`, `0xFFFF` (only `CRALERT_MAIL_SET` is ever tested) | only bit 0 matters; all other bits ignored → identical result | `err_33_getalertdata_out_of_range_flags` |
| 34 | `Init_FileQueue` / `driver` | `flags` out-of-range ints: `-1`, `INT_MIN`, `INT_MAX`, `0x20`, `0xFFFF`; only bits `0x004`/`0x010` are branched on and the raw value is stored in `fileq->flags` | raw `flags` stored verbatim; branch on bits 2 and 4 | `err_34_init_out_of_range_flags` |
| 35 | `GetAlertData` | input line at/over the `OS_MAXSTR`-1 = 1023 byte `fgets` limit, so a logical line is split across iterations | `fgets` reads at most 1023 bytes + NUL; the remainder is re-parsed as a fresh line | `cfg_*` long-line rows in CONFIGS.md + `err_35_oversized_line` |
| 36 | `Read_FileMon` (header, `nonnull`) | `fileq == NULL` or `p == NULL` | `nonnull`; NULL dereference → **SIGSEGV**. UB. | documented, not executed |
| 37 | `Init_FileQueue` (header, `nonnull`) | `fileq == NULL` or `p == NULL` | `nonnull`; NULL dereference → **SIGSEGV**. UB. | documented, not executed |
| 38 | `GetAlertData` (read-alert.c:126) | `z == 0` (header is `"** Alert :..."`, i.e. `strstr` hits at `str+9`) → `os_realloc(NULL, 1)`, `strncpy(dst,src,0)`, `alertid[0]='\0'` | `alertid` becomes the empty string `""` (not NULL) | `err_38_zero_length_alertid` |
| 39 | `GetAlertData` (read-alert.c:207/222/256/271) | `atoi` on non-numeric / overflowing / negative text for `rule`, `level`, `srcport`, `dstport` (e.g. `"99999999999999"`, `"-5"`, `"abc"`) | glibc `atoi` == `(int)strtol(p,NULL,10)`: clamps to `LONG_MAX/LONG_MIN` then truncates to `int`; `rule`/`level` are `unsigned int` so the bit pattern is reinterpreted | `err_39_atoi_extremes` |
| 40 | `Handle_Queue` (file-queue.c:107) | `fp == NULL` path skips `fstat`, so `last_change = f_status.st_mtime` copies the **stale/zeroed** `f_status` and still `return 1` — reachable when `CRALERT_FP_SET` and `CRALERT_READ_ALL` are both set and `fp == NULL` | `return 1` with `last_change` = previous (zeroed) `st_mtime` = 0 | `err_40_fp_set_read_all_null_fp` |

## Fix applied to the Rust during Phase C

`src/file_queue.rs::copy_mon` originally guarded the `s_month[p->tm_mon]`
lookup with `if (0..12).contains(&tm_mon)` and *skipped* the `strncpy` for
out-of-range months, on the theory that `mon` is unobservable. That was a
deliberate deviation from the C and it was wrong twice over:

* `mon` **is** observable — `Init_FileQueue` takes a caller-owned
  `file_queue *`, so the caller can read `fileq->mon` back;
* far more importantly, the C **traps** for `tm_mon >= 12` and for
  `tm_mon` in `-3..=-1`, and for *every* large-magnitude value (measured:
  `INT_MIN`, `±10^3…10^9`, `INT_MAX` all `SIGSEGV`). A range check makes the
  Rust return `0` normally on inputs where the C dies — i.e. the Rust would
  silently accept inputs the ground truth rejects with a fault.

`copy_mon` now performs the same unchecked
`*S_MONTH.0.as_ptr().offset(tm_mon as isize)` load the C does, so both
implementations fault on the same class of input. This also required changing
the table's Rust type from `[&CStr; 12]` (fat pointers, 16-byte stride) to
`[*const c_char; 12]` (thin pointers, 8-byte stride) so the out-of-bounds
address arithmetic matches the C's. In-range months (`0..=11`) are byte-identical
in both, which is what `err_23`/`cfg_37` assert.

## Boundary cases covered in addition to the table

* NULL pointers into `os_strdup` (row 3) and `merror` (row 31); NULL `FILE*`
  and NULL struct pointers documented as UB (rows 17, 18, 36, 37).
* Zero lengths: `os_calloc(0,0)`, `os_realloc(NULL,0)`, `os_strdup("")`,
  empty input file (row 15), zero-length `alertid` (row 38), zero-length
  syscheck filename (row 30), `timeout = 0`.
* Oversized lengths: `os_calloc(SIZE_MAX,1)` / `os_calloc(SIZE_MAX,SIZE_MAX)`
  (multiplication overflow) (row 1), `os_realloc(NULL,SIZE_MAX)` (row 2),
  1023/1024/4096-byte input lines (row 35), 300+ byte `merror` arguments
  (row 32), `file_name` longer than `MAX_FQUEUE` is impossible because
  `GetFile_Queue` always writes a fixed literal.
* One step past a documented range: `tm_mon = -1` and `tm_mon = 12`
  (row 23), `tm_year = INT_MIN/INT_MAX` (row 24), `tm_mday` extremes.
* Out-of-range "enum" ints across the FFI boundary: the `CRALERT_*` bit set is
  the only enum-like parameter; rows 33 and 34 pass `-1`, `INT_MIN`, `INT_MAX`,
  `0x20` (one bit past `CRALERT_FP_SET`) and `0xFFFF` (all defined bits plus
  undefined ones) to `GetAlertData`, `Init_FileQueue` and `driver`.

## Coverage accounting

41 rows enumerated (1–40 plus 11b). Seven are **not executable** and are
documented as such rather than tested:

| row | why not executed |
|-----|------------------|
| 4 | `strdup` OOM cannot be forced without an allocator hook; identical in shape to rows 1/2 which are covered |
| 11b | provably dead code (`date`/`location` can never be non-NULL while `_r == 1`) |
| 17, 18, 36, 37 | `__attribute__((nonnull))` parameters — passing NULL dereferences NULL in the C, so there is no defined result to match and the test process would die |
| 27 | provably dead code (`Handle_Queue` never returns 1 with a NULL `fp` on that path) |

The remaining **34 rows plus row 2b** (`realloc(p,0)` → NULL → `exit(1)`, found
while writing row 2) each have a dedicated passing differential test, giving the
35 `err_*` tests plus 4 extra generic-boundary tests = **39 tests, all passing**
under every feature combination and under both the debug and release cdylib.

Each error-path test asserts the **same specific rejection**, not merely "both
failed": the exact return sentinel (`NULL` / `0` / `-1`), the exact process exit
status (`Exited(1)`) *and* the exact `fprintf(stderr, ...)` bytes for the
`exit(EXIT_FAILURE)` paths, the exact `merror(...)` text including the
`errno`/`strerror` values for rows 21 and 22, the resulting `feof`/`ferror`
flags (proving `clearerr` ran), and the full `file_queue` contents.
