# CONFIGS.md — configuration surface table (Phase B gate)

Derived mechanically from the branch points the C actually takes. Sources of
truth: the two public headers plus every `if` / `else if` / ternary in
`file-queue.c`, `read-alert.c`, `driver.c`.

## The axes the C branches on

### Axis 1 — public entry points (ALL of them, lowest level first)

| level | symbol | header |
|-------|--------|--------|
| 0 | `os_calloc`, `os_realloc`, `os_strdup` | `shared.h` |
| 0 | `merror` | (no prototype; external linkage in `file-queue.c`) |
| 1 | `GetAlertData(int flag, FILE *fp)` | `read-alert.h` |
| 1 | `FreeAlertData(alert_data *)` | `read-alert.h` |
| 2 | `Init_FileQueue(file_queue *, const struct tm *, int flags)` | `file-queue.h` |
| 2 | `Read_FileMon(file_queue *, const struct tm *, unsigned timeout)` | `file-queue.h` |
| 3 | `driver(int day, int month, int year, unsigned timeout, int flags)` | (one-shot convenience wrapper) |

The `static` helpers `file_sleep`, `GetFile_Queue`, `Handle_Queue` are reached
only through levels 2–3, so levels 2–3 must be driven directly rather than only
through `driver`.

### Axis 2 — runtime flags (`read-alert.h`) and what each toggles

