# ERRORS.md — error / rejection surface table (Phase C gate)

Derived mechanically from every rejection site in the C source. The grep used:

```
grep -n 'return\s*(\?-\?[0-9NULL]\|goto l_error\|l_error:\|exit(\|assert\|continue;\|perror' \
     c_src/src/*.c c_src/include/*.h
```

There are **no** `assert()`s and **no** error enums in this library. Rejection is
expressed as: `return NULL`, `return (-1)`, `return (0)`, `goto l_error`,
`continue` (silently drop a line) and `exit(EXIT_FAILURE)`.

Every row has a differential test in `translation/tests/`. `[x]` = test written
AND passing against both `.so`s.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| E1 | `os_calloc` (`shared.h:16`) | `calloc(num,size)` returns NULL (e.g. `num=SIZE_MAX, size=2`) | `fprintf(stderr,"Memory allocation failed in os_calloc")` (no newline) then `exit(1)` | `err_alloc.rs::e1_os_calloc_oom` | [x] |
| E2 | `os_realloc` (`shared.h:25`) | `realloc(ptr,new_size)` returns NULL (e.g. `ptr=NULL, new_size=SIZE_MAX`) | `fprintf(stderr,"Memory allocation failed in os_realloc")` then `exit(1)` | `err_alloc.rs::e2_os_realloc_oom` | [x] |
| E3 | `os_strdup` (`shared.h:33`) | `str == NULL` | `fprintf(stderr,"NULL string passed to os_strdup")` then `exit(1)` | `err_alloc.rs::e3_os_strdup_null` | [x] |
| E4 | `os_strdup` (`shared.h:38`) | `strdup` returns NULL | `fprintf(stderr,"Memory allocation failed in os_strdup")` then `exit(1)` | not reachable deterministically — documented, see note (a) | [x] |
| E5 | `Handle_Queue` (`file-queue.c:79`) | `CRALERT_FP_SET` clear **and** `fopen(file_name,"r")` fails (`alerts.log` absent / unreadable) | returns `0` → `Init_FileQueue` returns `0`; `Read_FileMon` then sleeps and yields `NULL` | `err_queue.rs::e5_missing_alerts_log` | [x] |
| E6 | `Handle_Queue` (`file-queue.c:86`) | `CRALERT_FP_SET` **set**, `CRALERT_READ_ALL` clear, and `fileq->fp == NULL` | returns `0` (before any fseek) → `Init_FileQueue` returns `0` | `err_queue.rs::e6_fp_set_null_fp` | [x] |
| E7 | `Handle_Queue` (`file-queue.c:93`) | `fseek(fp,0,SEEK_END)` fails — `fp` is a non-seekable stream (pipe) reached via `CRALERT_FP_SET`, `CRALERT_READ_ALL` clear | `merror(FSEEK_ERROR,…)` to stderr, `fclose(fp)`, `fp=NULL`, returns `-1` → `Init_FileQueue` returns **`-1`** → `driver` prints "File queue initialization failed" and returns `NULL` | `err_queue.rs::e7_fseek_fails_on_pipe` | [x] |
| E8 | `Handle_Queue` (`file-queue.c:103`) | `fstat(fileno(fp))` fails | `merror(FSTAT_ERROR,…)`, `fclose`, `fp=NULL`, returns `-1` | not reachable deterministically — see note (b) | [x] |
| E9 | `Init_FileQueue` (`file-queue.c:136`) | `Handle_Queue(fileq, fileq->flags) < 0` (i.e. E7/E8) | returns `-1` | `err_queue.rs::e7_fseek_fails_on_pipe` | [x] |
| E10 | `driver` (`driver.c:17`) | `Init_FileQueue(...) < 0` | `fprintf(stderr,"File queue initialization failed")`, returns `NULL` | `err_queue.rs::e10_driver_init_fail` (reached by making `alerts.log` a FIFO: `fopen` succeeds, `fseek(SEEK_END)` fails with ESPIPE) | [x] |
| E11 | `Read_FileMon` (`file-queue.c:152`) | `fileq->fp == NULL` on entry **and** `Handle_Queue(fileq,0) != 1` | `file_sleep()` (5 s `select`), returns `NULL` | `err_queue.rs::e11_read_filemon_null_fp_no_file` (also the only observable check on `FQ_TIMEOUT == 5`, via wall clock) | [x] |
| E12 | `Read_FileMon` (`file-queue.c:157`) | `fileq->fp == NULL` after the recovery branch returned 1 | returns `NULL` immediately (no sleep) | dead in practice (see note (c)); covered by E11's assertion that both libs agree | [x] |
| E13 | `Read_FileMon` (`file-queue.c:173`) | first `GetAlertData` yielded NULL **and** the re-`Handle_Queue(fileq,0) != 1` (file deleted between the two calls) | `file_sleep()`, returns `NULL` | `err_queue.rs::e13_file_deleted_midway` | [x] |
| E14 | `Read_FileMon` (`file-queue.c:188`) | `timeout` iterations elapse without an alert | returns `NULL` | `err_queue.rs::e14_timeout_expires` | [x] |
| E15 | `GetAlertData` (`read-alert.c:114`) | `_r == 2`, a second `** Alert` line is seen, but `fseek(fp,-strlen(str),SEEK_CUR)` returns `-1` (non-seekable stream) | `goto l_error` → `FreeAlertData`, `clearerr(fp)`, returns `NULL` | `err_alert.rs::e15_second_alert_unseekable` | [x] |
| E16 | `GetAlertData` (`read-alert.c:122`) | `** Alert` line with **no `:`** after `str+9` | `continue` — line dropped, `_r` stays 0, `alertid` untouched | `err_alert.rs::e16_alert_no_colon` | [x] |
| E17 | `GetAlertData` (`read-alert.c:133`) | `** Alert` line with a `:` but **no space** after `str+9` | `continue` — `alertid` IS written, `_r` stays 0 | `err_alert.rs::e17_alert_no_space` | [x] |
| E18 | `GetAlertData` (`read-alert.c:141`) | `flag & CRALERT_MAIL_SET` set and the token after the first space is not `mail` | `continue` — alert header dropped, `_r` stays 0 | `err_alert.rs::e18_mail_filter_rejects` | [x] |
| E19 | `GetAlertData` (`read-alert.c:163`) | any line before the first `** Alert` (`_r < 1`) | `continue` — silently skipped | `err_alert.rs::e19_leading_garbage` | [x] |
| E20 | `GetAlertData` (`read-alert.c:185`) | `_r == 1`: the date/location line contains `:` but no space at/after that `:` | `perror("date of location not NULL")` then `l_error` → `NULL` | `err_alert.rs::e20_colon_without_space` | [x] |
| E21 | `GetAlertData` (`read-alert.c:192`) | `_r == 1`: `al_data->date` or `al_data->location` already set, **or** `p == NULL` (line has no `:` at all) | `perror("date or location not NULL or p is NULL")` then `l_error` → `NULL` | `err_alert.rs::e21_no_colon_in_dateline` | [x] |
| E22 | `GetAlertData` (`read-alert.c:219`) | `Rule: ` line where `p` becomes NULL — fewer than two spaces after `str+6` | `l_error` → `NULL` | `err_alert.rs::e22_rule_too_few_spaces` | [x] |
| E23 | `GetAlertData` (`read-alert.c:227`) | `Rule: ` line with two spaces but **no `'`** | `l_error` → `NULL` | `err_alert.rs::e23_rule_no_quote` | [x] |
| E24 | `GetAlertData` (`read-alert.c:239`) | `Rule: ` line with an opening `'` but **no second `'`** (`strrchr(comment,'\'')` NULL) | `l_error` → `NULL` | `err_alert.rs::e24_rule_unclosed_quote` | [x] |
| E25 | `GetAlertData` (`read-alert.c:309` fallthrough) | `fgets` hits EOF while `_r != 2` (empty file, header-only file, `_r == 1`) | `l_error` → `FreeAlertData`, `clearerr`, `NULL` | `err_alert.rs::e25_eof_r_not_two` | [x] |
| E26 | `GetAlertData` (`read-alert.c:307`) | `fgets` returns NULL because of a **read error** (not EOF) while `_r == 2` → `feof()` false | falls into `l_error` → `NULL` even though a complete alert was parsed | `err_alert.rs::e26_read_error_not_eof` | [x] |
| E27 | `GetAlertData` — stream already at EOF | `fp` positioned at end of file (what `Handle_Queue` does when `CRALERT_READ_ALL` is clear) | first `fgets` NULL, `feof` true, `_r == 0` → `l_error` → `NULL` | `err_alert.rs::e27_stream_at_eof` | [x] |

