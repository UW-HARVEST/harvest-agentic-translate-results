# CONFIGS.md — Phase B configuration-surface table

## Axes, derived mechanically from the C source

### Public entry points (all 9 exported symbols, lowest-level first)

`os_calloc`, `os_realloc`, `os_strdup` (`shared.h`) →
`merror`, `FreeAlertData` → `GetAlertData` (`read-alert.c`) →
`Init_FileQueue`, `Read_FileMon` (`file-queue.c`) → `driver` (`driver.c`).

### Runtime option axis — the `flags` / `flag` bitmask (`read-alert.h`)

| bit | macro | value | code that branches on it |
|-----|-------|-------|--------------------------|
| 0 | `CRALERT_MAIL_SET`    | 0x001 | `GetAlertData`: requires the token after the alert id to be `mail`, else `continue` |
| 1 | `CRALERT_EXEC_SET`    | 0x002 | **never branched on** — must still round-trip through `fileq->flags` |
| 2 | `CRALERT_READ_ALL`    | 0x004 | `Handle_Queue`: skips the `fseek(fp,0,SEEK_END)` |
| 3 | `CRALERT_READ_FAILED` | 0x008 | **never branched on** |
| 4 | `CRALERT_FP_SET`      | 0x010 | `Handle_Queue`: skips close+`fopen`; `GetFile_Queue`: file name becomes `"<stdin>"` instead of `"alerts.log"` |
| — | undefined bits        | e.g. 0xFFFF, -1 | no validation anywhere; C `int` accepts any value |

### Input-shape axes

* **alert-stream shape** (`GetAlertData`): empty / no-header / one alert /
  many alerts / alert with every field / syscheck alert / truncated alert.
* **line shape**: trailing newline vs bare EOF, CRLF, `> OS_MAXSTR-1` (1023)
  bytes so `fgets` splits it, blank lines, leading spaces after `-`.
* **field shape**: `Rule:`, `Src IP:`, `Src Port:`, `Dst IP:`, `Dst Port:`,
  `User:` present / absent / **repeated** (repeat exercises the
  `os_free` + `os_strdup` re-assignment paths).
* **numeric shape** (`atoi`): 0, 1, negative, `INT_MAX`, `INT_MAX+1`
  (overflow), non-numeric, leading `+`/spaces — for `rule`, `level`,
  `srcport`, `dstport`.
* **stream kind**: seekable regular file vs non-seekable pipe; stream
  pre-positioned mid-file vs at offset 0.
* **file system shape** (`Init_FileQueue`/`Read_FileMon`/`driver`):
  `alerts.log` present / absent / empty / directory-only.
* **`struct tm` shape**: `tm_mon` 0..11 (all twelve → all `s_month` entries),
  `tm_mday`/`tm_year` = 0, 1, −1, `INT_MAX`, `INT_MIN`.
* **`timeout`**: 0 (no retry loop) vs 1 (one retry — costs one 5 s
  `file_sleep`); larger values are the same loop and are avoided for runtime.

## Rows — one per combination the C actually distinguishes

