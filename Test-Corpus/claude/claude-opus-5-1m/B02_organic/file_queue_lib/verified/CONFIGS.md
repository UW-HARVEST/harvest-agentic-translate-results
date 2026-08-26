# CONFIGS.md — configuration-surface table (valid inputs)

Mirror of `ERRORS.md` for **accepted** inputs. Axes derived mechanically from
the public headers plus every `if` / `switch` / `? :` the C takes on a runtime
option or on an input shape. There are no `#ifdef`s and no build options
(`CMakeLists.txt` has none, `Cargo.toml` has no `[features]`), so the only
configuration is at *runtime*.

## Axis 1 — runtime option bits (`read-alert.h`)

| bit | name | value | what it toggles in the C |
|-----|------|-------|--------------------------|
| 0 | `CRALERT_MAIL_SET` | `0x001` | `GetAlertData`: header must have `mail` after the first space, else the alert is skipped (`read-alert.c:139`) |
| 1 | `CRALERT_EXEC_SET` | `0x002` | **never tested** — pass-through only (stored in `fileq->flags`, forwarded as `flag`) |
| 2 | `CRALERT_READ_ALL` | `0x004` | `Handle_Queue`: skip the `fseek(fp,0,SEEK_END)` (`file-queue.c:84`), i.e. read from the start |
| 3 | `CRALERT_READ_FAILED` | `0x008` | **never tested** — pass-through only |
| 4 | `CRALERT_FP_SET` | `0x010` | `GetFile_Queue`: `file_name` becomes `"<stdin>"` instead of `"alerts.log"` (`file-queue.c:60`); `Handle_Queue`: do **not** `fclose`/`fopen`, reuse the caller's `fp` (`file-queue.c:67`); `Init_FileQueue`: do **not** zero `fileq->fp` (`file-queue.c:116`) |

Note the asymmetry the C deliberately has: `Read_FileMon` always calls
`Handle_Queue(fileq, 0)` — **literal 0**, not `fileq->flags` — so re-opens
always `fopen` the `file_name` and always seek to end, regardless of
`CRALERT_FP_SET` / `CRALERT_READ_ALL`. Rows 30–39 exercise that.

## Axis 2 — stream kind for `GetAlertData` / `fileq->fp`

seekable regular file · non-seekable pipe (`ESPIPE` on `fseek`) ·
`fmemopen` buffer (`fileno` == -1) · write-only `FILE*` (`fgets` fails, `feof`
clear).

## Axis 3 — input shapes the parser special-cases

`_r` state machine 0→1→2; header token `** Alert`; body tokens `Rule: `,
`Src IP: `, `Src Port: `, `Dst IP: `, `Dst Port: `, `User: `; the syscheck
sub-mode (`group` contains `syscheck`) and its
`Integrity checksum changed for: '` line; the `log_size < LOG_LIMIT` catch-all;
`OS_MAXSTR`(1024)-bounded `fgets`; presence/absence of trailing `'\n'`;
duplicate tokens (the `os_free`-then-`os_strdup` re-assignment paths);
1, 2, N alerts per file; `atoi` value ranges.

## Axis 4 — `struct tm` / `driver` scalars

`tm_mday`, `tm_mon` (0..11 valid; `s_month[]` has 12 entries), `tm_year`
(`+1900`); `timeout` (0 = no `file_sleep`, N = N × `FQ_TIMEOUT`s).

---

## Configuration rows