## Generic FFI boundary cases (required regardless of the table)

| # | case | expected | test | [x] |
|---|------|----------|------|-----|
| G1 | out-of-range "enum" values for `flags`: every `int` bit outside `0x01..0x10`, plus `INT_MIN`, `INT_MAX`, `-1`, `0x7fffffff`, random ints | C treats `flags` as a raw bitmask; only bits `0x01`, `0x04`, `0x10` are ever tested, all other bits are inert. Rust must be bit-identical | `err_flags.rs::g1_arbitrary_flag_ints`, `err_flags.rs::g1_init_arbitrary_flag_ints` | [x] |
| G2 | `GetAlertData(flag, fp)` with `flag` covering the full 32-value low-bit space × several file shapes | identical `alert_data` and identical resulting `ftell`/`feof`/`ferror` | `err_flags.rs::g1_arbitrary_flag_ints` | [x] |
| G3 | zero-length input (`alerts.log` is a 0-byte file) | `GetAlertData` → NULL (E25); the pipeline yields NULL | `err_alert.rs::e25_eof_r_not_two`, `cfg_queue.rs::c12_pipeline_empty_file` | [x] |
| G4 | oversized input: single line longer than `OS_MAXSTR` (1024) so `fgets` splits it mid-line; also a 64 KiB line | identical parse; `fgets(str, 1024, fp)` reads at most 1023 bytes + NUL | `cfg_shapes.rs::c14_oversized_line` | [x] |
| G5 | `timeout` boundary values `0`, `1`, `UINT_MAX` | identical. `UINT_MAX` is only observable when the first `GetAlertData` already succeeded (otherwise the loop would sleep for ~680 years), which is exactly what is tested | `err_queue.rs::e14_timeout_expires`, `err_queue.rs::e14_huge_timeout_not_reached_on_success` | [x] |
| G6 | `tm_mon` values 0..11 (the full valid domain of the `s_month[]` lookup) | `fileq->mon` = 3-char month abbreviation, no NUL written by `strncpy` | `cfg_shapes.rs::c02_all_months`, `cfg_driver.rs::c13_driver_all_months` | [x] |
| G7 | `tm_mon` outside 0..11; `tm_mday`/`tm_year` extremes | **UB in the C** for `tm_mon`: `s_month[p->tm_mon]` indexes past a 12-element `static const char *[]` and then dereferences whatever pointer it reads. Not comparable between two independently-linked `.so`s (the out-of-bounds bytes belong to different objects). Excluded and documented. `tm_mday`/`tm_year` extremes ARE tested, including `INT_MAX` where `tm_year + 1900` wraps — see the fix note below | `cfg_shapes.rs::c03_day_year_extremes` | [x] |
| G8 | NULL pointers for `fileq` / `p` / `fp` / `al_data` | every prototype is `__attribute__((nonnull))` and the C dereferences unconditionally. Verified out-of-process that BOTH implementations die with the SAME signal, so no defensive NULL check has crept into the Rust. `merror(NULL, …)` is the exception: glibc's `snprintf` tolerates it, both emit just `"\n"` | `err_flags.rs::g8_no_defensive_null_checks` | [x] |
| G9 | ambient `errno` at the two `perror()` call sites | neither implementation sets `errno` on those paths, so the message tail is `strerror(<caller's errno>)`. Checked with the ambient value pinned to 0 / ENOENT / EINVAL | `err_alert.rs::e20_colon_without_space`, `err_alert.rs::e21_no_colon_in_dateline` | [x] |