| # | entry point(s) | configuration (options set + input shape) | pass / test |
|---|----------------|--------------------------------------------|-----|
| 1 | `os_calloc` | randomized `num`×`size` incl. 0×0, 1×n, n×1, large; zero-fill + returned-pointer validity | [x] `row01_os_calloc` |
| 2 | `os_realloc` | `ptr=NULL` grow-from-nothing, then randomized shrink/grow chains preserving contents | [x] `row02_os_realloc` |
| 3 | `os_strdup` | randomized byte strings: empty, 1 byte, 1 KiB, high-bit bytes, embedded spaces | [x] `row03_os_strdup` |
| 4 | `merror` | `FSEEK_ERROR`/`FSTAT_ERROR` templates × randomized file names/errnos, incl. names long enough to hit the 256-byte `snprintf` truncation | [x] `row04_merror` |
| 5 | `FreeAlertData` | struct with all pointers NULL; struct with every pointer set; struct with a mixed subset — via a `GetAlertData` result and via a hand-built heap struct | [x] `row05_free_alert_data` |
| 6 | `GetAlertData` | `flag=0`, one minimal complete alert (header + date/location + `Rule:`), seekable file, trailing newline | [x] `row06_minimal_alert` |
| 7 | `GetAlertData` | `flag=0`, one alert with **every** field: `Rule:`, `Src IP:`, `Src Port:`, `Dst IP:`, `Dst Port:`, `User:` + free-form log lines | [x] `row07_all_fields` |
| 8 | `GetAlertData` | `flag=0`, **multiple** alerts in one file → exercises the `_r==2` + `fseek(-strlen)` push-back; iterate `GetAlertData` until NULL and compare the whole sequence + `ftell` after each call | [x] `row08_multiple_alerts` |
| 9 | `GetAlertData` | `flag=0`, repeated `Rule:`/`Src IP:`/`Dst IP:`/`User:`/`Src Port:`/`Dst Port:` lines inside one alert (last-wins + `os_free` re-assignment) | [x] `row09_repeated_fields` |
| 10 | `GetAlertData` | `flag=0`, group field containing `syscheck` + `Integrity checksum changed for: '<path>'` next line → `filename` extraction | [x] `row10_syscheck_filename` |
| 11 | `GetAlertData` | `flag=0`, group contains `syscheck` but the next line does **not** match the 33-byte prefix → `issyscheck` reset, `filename` stays NULL | [x] `row11_syscheck_reset` |
| 12 | `GetAlertData` | `flag=0`, group **without** `syscheck` + an `Integrity checksum changed for: '…'` line → `filename` must stay NULL | [x] `row12_no_syscheck_group` |
| 13 | `GetAlertData` | `flag=CRALERT_MAIL_SET`, header token **is** `mail` → alert accepted | [x] `row13_14_mail_flag` |
| 14 | `GetAlertData` | `flag=CRALERT_MAIL_SET`, header token is **not** `mail` → header skipped | [x] `row13_14_mail_flag` |
| 15 | `GetAlertData` | `flag` = each of `CRALERT_EXEC_SET`, `CRALERT_READ_ALL`, `CRALERT_READ_FAILED`, `CRALERT_FP_SET` alone and `0x1F` together, on the same alert → only the MAIL bit may change the result | [x] `row15_each_flag_bit` |
| 16 | `GetAlertData` | `flag` = randomized undefined-bit patterns (incl. `-1`, `INT_MIN`, `INT_MAX`) on the same alert | [x] `row16_undefined_flag_bits` |
| 17 | `GetAlertData` | `flag=0`, no trailing newline before EOF (`feof && _r==2` success path) | [x] `row17_no_trailing_newline` |
| 18 | `GetAlertData` | `flag=0`, CRLF line endings (`\r` retained by `os_clearnl`) | [x] `row18_crlf` |
| 19 | `GetAlertData` | `flag=0`, a line > 1023 bytes → `fgets` splits it mid-alert | [x] `row19_oversized_lines` |
| 20 | `GetAlertData` | `flag=0`, blank lines / whitespace-only lines interleaved in the alert body | [x] `row20_blank_lines` |
| 21 | `GetAlertData` | `flag=0`, randomized `rule`/`level`/`srcport`/`dstport` numerics incl. overflow and non-numeric text | [x] `row21_numeric_shapes` |
| 22 | `GetAlertData` | `flag=0`, randomized fully-synthetic alert streams (property test, fixed seed): random field sets, orders, counts, group names, alert-id shapes | [x] `row22_random_streams` |
| 23 | `GetAlertData` | `flag=0`, stream pre-positioned mid-file via `fseek` before the call | [x] `row23_preseeked_stream` |
| 24 | `GetAlertData` | `flag=0`, **non-seekable** pipe holding one complete alert (succeeds — no push-back needed) | [x] `row24_pipe_single_alert` |
| 25 | `Init_FileQueue` | `flags=0`, `alerts.log` present → opens + seeks to END + `fstat`; compare the entire 440-byte `file_queue` (minus the `FILE*`) and `last_change`/`f_status` | [x] `row25_init_default_flags` |
| 26 | `Init_FileQueue` | `flags=CRALERT_READ_ALL`, `alerts.log` present → no seek-to-end, offset stays 0 | [x] `row26_init_read_all` |
| 27 | `Init_FileQueue` | `flags=CRALERT_FP_SET` with a caller-supplied `fp` → `fp` preserved, `file_name=="<stdin>"`, seek-to-END applied | [x] `row27_init_fp_set` |
| 28 | `Init_FileQueue` | `flags=CRALERT_FP_SET|CRALERT_READ_ALL` → `fp` preserved, no seek | [x] `row28_init_fp_set_read_all` |
| 29 | `Init_FileQueue` | `flags` = `CRALERT_MAIL_SET`, `CRALERT_EXEC_SET`, `CRALERT_READ_FAILED`, `0x1F`, and randomized undefined bit patterns | [x] `row29_init_flag_matrix` |
| 30 | `Init_FileQueue` | all twelve `tm_mon` values 0..11 → `mon[4]` must equal `Jan`..`Dec` (`strncpy(...,3)` leaves `mon[3]` from the memset) | [x] `row30_init_all_months` |
| 31 | `Init_FileQueue` | randomized `tm_mday`/`tm_year` incl. 0, −1, `INT_MAX`, `INT_MIN` (`year = tm_year + 1900` overflow) | [x] `row31_init_tm_extremes` |
| 32 | `Init_FileQueue` | pre-dirtied `file_queue` (non-zero garbage) → checks the `memset`/field-reset behaviour, incl. `flags=0` then `flags=flags` | [x] `row32_init_dirty_struct` |
| 33 | `Read_FileMon` | after `Init_FileQueue(flags=CRALERT_READ_ALL)` on a file with one alert, `timeout=0` → returns that alert; compare all fields + resulting `file_queue` | [x] `row33_read_filemon_single` |
| 34 | `Read_FileMon` | after `Init_FileQueue(flags=CRALERT_READ_ALL)` on a file with **many** alerts, `timeout=0`, called repeatedly → whole sequence compared | [x] `row34_read_filemon_sequence` |
| 35 | `Read_FileMon` | after `Init_FileQueue(flags=CRALERT_READ_ALL|CRALERT_MAIL_SET)` on mixed mail/non-mail alerts, `timeout=0` | [x] `row35_read_filemon_mail_filter` |
| 36 | `Read_FileMon` | `flags=CRALERT_FP_SET|CRALERT_READ_ALL` with a caller `fp` on a temp file containing alerts, `timeout=0` | [x] `row36_read_filemon_fp_set` |
| 37 | `Read_FileMon` | `tm` supplied to `Read_FileMon` **differs** from the one given to `Init_FileQueue` (day/mon/year re-stamped on the NULL path) | [x] `row37_read_filemon_different_tm` |
| 38 | `Read_FileMon` | `timeout=1` on an exhausted queue → one retry, one `file_sleep` (5 s), NULL | [x] `row38_read_filemon_timeout_one` |
| 39 | `driver` | `flags=CRALERT_READ_ALL`, `timeout=0`, `alerts.log` with one full alert → returns the alert | [x] `row39_driver_single_alert` |
| 40 | `driver` | `flags=CRALERT_READ_ALL|CRALERT_MAIL_SET`, `timeout=0`, mail and non-mail alerts | [x] `row40_driver_mail_filter` |
| 41 | `driver` | `flags=0`, `timeout=0` (seek-to-END → nothing to read → NULL) | [x] `row41_driver_seek_to_end` |
| 42 | `driver` | `flags=CRALERT_READ_ALL`, `timeout=0`, randomized `day`/`month`(0..11)/`year` (property test, fixed seed) | [x] `row42_driver_random_dates` |
| 43 | `driver` | `flags` = randomized full-bitmask patterns incl. `CRALERT_FP_SET` and undefined bits, `timeout=0` | [x] `row43_driver_flag_masks` |
| 44 | `driver` | `flags=CRALERT_READ_ALL`, `timeout=0`, randomized whole alert files (property test, fixed seed) — end-to-end pipeline | [x] `row44_driver_random_files` |
| 45 | pipeline | `Init_FileQueue` → `Read_FileMon` → `FreeAlertData` driven directly (not via `driver`), across the flag matrix, with the `file_queue` compared after **every** step | [x] `row45_pipeline_matrix` |
| 46 | `Read_FileMon` | the `while (i < timeout)` retry loop **succeeding**: first `GetAlertData` fails, the re-`Handle_Queue` succeeds, the loop's first attempt fails, then a complete alert is appended while the library is inside `file_sleep` so a later iteration returns it | [x] `row46_read_filemon_retry_loop_succeeds` |
| 47 | fuzz | mutation + random-byte differential fuzzing over `GetAlertData`, `Init_FileQueue`+`Read_FileMon` and `driver`: bit flips, byte splices, truncation, line duplication/removal, random flag words, and inputs straddling the 1023-byte `fgets` boundary (fixed seeds; each test asserts it is not vacuously comparing NULL against NULL) | [x] `fuzz_random_bytes + fuzz_prefixed_lines + fuzz_mutated_valid_alerts + fuzz_fgets_boundary + fuzz_driver_end_to_end + fuzz_filequeue_pipeline` |

## Binary executable

`c_src/CMakeLists.txt` builds **only** `add_library(driver SHARED ...)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`crate-type = ["cdylib"]` with no `[[bin]]`. There is therefore **no driver
binary** whose stdout could be compared; the `.so`-level differential tests are
the complete surface.
