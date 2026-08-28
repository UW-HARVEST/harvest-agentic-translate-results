# CONFIGS.md — Phase A: configuration-surface table (valid inputs)

## Axes derived from the C source

### A. Public entry points (all 9 exported symbols — `SYMBOLS.md`)

Lowest → highest level:

1. `os_calloc(num,size)`, `os_realloc(ptr,new_size)`, `os_strdup(str)` — `shared.h`
2. `merror(err_template,file_name,err,err_msg)` — `file-queue.c:24`
3. `GetAlertData(flag, FILE *fp)` / `FreeAlertData(alert_data*)` — `read-alert.c`
4. `Init_FileQueue(file_queue*, const struct tm*, int flags)` — `file-queue.c:113`
5. `Read_FileMon(file_queue*, const struct tm*, unsigned timeout)` — `file-queue.c:143`
6. `driver(day,month,year,timeout,flags)` — `driver.c:6` (one-shot wrapper)

### B. Runtime option flags (`read-alert.h:18-22`) and where the C branches on them

| flag | value | branches |
|------|-------|----------|
| `CRALERT_MAIL_SET`    | 0x001 | `read-alert.c:139` — only accept alerts whose flag word starts with `mail` |
| `CRALERT_EXEC_SET`    | 0x002 | never tested — inert |
| `CRALERT_READ_ALL`    | 0x004 | `file-queue.c:84` — skip the `fseek(fp,0,SEEK_END)`, i.e. read from position 0 |
| `CRALERT_READ_FAILED` | 0x008 | never tested — inert |
| `CRALERT_FP_SET`      | 0x010 | `file-queue.c:60` (name becomes `<stdin>` instead of `alerts.log`), `file-queue.c:67` (do not `fopen`/`fclose`, reuse caller's `fp`), `file-queue.c:116` (do not reset `fp` to NULL) |

Note the two *asymmetries* the C deliberately has, which the tests must pin down:
`Handle_Queue` is called with `fileq->flags` from `Init_FileQueue` but with a
hard-coded `0` from both call sites in `Read_FileMon`, so `Read_FileMon` always
re-`fopen`s and always seeks to EOF; and `GetAlertData` receives `fileq->flags`
(so only `CRALERT_MAIL_SET` matters there).

### C. Input shapes the code special-cases

* `struct tm`: `tm_mday`, `tm_mon` (0..11 valid — indexes the 12-entry
  `s_month` table), `tm_year` (`+1900`).
* queue file: absent / present-empty / present-with-data; name `alerts.log`
  vs `<stdin>` (selected by `CRALERT_FP_SET`); seek position 0 vs EOF.
* `timeout`: 0 (loop never entered) vs ≥1 (each miss costs `FQ_TIMEOUT`=5 s).
* alert stream shape (`read-alert.c`): `** Alert` header present/absent,
  header with/without `:`, with/without space, with/without `-` group part,
  group containing `syscheck` or not, `mail` token or not; body line 1
  (date/location) with `:`+space; then any mix of `Rule: `, `Src IP: `,
  `Src Port: `, `Dst IP: `, `Dst Port: `, `User: `, `Integrity checksum
  changed for: '…'`, arbitrary log lines; 0/1/many alerts per file;
  trailing newline present/absent; line lengths below/at/above
  `OS_MAXSTR-1`=1023; duplicated fields (each does `os_free` + `os_strdup`,
  except `filename` which leaks by design).
* `alert_data` numeric fields go through glibc `atoi` (garbage → 0, overflow →
  truncated `long`), `rule`/`level` are `unsigned int` so negatives wrap.

### D. `#ifdef` axes

None — `c_src` has no conditional compilation, and `Cargo.toml` declares no
`[features]`. There is therefore exactly one build configuration; the
`scripts/check_features.sh` loop confirms this and re-runs everything under
`--no-default-features` too.

---

## Configuration table

Each row is run against BOTH `.so`s through `libloading`, with many
randomized inputs (fixed-seed xorshift PRNG in `tests/common/mod.rs`).

| #  | entry point(s) | configuration (options set + input shape) | test | [x] |
|----|----------------|-------------------------------------------|------|-----|
| 1  | `os_calloc` | random `num`×`size` (0…4096, incl. `num=0`, `size=0`) — result must be non-NULL and fully zeroed | `low_level.rs::cfg01_os_calloc_random` | [x] |
| 2  | `os_realloc` | `ptr=NULL` + random size (acts as `malloc`); then grow/shrink an existing block through a random size sequence, checking the retained prefix bytes | `low_level.rs::cfg02_os_realloc_random` | [x] |
| 3  | `os_strdup` | random byte strings, len 0…512, including embedded high bytes and all-`0xff` | `low_level.rs::cfg03_os_strdup_random` | [x] |
| 4  | `merror` | `FSEEK_ERROR`/`FSTAT_ERROR`/custom templates × random file names (0…400 bytes) × random `errno` values × `strerror` messages; stderr captured and compared byte-for-byte | `low_level.rs::cfg04_merror_random` | [x] |
| 5  | `FreeAlertData` | heap `alert_data` with every subset of the 9 owned `char*` fields populated (2⁹ combinations, randomized contents), plus cross-library frees | `low_level.rs::cfg05_freealertdata_field_subsets` | [x] |
| 5b | ABI | `sizeof`/`alignof` of `alert_data` (96), `struct stat` (144) and `file_queue` (440), and proof that `Init_FileQueue` writes nothing past the end of `file_queue` in either library | `low_level.rs::cfg05b_struct_layout_matches_c` | [x] |
| 6  | `GetAlertData` | `flag=0`, single complete non-syscheck alert, all optional fields present, trailing newline | `get_alert_data.rs::cfg06_single_full_alert` | [x] |
| 7  | `GetAlertData` | `flag=0`, single alert, randomized *subsets/orderings* of the `Rule:`/`Src IP:`/`Src Port:`/`Dst IP:`/`Dst Port:`/`User:`/log lines | `get_alert_data.rs::cfg07_random_field_subsets` | [x] |
| 8  | `GetAlertData` | `flag=0`, duplicated field lines (`Src IP:` etc. twice or more) → last-wins via `os_free`+`os_strdup` | `get_alert_data.rs::cfg08_duplicate_fields` | [x] |
| 9  | `GetAlertData` | `flag=0`, **multiple** alerts in one stream; called repeatedly until NULL, comparing every returned record *and* `ftell` after each call (exercises the `fseek(-strlen)` push-back) | `get_alert_data.rs::cfg09_multi_alert_sequence` | [x] |
| 10 | `GetAlertData` | `flag=CRALERT_MAIL_SET`, stream mixing `mail` and non-`mail` alerts → only `mail` ones accepted | `get_alert_data.rs::cfg10_mail_flag_filtering` | [x] |
| 11 | `GetAlertData` | `flag` = random 32-bit word (undefined bits set) with a fixed stream → behaviour must depend only on bit 0x001 | `get_alert_data.rs::cfg11_random_flag_words` | [x] |
| 12 | `GetAlertData` | group field with `-` and randomized leading spaces / no `-` at all / group containing `syscheck` at random positions | `get_alert_data.rs::cfg12_group_and_syscheck_detection` | [x] |
| 13 | `GetAlertData` | syscheck alert + `Integrity checksum changed for: '…'` line: the filename is taken from the *first* body line after the group was flagged, and one trailing byte is chopped | `get_alert_data.rs::cfg13_syscheck_filename` | [x] |
| 14 | `GetAlertData` | alert with no trailing newline at EOF (`feof && _r==2` path); also the bare `** Alert` / `** Aler` / `** Alertx` last-line shapes that make `p = str+9` run past the NUL `fgets` wrote | `get_alert_data.rs::cfg14_no_trailing_newline` | [x] |
| 15 | `GetAlertData` | `\r\n` (CRLF) line endings — `os_clearnl` only strips `\n`, so the `\r` stays in the values | `get_alert_data.rs::cfg15_crlf_line_endings` | [x] |
| 16 | `GetAlertData` | body lines at length 1022 / 1023 / 1024 / 1025 / 2050 around the `OS_MAXSTR` `fgets` chunk boundary | `get_alert_data.rs::cfg16_os_maxstr_boundaries` | [x] |
| 17 | `GetAlertData` | fully randomized alert generator (headers, fields, junk lines, 0–6 alerts per file, 400 seeded cases) — the property-style sweep | `get_alert_data.rs::cfg17_fuzz_random_streams` | [x] |
| 18 | `GetAlertData` | totally random bytes / random line soup with no alert structure (200 seeded cases) | `get_alert_data.rs::cfg18_fuzz_random_bytes` | [x] |
| 19 | `GetAlertData` | `fp` positioned in the middle of the stream by the caller (`fseek` before the call) | `get_alert_data.rs::cfg19_preseeked_stream` | [x] |
| 20 | `Init_FileQueue` | `flags=0`, `alerts.log` present, random `tm` (`tm_mday`, `tm_mon` 0..11, `tm_year`) → compares the whole `file_queue` (last_change, year, day, flags, mon, file_name) + `ftell(fp)` (EOF) + return value | `file_queue.rs::cfg20_init_flags0_present` | [x] |
| 21 | `Init_FileQueue` | `flags=CRALERT_READ_ALL`, `alerts.log` present → no `fseek`, `ftell(fp)==0` | `file_queue.rs::cfg21_init_read_all` | [x] |
| 22 | `Init_FileQueue` | `flags=CRALERT_FP_SET`, caller supplies an already-open `FILE*` on a real file → `fp` kept, seeked to EOF, `file_name=="<stdin>"` | `file_queue.rs::cfg22_init_fp_set` | [x] |
| 23 | `Init_FileQueue` | `flags=CRALERT_FP_SET|CRALERT_READ_ALL` → `fp` kept at position 0, `file_name=="<stdin>"` | `file_queue.rs::cfg23_init_fp_set_read_all` | [x] |
| 24 | `Init_FileQueue` | `flags` = every combination of the 5 defined `CRALERT_*` bits (32 rows) × {`alerts.log` present, absent} × {`fp` preset, NULL} | `file_queue.rs::cfg24_init_all_flag_combos` | [x] |
| 25 | `Init_FileQueue` | garbage pre-filled `file_queue` (random bytes) to prove every field the C writes is written identically and the rest is left alone | `file_queue.rs::cfg25_init_prefilled_struct` | [x] |
| 26 | `Read_FileMon` | `timeout=0`, `flags=CRALERT_READ_ALL`, `alerts.log` holds 1 alert → first `GetAlertData` on the position-0 `fp` returns it | `file_queue.rs::cfg26_readfilemon_read_all_hit` | [x] |
| 27 | `Read_FileMon` | `timeout=0`, `flags=0` → `fp` is at EOF so the first `GetAlertData` misses, the queue is re-opened+seeked to EOF, loop not entered → `NULL`, and the post-call `file_queue` (day/year/mon/file_name/last_change) is compared | `file_queue.rs::cfg27_readfilemon_flags0_miss` | [x] |
| 28 | `Read_FileMon` | `timeout=0`, `flags=CRALERT_READ_ALL|CRALERT_MAIL_SET`, mixed mail/non-mail stream | `file_queue.rs::cfg28_readfilemon_mail_filter` | [x] |
| 29 | `Read_FileMon` | called repeatedly on the same `file_queue` to drain a multi-alert file (state carried across calls) | `file_queue.rs::cfg29_readfilemon_repeated` | [x] |
| 30 | `Read_FileMon` | `flags=CRALERT_FP_SET` with a real file named `<stdin>` in cwd (so the hard-coded `Handle_Queue(fileq,0)` re-open succeeds) | `file_queue.rs::cfg30_readfilemon_fp_set_stdin_file` | [x] |
| 31 | `Read_FileMon` | random `tm` values written into the queue by the miss path (`day`, `year`, `mon`) | `file_queue.rs::cfg31_readfilemon_tm_fields` | [x] |
| 32 | `driver` | `flags=CRALERT_READ_ALL`, `alerts.log` with a randomized alert corpus, `timeout=0`, random day/month(0..11)/year (300 seeded cases) | `driver.rs::cfg32_driver_read_all_random` | [x] |
| 33 | `driver` | `flags=0` (seek to EOF ⇒ nothing to read) | `driver.rs::cfg33_driver_flags0` | [x] |
| 34 | `driver` | `flags=CRALERT_READ_ALL|CRALERT_MAIL_SET` × mail/non-mail corpora | `driver.rs::cfg34_driver_mail_filter` | [x] |
| 35 | `driver` | `flags=CRALERT_FP_SET|CRALERT_READ_ALL` with a `<stdin>` file present in cwd | `driver.rs::cfg35_driver_fp_set` | [x] |
| 36 | `driver` | every combination of the 5 defined `CRALERT_*` bits (32 rows) with both `alerts.log` and `<stdin>` present, `timeout=0` | `driver.rs::cfg36_driver_all_flag_combos` | [x] |
| 37 | `driver` | `alerts.log` empty / absent / unreadable-but-present; `timeout=0` | `driver.rs::cfg37_driver_degenerate_queue_files` | [x] |
| 38 | `driver` | random `day`/`year` incl. negative and `INT_MIN`/`INT_MAX` (only `mon` is range-restricted, see `ERRORS.md` #27) | `driver.rs::cfg38_driver_extreme_day_year` | [x] |
| 39 | `Read_FileMon` | `file_queue` **hand-built by the caller** (not produced by `Init_FileQueue`): caller-chosen `file_name`, `flags`, `day`/`year`/`mon`/`last_change`, and `fp` ∈ {NULL, open at 0, open at EOF, pipe} × 6 flag settings | `file_queue.rs::cfg39_readfilemon_handcrafted_queue` | [x] |
| 40 | `Init_FileQueue` + `Read_FileMon` | interleaved / repeated calls on the same `file_queue` (re-init after a read, re-init with different flags, several reads in a row) | `file_queue.rs::cfg40_init_then_reinit_and_interleave` | [x] |
| 41 | `GetAlertData` | non-seekable stream (pipe) — 64 inputs incl. the randomized generator; the `fseek` push-back is never reached with a single alert | `get_alert_data.rs::cfg41_nonseekable_stream` | [x] |
| 42 | `GetAlertData` | stream where `fgets` fails outright: a directory stream (`open` succeeds on Linux, `read` gives `EISDIR`) | `get_alert_data.rs::cfg42_directory_stream` | [x] |