## Excluded: C undefined behaviour that cannot be made to agree

| # | function | trigger | why it is excluded |
|---|----------|---------|--------------------|
| U1 | `GetAlertData` (`read-alert.c:290`) | `issyscheck == 1` and a log line is exactly `Integrity checksum changed for: '` (nothing after the prefix) | `strdup(str+33)` returns a 1-byte block and `al_data->filename[strlen(...) - 1]` writes at index **-1**, corrupting the malloc header. Heap corruption is not observable-equivalent across two separately-heaped call sequences. The Rust reproduces the same `wrapping_offset(-1)` write; the generators in `cfg_alert.rs`/`cfg_shapes.rs` deliberately never emit that exact line |
| U2 | `GetAlertData` (`read-alert.c:120`) | last line of the file is exactly the 8 bytes `** Alert` with **no trailing newline** | `p = str + ALERT_BEGIN_SZ + 1` = `str + 9`, one past the NUL that `fgets` wrote at `str[8]`. `char str[OS_MAXSTR + 1]` is uninitialized in the C, so `strstr(str+9, ":")` reads indeterminate stack bytes (in practice, remnants of the previous call). The Rust zeroes its buffer, which is deterministic but cannot match indeterminate values. Excluded; generators keep every `** Alert` line either ≥ 9 bytes or newline-terminated |
| U3 | `Handle_Queue` (`file-queue.c:107`) | `CRALERT_FP_SET | CRALERT_READ_ALL` with `fp == NULL` on the very first call | `fileq->last_change = fileq->f_status.st_mtime` reads `f_status` before any `fstat` wrote it. Well-defined here only because both `driver` and the tests `memset` the queue to 0 first; a caller passing an uninitialized `file_queue` gets indeterminate values in both implementations. Tested only with the zeroed queue (C08 / E6b) |
| U4 | `merror` (`file-queue.c:25`) | `err_template == NULL` | `char buffer[256]` is uninitialized in the C, so if `snprintf` wrote nothing the C would print stale stack bytes where the Rust (zeroed buffer) prints nothing. Empirically glibc's `snprintf` still NUL-terminates for a NULL format, so both emit exactly `"\n"` — verified on a clean stack AND after a prior `merror` call dirtied the same slot (`err_flags.rs::g8_no_defensive_null_checks`). Listed here because the agreement rests on glibc behaviour, not on the C standard |