| bit | name | branches it controls |
|-----|------|----------------------|
| `0x001` | `CRALERT_MAIL_SET` | `read-alert.c:139` — alert header must start with `mail` after the first space, else the header is dropped |
| `0x002` | `CRALERT_EXEC_SET` | **never tested** anywhere in the C → inert; must stay inert in Rust |
| `0x004` | `CRALERT_READ_ALL` | `file-queue.c:82` — skip the `fseek(fp,0,SEEK_END)`, i.e. read from offset 0 instead of from EOF |
| `0x008` | `CRALERT_READ_FAILED` | **never tested** → inert |
| `0x010` | `CRALERT_FP_SET` | `file-queue.c:57` (`file_name` becomes `"<stdin>"` instead of `"alerts.log"`), `file-queue.c:66` (skip `fclose`/`fopen`, adopt the caller's `fp`), `file-queue.c:114` (skip `fileq->fp = NULL`) |

Interaction that matters: `Read_FileMon` calls `Handle_Queue(fileq, **0**)`, not
`fileq->flags`. So a queue opened with `FP_SET` and/or `READ_ALL` is re-handled
with those bits *cleared* on the recovery path — while `file_name` still holds
whatever `GetFile_Queue` derived from `fileq->flags`. That asymmetry is a real
configuration axis and is exercised below.

### Axis 3 — input shapes `read-alert.c` special-cases

`_r` state machine (0 → 1 → 2), plus per-line prefix dispatch on
`** Alert` / `Rule: ` / `Src IP: ` / `Src Port: ` / `Dst IP: ` / `Dst Port: ` /
`User: ` / else-log, plus the `syscheck` sub-mode
(`Integrity checksum changed for: '`), plus line-length (`OS_MAXSTR` = 1024) and
trailing-newline presence.

### Axis 4 — `struct tm` fields consumed

`tm_mday` (copied to `fileq->day`), `tm_mon` (index into `s_month[12]`),
`tm_year` (`+1900` into `fileq->year`). No other field is read.

### Axis 5 — `timeout`

Only reached when the *first* `GetAlertData` returns NULL; each iteration costs a
5 s `select`. Tested at `0` and `1`.

### Axis 6 — build features

`Cargo.toml` has no `[features]` table ⇒ exactly one configuration. Enumerated
mechanically by `scripts/check_features.sh`.

---

## Configuration rows

Each row is driven with **many randomized inputs** (fixed seed `0x5EED_1234`,
xorshift64* PRNG in `tests/common/mod.rs`) unless the row is inherently a single
shape. `[x]` = passes across all randomized inputs for that row.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C01 | `os_calloc`, `os_realloc`, `os_strdup` | randomized sizes/strings incl. `num=0`, `size=0`, empty string, 1 byte, 4 KiB, embedded high bytes; `os_realloc` grow **and** shrink from a live pointer | `cfg_shared.rs::c01_alloc_helpers` | [x] |
| C02 | `Init_FileQueue` | `tm_mon` = 0..11 (all 12 `s_month` entries) × flags `READ_ALL` (so init succeeds), checking `fileq->mon[0..4]`, `day`, `year`, `file_name`, `flags`, `last_change` | `cfg_shapes.rs::c02_all_months` | [x] |
| C03 | `Init_FileQueue` | randomized `tm_mday`/`tm_year` incl. `INT_MIN`, `INT_MAX`, `-1900`, `0`, `INT_MAX-1900` (year overflow wraps in C) | `cfg_shapes.rs::c03_day_year_extremes` | [x] |
| C04 | `Init_FileQueue` | flags = 0 (no bits): `alerts.log` present ⇒ `fopen` + `fseek(END)` + `fstat`, `file_name=="alerts.log"`, `last_change==st_mtime`, returns 0 | `cfg_queue.rs::c04_init_default` | [x] |
| C05 | `Init_FileQueue` | flags = `READ_ALL`: no seek to end, so `fp` is left at offset 0 | `cfg_queue.rs::c05_init_read_all` | [x] |
| C06 | `Init_FileQueue` | flags = `FP_SET` with a caller-supplied seekable `fp`: `file_name=="<stdin>"`, no reopen, seek-to-end still happens | `cfg_queue.rs::c06_init_fp_set` | [x] |
| C07 | `Init_FileQueue` | flags = `FP_SET\|READ_ALL` with caller-supplied `fp`: no reopen **and** no seek ⇒ offset preserved exactly as the caller left it (randomized starting offsets) | `cfg_queue.rs::c07_init_fp_set_read_all` | [x] |
| C08 | `Init_FileQueue` | flags = `FP_SET\|READ_ALL` with `fp == NULL`: `fstat` skipped, `last_change` taken from the (zeroed) `f_status`, returns 0 | `cfg_queue.rs::c08_init_fp_set_null_fp_read_all` | [x] |
| C09 | `Init_FileQueue` | inert bits only (`EXEC_SET`, `READ_FAILED`) and randomized ints containing them ⇒ must behave exactly like the same value with those bits cleared | `cfg_queue.rs::c09_inert_bits` | [x] |
| C10 | `Init_FileQueue` + `Read_FileMon` | full pipeline, flags = `READ_ALL`, `alerts.log` holding 1 complete alert ⇒ alert returned from the *first* `GetAlertData`, no sleep | `cfg_queue.rs::c10_pipeline_read_all_one_alert` | [x] |
| C11 | `Init_FileQueue` + `Read_FileMon` | full pipeline, flags = `READ_ALL`, `alerts.log` holding **many** alerts ⇒ repeated `Read_FileMon` walks them one at a time via the `fseek`-back, until the stream is exhausted | `cfg_queue.rs::c11_pipeline_many_alerts` | [x] |
| C12 | `Init_FileQueue` + `Read_FileMon` | flags = 0 (seek-to-end): first `GetAlertData` sees EOF, recovery reopens+reseeks to end, `timeout=0` ⇒ NULL with no sleep. Also `timeout=1` ⇒ exactly one 5 s sleep | `cfg_queue.rs::c12_pipeline_seek_end` | [x] |
| C13 | `driver` | `day`/`month`/`year`/`timeout`/`flags` randomized over the *valid* month domain, `flags` = `READ_ALL` (+ random inert bits), randomized alert files ⇒ identical `alert_data` | `cfg_driver.rs::c13_driver_read_all` | [x] |
| C14 | `GetAlertData` | line-length shapes: exactly 1023 / 1024 / 1025 bytes, 64 KiB single line, no trailing newline at EOF, `\r\n`, NUL-free binary bytes | `cfg_shapes.rs::c14_oversized_line` | [x] |
| C15 | `GetAlertData` | `flag = 0`, minimal well-formed alert (`** Alert 1234.5: something` + date/location + `Rule: `) | `cfg_alert.rs::c15_minimal_alert` | [x] |
| C16 | `GetAlertData` | `flag = 0`, alert exercising **every** field prefix: `Rule: `, `Src IP: `, `Src Port: `, `Dst IP: `, `Dst Port: `, `User: `, plus free-form log lines | `cfg_alert.rs::c16_all_fields` | [x] |
| C17 | `GetAlertData` | `flag = 0`, field prefixes in **randomized order**, randomized presence/absence, randomized duplicate prefixes (each duplicate `os_free`s and replaces the previous value) | `cfg_alert.rs::c17_random_field_orders` | [x] |
| C18 | `GetAlertData` | `flag = 0`, `Src Port: ` / `Dst Port: ` / `Rule: ` numeric shapes fed to `atoi`: empty, `+`/`-`, leading spaces, `2147483647`, `2147483648`, `-2147483649`, `999999999999`, hex-looking, trailing junk | `cfg_alert.rs::c18_atoi_shapes` | [x] |
| C19 | `GetAlertData` | `flag = 0`, alertid shapes: `strstr(p,":")` at various offsets incl. offset 0 (`z == 0` ⇒ empty alertid), colon as last char, multiple colons | `cfg_alert.rs::c19_alertid_shapes` | [x] |
| C20 | `GetAlertData` | `flag = 0`, group parsing: `-` present/absent, 0..N leading spaces after `-`, group containing `syscheck` as substring vs not, group with trailing newline vs at EOF | `cfg_alert.rs::c20_group_shapes` | [x] |
| C21 | `GetAlertData` | `flag = 0`, `group` contains `syscheck` **and** the next non-prefix line is `Integrity checksum changed for: '<path>'` ⇒ `filename` set and its last byte stripped. Randomized paths incl. 1-char and long paths | `cfg_alert.rs::c21_syscheck_filename` | [x] |
| C22 | `GetAlertData` | `flag = 0`, group contains `syscheck` but the first log line is **not** the integrity prefix ⇒ `issyscheck` reset to 0, `filename` stays NULL even if a later line matches | `cfg_alert.rs::c22_syscheck_one_shot` | [x] |
| C23 | `GetAlertData` | `flag = CRALERT_MAIL_SET`, header token **is** `mail` ⇒ accepted | `cfg_alert.rs::c23_mail_accepted` | [x] |
| C24 | `GetAlertData` | `flag = CRALERT_MAIL_SET`, mixture of `mail` and non-`mail` headers in one file ⇒ only `mail` alerts are picked up, and the `_r` state machine's position after the skipped headers matches | `cfg_alert.rs::c24_mail_mixed` | [x] |
| C25 | `GetAlertData` | `flag` = every value in `0x00..0x1F` (full cross-product of the five documented bits) × 6 representative file shapes | `cfg_alert.rs::c25_flag_cross_product` | [x] |
| C26 | `GetAlertData` | called repeatedly on the same `FILE*` until NULL, over randomized multi-alert files ⇒ the whole `fseek`-back sequence and final `ftell`/`feof` match | `cfg_alert.rs::c26_sequential_drain` | [x] |
| C27 | `GetAlertData` | `fp` opened in `"r"` at a randomized non-zero starting offset (possibly mid-line) | `cfg_alert.rs::c27_random_start_offset` | [x] |
| C28 | `GetAlertData` | fully randomized fuzz corpus: lines drawn from a weighted alphabet of every recognised prefix, malformed variants, blank lines and random bytes; 400 files × random `flag` | `cfg_alert.rs::c28_fuzz_corpus` | [x] |
| C29 | `FreeAlertData` | a fully populated `alert_data` (all 9 owned pointers non-NULL), a fully NULL one, and randomized partial mixes ⇒ no crash, and the struct is scrubbed identically before the final `free` | `cfg_shared.rs::c29_free_alert_data` | [x] |
| C30 | `merror` | both format templates (`FSTAT_ERROR`, `FSEEK_ERROR`) plus randomized `file_name` / `err` / `err_msg`, including a `file_name` long enough to truncate the 256-byte `snprintf` buffer | `cfg_shared.rs::c30_merror` | [x] |
