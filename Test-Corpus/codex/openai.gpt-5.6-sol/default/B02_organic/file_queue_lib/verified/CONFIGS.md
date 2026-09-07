# Configuration-surface table

Mechanically derived from public headers and the `if` branches in all three C
translation units. Cargo declares no features, so the build matrix is the
default build and the equivalent `--no-default-features` build.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `os_calloc` | nonzero count × nonzero size; returned bytes are zero-filled | [x] |
| 2 | `os_calloc` | zero count or zero element size where libc returns a valid freeable pointer | [x] |
| 3 | `os_realloc` | `NULL` input pointer with nonzero size (allocation form) | [x] |
| 4 | `os_realloc` | existing allocation grown, preserving the old byte prefix | [x] |
| 5 | `os_realloc` | existing allocation shrunk, preserving the retained byte prefix | [x] |
| 6 | `os_strdup` | empty and nonempty NUL-terminated strings | [x] |
| 7 | `FreeAlertData` | every nullable string field absent | [x] |
| 8 | `FreeAlertData` | every string field allocated and present | [x] |
| 9 | `merror` | format template consuming file name, integer errno, and error string; output longer and shorter than the 256-byte local buffer | [x] |
| 10 | `GetAlertData` | flags with `CRALERT_MAIL_SET` clear; pre-alert lines and malformed headers (no `:`, no space) are skipped before a complete alert | [x] |
| 11 | `GetAlertData` | `CRALERT_MAIL_SET` set; non-`mail` headers are skipped and a later `mail` alert is parsed | [x] |
| 12 | `GetAlertData` | header has no `-` group separator versus a group with leading spaces; group is non-syscheck | [x] |
| 13 | `GetAlertData` | syscheck group plus matching first ordinary log line extracts and dequotes `filename` | [x] |
| 14 | `GetAlertData` | syscheck group plus nonmatching first ordinary log line clears syscheck state and leaves `filename` null | [x] |
| 15 | `GetAlertData` | EOF ends a complete state-2 alert with no `Rule`, address, port, user, or ordinary-log fields | [x] |
| 16 | `GetAlertData` | valid `Rule` line across randomized signed/unsigned-looking decimal text and comments, exercising `atoi` then unsigned storage | [x] |
| 17 | `GetAlertData` | each optional `Src IP`, `Src Port`, `Dst IP`, `Dst Port`, and `User` line, including empty values and `atoi` edge-shaped text | [x] |
| 18 | `GetAlertData` | repeated group/rule/src/dst/user fields replace prior allocated values; later numeric ports overwrite earlier values | [x] |
| 19 | `GetAlertData` | ordinary log lines at the `LOG_LIMIT` branch shape (zero, one, and more than 100 lines) | [x] |
| 20 | `GetAlertData` | lines below, at, and above the `fgets(..., OS_MAXSTR, ...)` chunk boundary | [x] |
| 21 | `GetAlertData` | a second alert header terminates the first and is rewound so the next call parses the second | [x] |
| 22 | `GetAlertData` | all documented flag bits and unknown flag bits set; only `CRALERT_MAIL_SET` changes parser behavior | [x] |
| 23 | `Init_FileQueue` | month `0..=11`, varied day/year, no `CRALERT_FP_SET`; absent versus present `alerts.log`, with `CRALERT_READ_ALL` clear/set | [x] |
| 24 | `Init_FileQueue` | `CRALERT_FP_SET` with a supplied seekable stream; `CRALERT_READ_ALL` clear seeks to EOF, set preserves the current position | [x] |
| 25 | `Init_FileQueue` | unused/unknown flag bits combined with the queue flags are preserved in `fileq->flags` and do not alter other branches | [x] |
| 26 | `Read_FileMon` | initialized seekable `CRALERT_FP_SET \| CRALERT_READ_ALL` queue containing a complete alert; first parse succeeds | [x] |
| 27 | `Read_FileMon` | first parse on an empty supplied stream returns `NULL`; `alerts.log` is reopened at EOF, a producer appends during the retry sleep, and a later retry succeeds | [x] |
| 28 | `Read_FileMon` | parser flag interaction: `CRALERT_MAIL_SET` skips non-mail and returns a later mail alert | [x] |
| 29 | `Read_FileMon` | timeout boundary `0` and `UINT_MAX` when a complete alert is immediately available (no retry sleep) | [x] |
| 30 | `driver` | `alerts.log` complete alert; `CRALERT_READ_ALL` set so parsing starts at byte zero | [x] |
| 31 | `driver` | `CRALERT_MAIL_SET \| CRALERT_READ_ALL` with non-mail then mail alerts | [x] |
| 32 | `driver` | all month indices `0..=11`, varied day/year/timeout, and unused/unknown flag bits that do not alter the selected path | [x] |