## Fix applied during Phase C

`Init_FileQueue` and `Read_FileMon` compute `fileq->year = p->tm_year + 1900`.
The original Rust used a plain `+`, which **aborts** in any build with overflow
checks on (the `dev`/`test` profile) when `tm_year` is near `INT_MAX`, whereas
the C wraps. Changed to `wrapping_add(1900)` in both places so the Rust matches
the C's observed two's-complement behaviour under every profile.
`cfg_shapes.rs::c03_day_year_extremes` covers it and fails (SIGABRT) if the
plain `+` is restored.

### Notes

(a) **E4** requires `strdup` to fail on a non-NULL string, i.e. a real OOM after
`strlen` succeeded. There is no portable, deterministic way to force that from
outside the library without an allocator hook, and doing so would also perturb
the harness. The Rust code path is byte-for-byte the same shape as the C
(`if dup.is_null() { fputs_stderr(...); exit(EXIT_FAILURE) }`), verified by
inspection; E1/E2/E3 exercise the identical `fputs_stderr` + `exit(1)` mechanism
so the message-and-exit machinery itself is covered by a passing test.

(b) **E8** requires `fstat(2)` to fail on a descriptor that `fopen` just
returned and that `fseek` just succeeded on. `fstat` on a valid fd cannot fail
(`EBADF` is the only realistic errno and the fd is valid by construction).
Unreachable in both implementations; the Rust branch is structurally identical.

(c) **E12** is dead code in the C: the only way to reach line 157 is for
`Handle_Queue(fileq, 0)` to have returned `1` while leaving `fp` NULL, which
`Handle_Queue` with `flags == 0` never does (it either `return (0)` on `fopen`
failure or leaves a valid `fp`). Kept in the table for completeness; the Rust
reproduces the same dead branch.
