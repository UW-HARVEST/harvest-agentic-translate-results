# CONFIGS.md — Phase B configuration-surface table

## Axes the C code actually branches on

**A. Runtime flag bits** (`include/read-alert.h:18-22`), consumed as an `int` bitmask:

| bit | name | where the C branches on it |
|-----|------|----------------------------|
| `0x001` | `CRALERT_MAIL_SET`   | `read-alert.c:139` — requires the token after the alert id to be `mail`, else the header line is skipped |
| `0x002` | `CRALERT_EXEC_SET`   | never read — inert |
| `0x004` | `CRALERT_READ_ALL`   | `file-queue.c:84` — skips the `fseek(fp,0,SEEK_END)`, so parsing starts at offset 0 |
| `0x008` | `CRALERT_READ_FAILED`| never read — inert |
| `0x010` | `CRALERT_FP_SET`     | `file-queue.c:60` — file name becomes `<stdin>`; `file-queue.c:67` / `116` — do not `fclose`/`fopen`/reset `fp`, i.e. use the caller's `fp` |

Flags reach two places: `fileq->flags` (used by `Init_FileQueue` → `Handle_Queue`,
and passed to `GetAlertData` as `flag`) and the literal `0` that `Read_FileMon`
passes to its own `Handle_Queue` calls (`file-queue.c:150,171` — so a re-open
always seeks to EOF regardless of `CRALERT_READ_ALL`).

**B. `struct tm` fields** — `tm_mday` → `fileq->day`, `tm_year` → `fileq->year + 1900`,
`tm_mon` → `s_month[tm_mon]` copied into `fileq->mon` (3 bytes, `strncpy`).

**C. `timeout`** (`unsigned int`) — retry count in `Read_FileMon`; `0` = no retry
loop, `n > 0` = `n` iterations each with a 5 s `file_sleep()`.

**D. Alert-file input shapes** — the parser state machine `_r ∈ {0,1,2}` and the
`strncmp` prefix ladder in `GetAlertData`:
`** Alert` header, date/location line, `Rule: `, `Src IP: `, `Src Port: `,
`Dst IP: `, `Dst Port: `, `User: `, `Integrity checksum changed for: '`,
arbitrary log lines; group containing `syscheck`; multi-alert push-back via
`fseek(-strlen(str),SEEK_CUR)`; `OS_MAXSTR`(1024)-byte `fgets` line splitting;
missing trailing newline; empty file.

**E. Entry points** — lowest level first:
`os_calloc`, `os_realloc`, `os_strdup`, `merror`, `FreeAlertData`,
`GetAlertData(flag, FILE*)`, `Init_FileQueue(file_queue*, tm*, flags)`,
`Read_FileMon(file_queue*, tm*, timeout)`, `driver(day,month,year,timeout,flags)`.

`GetAlertData` and `Init_FileQueue`/`Read_FileMon` are exercised **directly**, not
only through the `driver` one-shot wrapper.

## Configuration rows

All rows are driven with many randomized inputs (fixed seed, xorshift PRNG in
`tests/common/mod.rs`), asserting byte-for-byte equality of every scalar and
every string field of `alert_data` (and of `file_queue` for the `Init_FileQueue`
rows), plus the returned pointer's NULL-ness.