Each row is run against **many randomized inputs** with a fixed seed
(`tests/common/mod.rs::Rng`, seed noted per test) unless the row is inherently
a single shape.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `os_calloc` | `(num,size)` over randomized small/zero/1-byte pairs; check pointer non-NULL and buffer fully zeroed | [x] |
| 2 | `os_calloc` | `num*size == 0` (`(0,0)`, `(0,16)`, `(16,0)`) — allocator may return NULL or a unique ptr; assert both agree on NULL-ness | [x] |
| 3 | `os_realloc` | `ptr == NULL` (acts as `malloc`) with randomized sizes incl. 0 | [x] |
| 4 | `os_realloc` | grow an existing block, randomized old/new sizes, assert the prefix bytes survive identically | [x] |
| 5 | `os_realloc` | shrink an existing block, randomized sizes | [x] |
| 6 | `os_strdup` | randomized byte strings (len 0..512, all non-NUL byte values incl. high-bit) — assert identical contents+length | [x] |
| 7 | `merror` | `FSEEK_ERROR` template, randomized `file_name`/`err`/`err_msg`; stderr captured and compared byte-for-byte | [x] |
| 8 | `merror` | `FSTAT_ERROR` template, randomized args | [x] |
| 9 | `merror` | arbitrary caller template with `%s %d %s`, randomized args incl. empty strings and negative `err` | [x] |
| 10 | `GetAlertData` + `FreeAlertData` | `flag = 0`, seekable file, **one complete alert**: header+date/location+`Rule:`+`Src IP:`+`Src Port:`+`Dst IP:`+`Dst Port:`+`User:`+log lines; all fields randomized | [x] |
| 11 | `GetAlertData` | `flag = 0`, seekable, **minimal accepted alert**: header + date/location only, EOF (`feof && _r==2`) | [x] |
| 12 | `GetAlertData` | `flag = 0`, seekable, **two alerts** — first call must `fseek` back onto the second `** Alert`; assert returned struct **and** `ftell` | [x] |
| 13 | `GetAlertData` | `flag = 0`, seekable, **N alerts** (N randomized 3..6): call repeatedly until NULL, compare the whole sequence and every intermediate `ftell`/`feof`/`ferror` | [x] |
| 14 | `GetAlertData` | `flag = CRALERT_MAIL_SET`, header **is** `mail` → accepted | [x] |
| 15 | `GetAlertData` | `flag = CRALERT_MAIL_SET`, mixed file: non-`mail` alerts interleaved with `mail` alerts (skip-then-accept) | [x] |
| 16 | `GetAlertData` | `flag` = each of `CRALERT_EXEC_SET`, `CRALERT_READ_ALL`, `CRALERT_READ_FAILED`, `CRALERT_FP_SET` alone and all 32 subsets of the 5 bits, on the same alert → only bit 0 may change the result | [x] |
| 16b | `GetAlertData` | `flag` = `mail`-prefix words (`mailx`, `mail`, `maiL`, ...) with `CRALERT_MAIL_SET`: only the first `ALERT_MAIL_SZ == 4` bytes are compared, so `mailx` is **accepted** | [x] |
| 17 | `GetAlertData` | header **without** `-` → `group` stays NULL | [x] |
| 18 | `GetAlertData` | header **with** `-` and randomized leading-space runs (`- `, `-  `, `-\t`) → `group` = text after `-` with spaces skipped, newline stripped | [x] |
| 19 | `GetAlertData` | header group containing `syscheck` **and** a following `Integrity checksum changed for: '<path>'` line → `filename` = `<path>` with the last byte dropped | [x] |
| 20 | `GetAlertData` | group contains `syscheck` but the **next** body line is not the integrity line → `issyscheck` reset to 0, `filename` NULL, and a later integrity line is ignored | [x] |
| 21 | `GetAlertData` | group contains `syscheck` as a **substring** (e.g. `xsyscheckx`, `ossec,syscheck,`) — `strstr` semantics | [x] |
| 22 | `GetAlertData` | duplicate `Src IP:` / `Dst IP:` / `User:` / `Rule:` lines in one alert → the `os_free`+re-`os_strdup` paths; last wins | [x] |
| 23 | `GetAlertData` | `Src Port:` / `Dst Port:` with randomized values incl. 0, negative, `65535`, `2147483647`, non-numeric, leading `+`/spaces | [x] |
| 24 | `GetAlertData` | `Rule:` with randomized rule/level incl. 0, huge (`unsigned` reinterpretation), negative | [x] |
| 25 | `GetAlertData` | `Rule:` comment containing embedded quotes → `strrchr` picks the **last** `'` | [x] |
| 26 | `GetAlertData` | date/location line with several colons and several spaces (first colon, then first space at/after it) | [x] |
| 27 | `GetAlertData` | **no trailing newline** before EOF on the last line, for header / date line / body line variants | [x] |
| 28 | `GetAlertData` | lines at the `fgets` boundary: total line length 1022, 1023, 1024, 1025 and 4096 bytes → a logical line is split and the tail is re-parsed as a fresh line | [x] |
| 29 | `GetAlertData` | stream kinds: same alert bytes served from a seekable file, from a `pipe`, and from `fmemopen` → same struct (the `fseek`-back row differs and is `ERRORS.md` #5) | [x] |
| 30 | `Init_FileQueue` | `flags = 0`, `alerts.log` **present** in CWD → returns 0, whole `file_queue` struct compared (`last_change`, `year`, `day`, `flags`, `mon`, `file_name`, `fp != NULL`, `f_status`), file offset is at EOF | [x] |
| 31 | `Init_FileQueue` | `flags = CRALERT_READ_ALL`, `alerts.log` present → offset stays 0 | [x] |
| 32 | `Init_FileQueue` | `flags = CRALERT_FP_SET`, caller supplies a real seekable `fp`, `alerts.log` absent → `file_name == "<stdin>"`, caller's `fp` reused, seeked to EOF | [x] |
| 33 | `Init_FileQueue` | `flags = CRALERT_FP_SET \| CRALERT_READ_ALL`, caller `fp` real → no `fseek`, `fstat` runs, offset unchanged | [x] |
| 34 | `Init_FileQueue` | `flags = 0` but a file literally named `<stdin>` exists and `alerts.log` also exists → confirms `alerts.log` is chosen | [x] |
| 35 | `Init_FileQueue` | `flags = CRALERT_FP_SET` **but** `fileq->fp` NULL and a file named `<stdin>` exists → `Handle_Queue` still doesn't `fopen` (row is the accepted-side twin of `ERRORS.md` #20) | [x] |
| 36 | `Init_FileQueue` | all 32 subsets of the 5 `CRALERT_*` bits × {`alerts.log` present, absent} × {caller `fp` real, NULL} → return code + full struct compared | [x] |
| 37 | `Init_FileQueue` | `tm` scalars randomized over the valid ranges (`tm_mday` 1..31, `tm_mon` 0..11, `tm_year` −1900..8099) → `day`, `year`, `mon` compared | [x] |
| 38 | `Init_FileQueue` | pre-dirtied `file_queue` (non-zero garbage in every field) → confirms the `memset`/`= 0` re-initialisation order, incl. `f_status` being left untouched when `fp` is NULL | [x] |
| 39 | `Read_FileMon` | `fileq` from `Init_FileQueue(flags = CRALERT_READ_ALL)` with a multi-alert `alerts.log`, `timeout = 0` → first alert returned; repeated calls walk the file | [x] |
| 40 | `Read_FileMon` | `fileq->fp == NULL`, `alerts.log` present, `timeout = 0` → `Handle_Queue(fileq,0)` re-opens **and seeks to EOF**, so `GetAlertData` sees EOF → then re-`GetFile_Queue` + `Handle_Queue` → NULL | [x] |
| 41 | `Read_FileMon` | `flags = CRALERT_FP_SET` with caller `fp` positioned at 0 on a multi-alert file, `timeout = 0` → alert parsed from the caller's stream | [x] |
| 42 | `Read_FileMon` | `flags = CRALERT_MAIL_SET \| CRALERT_READ_ALL` → the `flag` forwarded into `GetAlertData` comes from `fileq->flags`; mail/non-mail file variants | [x] |
| 43 | `Read_FileMon` | `p` scalars differ from the ones used at `Init_FileQueue` → confirms `day`/`year`/`mon` are re-assigned on the NULL path only | [x] |
| 44 | `Read_FileMon` | `timeout = 1` with `alerts.log` truncated after the seek → exercises the retry loop once (≈5 s per implementation) | [x] |
| 45 | `driver` | randomized `day`/`month`(0..11)/`year`, `timeout = 0`, `flags = CRALERT_READ_ALL`, randomized multi-alert `alerts.log` → returned `alert_data` compared field-by-field | [x] |
| 46 | `driver` | `flags = 0` (seek-to-EOF), `alerts.log` present → NULL from both | [x] |
| 47 | `driver` | `flags = CRALERT_MAIL_SET \| CRALERT_READ_ALL`, mail and non-mail files | [x] |
| 48 | `driver` | all 32 `CRALERT_*` subsets × {`alerts.log` present with one alert, absent} with `timeout = 0` | [x] |
| 49 | `driver` | `alerts.log` containing a syscheck alert → `group`/`filename` fields via the full pipeline | [x] |
| 50 | `FreeAlertData` | on a struct with every pointer set, and on a struct with a mix of NULL/non-NULL pointers (the `os_free` NULL guard) → no crash, both idempotent w.r.t. the returned void | [x] |

## Test mapping

| CONFIGS row(s) | test | file |
|----------------|------|------|
| 1 | `cfg_01_os_calloc_randomized` | `tests/phase_b_lowlevel.rs` |
| 2 | `cfg_02_os_calloc_zero_product` | `tests/phase_b_lowlevel.rs` |
| 3 | `cfg_03_os_realloc_from_null` | `tests/phase_b_lowlevel.rs` |
| 4, 5 | `cfg_04_05_os_realloc_grow_and_shrink` | `tests/phase_b_lowlevel.rs` |
| 6 | `cfg_06_os_strdup_randomized` | `tests/phase_b_lowlevel.rs` |
| 7 | `cfg_07_merror_fseek_template` | `tests/phase_b_lowlevel.rs` |
| 8 | `cfg_08_merror_fstat_template` | `tests/phase_b_lowlevel.rs` |
| 9 | `cfg_09_merror_arbitrary_template` | `tests/phase_b_lowlevel.rs` |
| 10 | `cfg_10_one_complete_alert` | `tests/phase_b_getalertdata.rs` |
| 11 | `cfg_11_minimal_alert` | `tests/phase_b_getalertdata.rs` |
| 12 | `cfg_12_two_alerts_fseek_back` | `tests/phase_b_getalertdata.rs` |
| 13 | `cfg_13_many_alerts` | `tests/phase_b_getalertdata.rs` |
| 14 | `cfg_14_mail_set_matching` | `tests/phase_b_getalertdata.rs` |
| 15 | `cfg_15_mail_set_mixed_file` | `tests/phase_b_getalertdata.rs` |
| 16 | `cfg_16_all_flag_subsets` | `tests/phase_b_getalertdata.rs` |
| 16b | `err_08_mail_set_but_not_mail` | `tests/phase_c_errors.rs` |
| 17 | `cfg_17_header_without_dash` | `tests/phase_b_getalertdata.rs` |
| 18 | `cfg_18_header_dash_leading_spaces` | `tests/phase_b_getalertdata.rs` |
| 19 | `cfg_19_syscheck_filename` | `tests/phase_b_getalertdata.rs` |
| 20 | `cfg_20_syscheck_flag_consumed_by_other_body_line` | `tests/phase_b_getalertdata.rs` |
| 21 | `cfg_21_syscheck_substring_variants` | `tests/phase_b_getalertdata.rs` |
| 22 | `cfg_22_duplicate_token_lines` | `tests/phase_b_getalertdata.rs` |
| 23 | `cfg_23_port_atoi_ranges` | `tests/phase_b_getalertdata.rs` |
| 24 | `cfg_24_rule_level_atoi_ranges` | `tests/phase_b_getalertdata.rs` |
| 25 | `cfg_25_comment_embedded_quotes` | `tests/phase_b_getalertdata.rs` |
| 26 | `cfg_26_dateline_colon_space_shapes` | `tests/phase_b_getalertdata.rs` |
| 27 | `cfg_27_no_trailing_newline` | `tests/phase_b_getalertdata.rs` |
| 28 | `cfg_28_fgets_boundary_lines` | `tests/phase_b_getalertdata.rs` |
| 29 | `cfg_29_stream_kinds_agree` | `tests/phase_b_getalertdata.rs` |
| 30 | `cfg_30_init_flags0_file_present` | `tests/phase_b_filequeue.rs` |
| 31 | `cfg_31_init_read_all` | `tests/phase_b_filequeue.rs` |
| 32, 33 | `cfg_32_33_init_fp_set_with_caller_stream` | `tests/phase_b_filequeue.rs` |
| 34 | `cfg_34_init_stdin_named_file_ignored_without_fp_set` | `tests/phase_b_filequeue.rs` |
| 35 | `cfg_35_init_fp_set_null_fp` | `tests/phase_b_filequeue.rs` |
| 36 | `cfg_36_init_full_flag_matrix` | `tests/phase_b_filequeue.rs` |
| 37 | `cfg_37_init_tm_scalars` | `tests/phase_b_filequeue.rs` |
| 38 | `cfg_38_init_pre_dirtied_struct` | `tests/phase_b_filequeue.rs` |
| 39 | `cfg_39_read_filemon_walks_file` | `tests/phase_b_filequeue.rs` |
| 40 | `cfg_40_read_filemon_after_seek_to_eof` | `tests/phase_b_filequeue.rs` |
| 41 | `cfg_41_read_filemon_from_caller_stream` | `tests/phase_b_filequeue.rs` |
| 42 | `cfg_42_read_filemon_forwards_mail_flag` | `tests/phase_b_filequeue.rs` |
| 43 | `cfg_43_read_filemon_reassigns_tm` | `tests/phase_b_filequeue.rs` |
| 44 | `cfg_44_read_filemon_timeout_one` | `tests/phase_b_filequeue.rs` |
| 45 | `cfg_45_driver_read_all_randomized` | `tests/phase_b_filequeue.rs` |
| 46 | `cfg_46_driver_flags_zero` | `tests/phase_b_filequeue.rs` |
| 47 | `cfg_47_driver_mail_variants` | `tests/phase_b_filequeue.rs` |
| 48 | `cfg_48_driver_full_flag_matrix` | `tests/phase_b_filequeue.rs` |
| 49 | `cfg_49_driver_syscheck_alert` | `tests/phase_b_filequeue.rs` |
| 50 | `cfg_50_free_alert_data_mixed_null` | `tests/phase_b_getalertdata.rs` |
| (cross-product fuzzing over every row above) | `cfg_fuzz_line_grammar`, `cfg_fuzz_random_bytes`, `cfg_fuzz_random_bytes_with_headers` | `tests/phase_b_getalertdata.rs` |

Every row's differential compares, for BOTH `.so`s:
the full `alert_data` (all 13 fields, strings by bytes and NULL-vs-empty),
the full `file_queue` (`last_change`, `year`, `day`, `flags`, all 4 bytes of
`mon`, all 257 bytes of `file_name`, `fp` NULL-ness and 12 `struct stat`
fields), the return code, the stream position / `feof` / `ferror` after every
call, and anything written to `stderr`.
