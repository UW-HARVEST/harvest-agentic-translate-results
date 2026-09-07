# ERRORS.md — Phase C error-surface table

Every distinct rejection / error exit found by grepping the C sources for
`return -1`, `return (0)`, `return NULL`, `goto l_error`, `exit(`, `continue`,
`perror`, `merror`, and every explicit null / range / bound check.

`l_error` in `GetAlertData` means: `FreeAlertData(al_data); clearerr(fp); return NULL;`

Tests live in `tests/phase_c_errors.rs`, except rows 9-12 and 35 — whose expected
result is a message on **stderr** — which live in `tests/phase_c_stderr.rs`. fd 2
is process-global, so a test that redirects it cannot tolerate a concurrent test
calling `perror` (which `GetAlertData` does on rows 22/23). Cargo runs test
binaries one at a time, so the split makes those captures race-free; within each
file `capture_stderr` and `fork` additionally serialise on a mutex.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|----|----------|---------------------------------------------|-------------------|------|---|
| 1  | `os_strdup` (`shared.h:32`) | `str == NULL` | `fprintf(stderr,"NULL string passed to os_strdup")` then `exit(1)` | `err_01_os_strdup_null_exits` (subprocess) | [x] |
| 2  | `os_calloc` (`shared.h:15`) | `calloc` returns NULL (unsatisfiable `num*size`, e.g. `SIZE_MAX`) | `fprintf(stderr,"Memory allocation failed in os_calloc")` then `exit(1)` | `err_02_os_calloc_oom_exits` (subprocess) | [x] |
| 3  | `os_realloc` (`shared.h:24`) | `realloc` returns NULL (`new_size == SIZE_MAX`) | `fprintf(stderr,"Memory allocation failed in os_realloc")` then `exit(1)` | `err_03_os_realloc_oom_exits` (subprocess) | [x] |
| 4  | `os_calloc` | `num == 0` or `size == 0` — glibc returns a non-NULL minimal block, so **no** error | returns non-NULL | `err_04_os_calloc_zero_ok` | [x] |
| 5  | `os_realloc` | `ptr == NULL` (acts as `malloc`) — **no** error | returns non-NULL | `err_05_os_realloc_null_ptr_ok` | [x] |
| 6  | `os_realloc` | `new_size == 0` — glibc frees and returns NULL ⇒ hits the error branch | `exit(1)` (glibc ≥ 2.29 returns NULL for size 0) | `err_06_os_realloc_zero_size` (subprocess, compares C vs Rust exit status) | [x] |
| 7  | `Handle_Queue` (`file-queue.c:77-80`) | `!(flags & CRALERT_FP_SET)` and `fopen(file_name,"r")` fails (`alerts.log` absent) | `return 0` → propagates as `Init_FileQueue == 0`, `Read_FileMon == NULL` | `err_07_missing_alerts_log` | [x] |
| 8  | `Handle_Queue` (`file-queue.c:85-87`) | `!(flags & CRALERT_READ_ALL)` and `fileq->fp == NULL` (only reachable with `CRALERT_FP_SET` and a NULL `fp`) | `return 0` (not `< 0`) → `Init_FileQueue` returns `0` | `err_08_fp_set_null_fp` | [x] |
| 9  | `Handle_Queue` (`file-queue.c:89-94`) | `!(flags & CRALERT_READ_ALL)` and `fseek(fp,0,SEEK_END) < 0` — `fp` is an unseekable stream (a pipe), `CRALERT_FP_SET` set | `merror(FSEEK_ERROR,…)` on stderr, `fclose(fp)`, `fp=NULL`, `return -1` → `Init_FileQueue == -1` | `err_09_fseek_error_unseekable` | [x] |
| 10 | `Handle_Queue` (`file-queue.c:99-104`) | `fp != NULL` and `fstat(fileno(fp),…) < 0` — `fp` from `fmemopen` (`fileno` = -1 ⇒ `EBADF`), `CRALERT_FP_SET \| CRALERT_READ_ALL` | `merror(FSTAT_ERROR,…)` on stderr, `fclose(fp)`, `fp=NULL`, `return -1` → `Init_FileQueue == -1` | `err_10_fstat_error_fmemopen` | [x] |
| 11 | `Init_FileQueue` (`file-queue.c:135-137`) | `Handle_Queue(...) < 0` (any of #9 / #10) | `return -1` | `err_09…`, `err_10…` | [x] |
| 12 | `driver` (`driver.c:15-18`) | `Init_FileQueue(&fq,&time,flags) < 0` | `fprintf(stderr,"File queue initialization failed")`, `return NULL` | unreachable via `driver` (see note A) — documented + asserted in `err_12_driver_init_never_fails` | [x] |
| 13 | `Read_FileMon` (`file-queue.c:149-153`) | `fileq->fp == NULL` and `Handle_Queue(fileq,0) != 1` (file absent) | `file_sleep()` (5 s), `return NULL` | `err_07_missing_alerts_log` | [x] |
| 14 | `Read_FileMon` (`file-queue.c:156-158`) | `fileq->fp == NULL` after a `Handle_Queue` that returned `1` (`CRALERT_FP_SET\|CRALERT_READ_ALL`, `fp` left NULL) | `return NULL` immediately (no sleep) | `err_14_read_filemon_null_fp_fast` | [x] |
| 15 | `Read_FileMon` (`file-queue.c:171-174`) | first `GetAlertData` returned NULL and the re-`Handle_Queue(fileq,0)` `!= 1` (file deleted between the two calls) | `file_sleep()` (5 s), `return NULL` | `err_15_file_deleted_midway` | [x] |
| 16 | `Read_FileMon` (`file-queue.c:177-188`) | loop exhausts `timeout` retries without an alert | `return NULL` | `err_16_timeout_expires` (`timeout` 0 and 1) | [x] |
| 17 | `GetAlertData` (`read-alert.c:111-115`) | `_r == 2`, a new `** Alert` line, and `fseek(fp,-strlen(str),SEEK_CUR) == -1` (unseekable stream) | `l_error` → NULL | `err_17_alert_pushback_fseek_fail` | [x] |
| 18 | `GetAlertData` (`read-alert.c:120-123`) | `** Alert` line whose remainder (`str+9`) contains no `':'` | `continue` — line skipped, `_r` stays `0` ⇒ eventually NULL | `err_18_alert_no_colon` | [x] |
| 19 | `GetAlertData` (`read-alert.c:131-134`) | `** Alert` line with a `':'` but no `' '` after `str+9` | `continue`, `_r` stays `0` ⇒ NULL | `err_19_alert_no_space` | [x] |
| 20 | `GetAlertData` (`read-alert.c:139-142`) | `flag & CRALERT_MAIL_SET` and the token after the first space is not `mail` | `continue`, `_r` stays `0` ⇒ NULL | `err_20_mail_set_rejects_nonmail` | [x] |
| 21 | `GetAlertData` (`read-alert.c:166-168`) | `_r < 1`: any line before the first `** Alert` header | `continue` — silently ignored | `err_21_lines_before_header_ignored` | [x] |
| 22 | `GetAlertData` (`read-alert.c:183-187`) | `_r == 1` and the date/location line contains a `':'` but no `' '` at/after it | `perror("date of location not NULL")`, `l_error` → NULL | `err_22_dateline_colon_no_space` | [x] |
| 23 | `GetAlertData` (`read-alert.c:191-194`) | `_r == 1` and `p == NULL` (date/location line has no `':'` at all) | `perror("date or location not NULL or p is NULL")`, `l_error` → NULL | `err_23_dateline_no_colon` | [x] |
| 24 | `GetAlertData` (`read-alert.c:191-194`) | `_r == 1` and `al_data->date`/`location` already set — unreachable (`_r` becomes 2 on success) | same `perror` + `l_error` | documented unreachable in `err_24_date_already_set_unreachable` | [x] |
| 25 | `GetAlertData` (`read-alert.c:209-220`) | `Rule: ` line where `p` becomes NULL after the two `strchr(p,' ')` hops (fewer than two spaces after the rule id) | `l_error` → NULL | `err_25_rule_too_few_spaces` | [x] |
| 25b | `GetAlertData` (`read-alert.c:203`) | `"Rule:"` with **no** trailing space: after `os_clearnl` strips the newline, `strncmp("Rule: ", str, 6)` compares `' '` against `'\0'` and does **not** match, so the line is NOT rejected — it falls through to the log-message branch and the alert is accepted | alert returned with `rule == 0`, `comment == NULL` | `err_25_rule_too_few_spaces` (second half) | [x] |
| 26 | `GetAlertData` (`read-alert.c:225-228`) | `Rule: ` line with ≥ 2 spaces but no `'\''` | `l_error` → NULL | `err_26_rule_no_quote` | [x] |
| 27 | `GetAlertData` (`read-alert.c:235-240`) | `Rule: ` line whose comment has an opening `'\''` but no second `'\''` (`strrchr` finds only the one already consumed) | `l_error` → NULL | `err_27_rule_unterminated_comment` | [x] |
| 28 | `GetAlertData` (`read-alert.c:305-307`) | loop ended (`fgets` NULL) but **not** (`feof(fp) && _r == 2`) — e.g. empty file (`_r == 0`), or header-only file (`_r == 1`) | `l_error` → NULL | `err_28_eof_wrong_state` | [x] |
| 29 | `GetAlertData` (`read-alert.c:104`) | `fp` positioned at end of file (the default `Init_FileQueue` non-`READ_ALL` state) | `fgets` NULL immediately, `_r == 0` ⇒ `l_error` → NULL | `err_29_fp_at_eof` | [x] |
| 30 | `GetAlertData` (`read-alert.c:284`) | `log_size < LOG_LIMIT` — `log_size` is only ever reset to 0 (the increment is commented out) so the guard **never** rejects, even with > 100 log lines | log branch always taken | `err_30_log_limit_never_trips` (>100 log lines) | [x] |
| 31 | `GetAlertData` (`read-alert.c:289-291`) | syscheck alert with an `Integrity checksum changed for: '` line and an *empty* filename ⇒ `filename[strlen-1]` writes at index `-1` | C writes one byte before the buffer (UB, benign in practice); Rust reproduces with `wrapping_sub` | `err_31_integrity_empty_filename` | [x] |
| 32 | `GetAlertData` / `Init_FileQueue` / `Read_FileMon` / `FreeAlertData` (`__attribute__((nonnull))`) | NULL pointer for a `nonnull` parameter | undefined behaviour in C (typically SIGSEGV); **not** a defined rejection | documented; not differentially tested (would crash the harness) | [x] |
| 33 | `Init_FileQueue` / `Read_FileMon` (`file-queue.c:125,166`) | `p->tm_mon` inside `0..=11` | `mon` = the 3-byte month abbreviation | `err_33a_in_range_month_exact` (all 12, full byte equality) | [x] |
| 33b | same | `p->tm_mon` **outside** `0..=11` — `s_month[tm_mon]` is an out-of-bounds read, then `strncpy` from whatever pointer it yields | UB in C (usually SIGSEGV). See **Note B** — this was a REAL translation defect, now fixed | `err_33b_out_of_range_month_is_ub_in_both`, `err_33c_rust_does_not_bounds_check_the_month` | [x] |
| 34 | `driver` / `Init_FileQueue` | out-of-range **enum-like** `flags` int with no valid bit (e.g. `-1`, `0x7fffffff`, `0x20`, `INT_MIN`) — C `int` accepts any value; only bits `0x1`,`0x4`,`0x10` are tested | bit-tested, so unknown bits are ignored; behaviour determined by the known bits | `err_34_unknown_flag_bits` | [x] |
| 35 | `merror` (`file-queue.c:24-28`) | template/arg combinations: `snprintf` truncation past the 256-byte buffer; a template with no conversions; `err` = `INT_MIN`/`INT_MAX` | truncated at 255 chars + NUL, printed with a trailing `\n` | `err_35_merror_truncation` (6 templates × 11 lengths × 5 `err` values) | [x] |
| 35b | `merror` | a template demanding MORE conversions than the three arguments supplied (e.g. `"%s%s%s%s"`) | reads past the varargs list — UB, not a rejection; excluded from testing | documented in `err_35_merror_truncation` | [x] |

**Note A** — `driver` `memset`s its `file_queue` to 0 *before* `Init_FileQueue`, so
`fileq->fp` is always NULL on entry. Row #9/#10 need a non-NULL unseekable/
unstattable `fp`, which only the direct `Init_FileQueue` entry point can supply.
Hence row #12's branch is unreachable through `driver` for every `int flags`;
the test asserts C and Rust agree on that (both never print the message).


**Note B — the one real defect this phase found.**
`src/file_queue.rs::copy_month` originally read:

```rust
if tm_mon >= 0 && (tm_mon as usize) < 12 {
    strncpy((*fileq).mon.as_mut_ptr(), cs(S_MONTH[tm_mon as usize]), 3);
}
```

i.e. it silently skipped the copy for an out-of-range `tm_mon`, where the C
performs an unchecked `s_month[tm_mon]` read. That is a behavioural divergence,
not a hardening: `driver(day, month, year, …)` passes `month` straight into
`tm_mon`, so the C faults on `month = 12` while the Rust returned normally
(observed: `C signal 11 / Rust exit 7`). Adding a bounds check the ground truth
does not have is exactly the kind of "fix" that must not happen, so the Rust now
reproduces the unchecked read verbatim via a raw `[*const c_char; 12]` table and
`.offset(tm_mon as isize)`.

Residual difference: once both objects perform the same out-of-bounds read, the
*value* obtained depends on whatever bytes happen to follow a 12-pointer table
inside each individual `.so`. That is not a property of the translation and
cannot be matched. `err_33b` therefore asserts full equality whenever both
children survive the wild read, and `err_33c` proves the Rust still faults on
out-of-range months (which a bounds-checked version never could).

**Note C — the one remaining unmatched-by-construction read.**
`GetAlertData` declares `char str[OS_MAXSTR + 1];` and initialises only
`str[OS_MAXSTR] = '\0'`, so the rest is stack garbage on the first `fgets`.
The single place the C reads past the NUL that `fgets` wrote is

```c
p = str + ALERT_BEGIN_SZ + 1;   /* str + 9 */
```

which is past the terminator exactly when the header line is the 8 bytes
`"** Alert"` with no newline — i.e. at EOF, or when an embedded NUL truncates the
line. The Rust buffer is zero-initialised (Rust has no way to read uninitialised
memory without its own UB), so the first call could in principle differ.
In practice it does not: the buffer is reused across `fgets` iterations, so from
the second line onwards both hold byte-identical leftovers, and a fresh stack
page reads as zero. `fuzz_e_header_reads_past_terminator` (400 cases with varying
preceding lines) and `fuzz_c_raw_bytes` (800 files including embedded NULs)
exercise this path and agree.

## Mechanical constant audit

`ERRORS.md`/`CONFIGS.md` coverage is backed by a scripted comparison of every
`#define` in the C against the corresponding Rust `const`:

* 27 numeric constants (`OS_MAXSTR`, `MAX_FQUEUE`, `FQ_TIMEOUT`, `LOG_LIMIT`,
  all `CRALERT_*` bits, all `*_BEGIN_SZ`) — **0 mismatches**.
* 21 string constants (`ALERTS_DAILY`, `ALERT_BEGIN`, `ALERT_MAIL`, all
  `*_BEGIN` prefixes, `FSTAT_ERROR`, `FSEEK_ERROR`) — **0 mismatches**.
* Argument order of every `strncmp` call site matches, including
  `read-alert.c:287` where the C uniquely puts the buffer first
  (`strncmp(str, "Integrity checksum changed for: '", 33)`) while every other
  site puts the literal first.
* The `goto l_error` control flow maps exactly onto the Rust
  `'l_error: { ... }` block: `break 'l_error` == `goto l_error`, falling out of
  the block == the C falling through the `if (feof(fp) && _r == 2)` test.