| #  | entry point(s) | configuration (options set + input shape) | ✔ |
|----|----------------|-------------------------------------------|---|
| 1  | `os_calloc` | randomized `(num,size)` incl. `(0,n)`, `(n,0)`, `(1,1)`, large; compare zero-fill + non-NULL | [x] |
| 2  | `os_realloc` | `ptr=NULL` grow, grow existing, shrink existing, randomized sizes; content preserved | [x] |
| 3  | `os_strdup` | randomized byte strings: empty, 1 byte, 1..4096 bytes, high-bit bytes, embedded spaces/quotes | [x] |
| 4  | `merror` | both real templates (`FSTAT_ERROR`, `FSEEK_ERROR`) × randomized `file_name`/`err`/`err_msg`, incl. names long enough to truncate the 256-byte buffer; stderr captured and compared | [x] |
| 5  | `FreeAlertData` | struct with all-NULL string fields; all fields set; a mix — cross-freed (C-allocated struct freed by Rust and vice versa) | [x] |
| 6  | `GetAlertData` | `flag=0`, `fp` at offset 0, single well-formed alert: header + date/location + `Rule:` + log lines | [x] |
| 7  | `GetAlertData` | `flag=0`, single alert with **every** recognized field: `Rule:`, `Src IP:`, `Src Port:`, `Dst IP:`, `Dst Port:`, `User:` (randomized values incl. negative / huge / non-numeric ports) | [x] |
| 8  | `GetAlertData` | `flag=0`, field lines in **randomized order** and randomly present/absent (2^6 subsets sampled) | [x] |
| 9  | `GetAlertData` | `flag=0`, repeated duplicate field lines (`Src IP:` twice, `Rule:` twice, `User:` 3×) — exercises the `os_free` + re-`strdup` paths and last-wins semantics | [x] |
| 10 | `GetAlertData` | `flag=0`, **multi-alert** file: successive calls on the same `FILE*` must return alert 1, then alert 2, … via the `fseek` push-back; compare the whole sequence | [x] |
| 11 | `GetAlertData` | `flag=CRALERT_MAIL_SET`, header token is `mail` (accepted) | [x] |
| 12 | `GetAlertData` | `flag=CRALERT_MAIL_SET`, header token is not `mail` (rejected → NULL) | [x] |
| 13 | `GetAlertData` | `flag=CRALERT_MAIL_SET`, mixed file: some alerts `mail`, some not | [x] |
| 14 | `GetAlertData` | `flag=0`, group **contains** `syscheck` + `Integrity checksum changed for: '<path>'` line ⇒ `filename` set (randomized paths incl. 1-char and 300-char) | [x] |
| 15 | `GetAlertData` | `flag=0`, group contains `syscheck` but **no** integrity line ⇒ `filename` NULL; `issyscheck` cleared after the first log line | [x] |
| 16 | `GetAlertData` | `flag=0`, group does **not** contain `syscheck` but an integrity line is present ⇒ `filename` NULL | [x] |
| 17 | `GetAlertData` | `flag=0`, integrity line is **not** the first log line after the header ⇒ `issyscheck` already cleared ⇒ `filename` NULL | [x] |
| 18 | `GetAlertData` | `flag=0`, alert id shapes: no `'-'` in header (group stays NULL), multiple `'-'`, many leading spaces after `'-'`, `'-'` before the `':'` | [x] |
| 19 | `GetAlertData` | `flag=0`, line length boundaries around `OS_MAXSTR`: 1022 / 1023 / 1024 / 1025 / 4096-byte lines split by `fgets` into continuation chunks | [x] |
| 20 | `GetAlertData` | `flag=0`, no trailing newline on the last line; `\r\n` line endings; blank lines inside the alert | [x] |
| 21 | `GetAlertData` | `flag=0`, empty file, whitespace-only file, file of only log lines (no header) | [x] |
| 22 | `GetAlertData` | `flag` = randomized full `int` incl. inert bits `CRALERT_EXEC_SET`/`CRALERT_READ_FAILED`, `CRALERT_READ_ALL`, `CRALERT_FP_SET`, and unknown bits (`0x20`, `-1`, `INT_MIN`) — only `0x1` may alter behaviour | [x] |
| 23 | `GetAlertData` | `flag=0`, `fp` at a **non-zero** start offset (mid-file, mid-line) — mirrors the post-`fseek(END)` / re-read states | [x] |
| 24 | `Init_FileQueue` | `flags=0` (default), `alerts.log` present ⇒ `fp` opened + seeked to EOF; compare all `file_queue` fields (`last_change`, `year`, `day`, `flags`, `mon`, `file_name`) and `ftell(fp)` | [x] |
| 25 | `Init_FileQueue` | `flags=CRALERT_READ_ALL`, `alerts.log` present ⇒ `fp` at offset 0 | [x] |
| 26 | `Init_FileQueue` | `flags=CRALERT_FP_SET` ⇒ `file_name` is `<stdin>`, caller's `fp` reused (not re-opened), seeked to EOF | [x] |
| 27 | `Init_FileQueue` | `flags=CRALERT_FP_SET\|CRALERT_READ_ALL` ⇒ `<stdin>`, caller's `fp` untouched (no seek) | [x] |
| 28 | `Init_FileQueue` | randomized `tm_mday` / `tm_year` (incl. negative, 0, 31, huge) × `tm_mon` 0..11 ⇒ `day`, `year+1900`, `mon` 3-byte `strncpy` | [x] |
| 29 | `Init_FileQueue` | `alerts.log` **absent**, `flags=0` ⇒ `Handle_Queue` returns 0, `Init` returns 0, `fp` NULL, `last_change` 0 | [x] |
| 30 | `Init_FileQueue` | `alerts.log` present but **empty** / present with size > 0 ⇒ `last_change` == file `st_mtime`, EOF offset == size | [x] |
| 31 | `Read_FileMon` | `flags=CRALERT_READ_ALL`, `timeout=0`, populated `alerts.log` ⇒ returns the first alert without sleeping | [x] |
| 32 | `Read_FileMon` | `flags=CRALERT_READ_ALL`, `timeout=0`, successive calls drain multiple alerts from the same `file_queue` | [x] |
| 33 | `Read_FileMon` | `flags=CRALERT_READ_ALL\|CRALERT_MAIL_SET`, `timeout=0`, mixed mail/non-mail alerts | [x] |
| 34 | `Read_FileMon` | `flags=CRALERT_FP_SET\|CRALERT_READ_ALL`, caller-supplied `fp` at offset 0 ⇒ parses from the caller's stream | [x] |
| 35 | `Read_FileMon` | `flags=0`, `timeout=0`, populated `alerts.log` ⇒ `fp` at EOF, both `GetAlertData`s fail, loop body never runs ⇒ NULL, no sleep | [x] |
| 36 | `Read_FileMon` | `flags=CRALERT_READ_ALL`, `timeout=1`, file has **no** parseable alert ⇒ one 5 s retry then NULL | [x] |
| 37 | `Read_FileMon` | randomized `tm` (`tm_mday`/`tm_year`/`tm_mon` 0..11) on the second-pass `GetFile_Queue` path ⇒ `day`/`year`/`mon` updated | [x] |
| 38 | `driver` | `flags=CRALERT_READ_ALL`, `timeout=0`, randomized `day`/`month` 0..11/`year`, populated `alerts.log` — full end-to-end | [x] |
| 39 | `driver` | `flags=CRALERT_READ_ALL\|CRALERT_MAIL_SET`, `timeout=0`, mixed alerts | [x] |
| 40 | `driver` | `flags=0`, `timeout=0` ⇒ EOF-seek path ⇒ NULL | [x] |
| 41 | `driver` | `flags=CRALERT_FP_SET\|CRALERT_READ_ALL`, `timeout=0` ⇒ `file_name` `<stdin>`; with a real file named `<stdin>` in the CWD it is opened by `Read_FileMon`'s `Handle_Queue(0)` | [x] |
| 42 | `driver` | randomized full-`int` `flags` (all 32 bits) with `timeout=0` and a fixed `alerts.log` — cross-product of the 3 live bits + noise bits | [x] |
| 43 | `driver` | `flags=CRALERT_READ_ALL`, randomized whole-file fuzz (random lines drawn from the recognized prefixes + junk, 200 files) | [x] |
| 44 | `GetAlertData` | randomized whole-file fuzz at the lowest level: 200 randomly generated files × random `flag`, all sequential calls until NULL | [x] |
| 45 | `GetAlertData` | prefix-boundary sweep: every one of the 8 recognized `strncmp` prefixes truncated/extended one byte at a time (probing N-1 / N / N+1 available bytes) × 7 suffixes × 3 positions in the alert (header line / body line / date line) — 1176 shapes | [x] |
| 46 | `GetAlertData` | mutation fuzz: 1500 mutants of a well-formed 3-alert file (byte flip / delete / insert / line duplicate / line drop / truncate / pad past `OS_MAXSTR`), 1-6 mutations each | [x] |
| 47 | `GetAlertData` | 800 pure byte-noise files (0-1400 bytes, embedded NULs, biased towards `\n : ' - *`), random `flag` | [x] |
| 48 | `GetAlertData` | 1200 files assembled from a wide line vocabulary: randomized whitespace, quote counts, and numeric shapes (`0`, `-0`, `2147483648`, `0x10`, `010`, `+5`, `9999999999999999999`, ` 7 `) feeding `atoi` for `rule`/`level`/`srcport`/`dstport` | [x] |
| 49 | `GetAlertData` | the one input where the C reads past the NUL `fgets` wrote: a header line of exactly `"** Alert"` (8 bytes) at EOF with no newline, so `p = str + 9` is past the terminator — 200 variants with differing preceding lines so the reused buffer contents vary | [x] |

Rows 45-49 live in `tests/phase_b_fuzz.rs`; rows 1-5 in `tests/phase_b_low.rs`,
6-23 in `tests/phase_b_parser.rs`, 24-44 in `tests/phase_b_queue.rs`.

## Not applicable

* **Feature combinations** — `translation/Cargo.toml` declares no `[features]`
  table, so the only build configuration is the default one. Verified with
  `cargo metadata --no-deps` (`features = {}`). The whole suite was nevertheless
  run under both `cargo test` and `cargo test --no-default-features`, with
  identical results (the sole build configuration).
* **Binary executable** — `CMakeLists.txt` builds only `add_library(driver SHARED …)`
  and `Cargo.toml` declares only `crate-type = ["cdylib"]`. There is no driver
  binary, so there is no stdout to compare. (`driver()` is a library entry point,
  covered by rows 38-43.)
