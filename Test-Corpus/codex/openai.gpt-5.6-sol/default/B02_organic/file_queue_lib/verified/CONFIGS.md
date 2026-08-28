# Configuration Surface

Derived from all public dynamic symbols, public flags, parser-state branches,
record prefixes, queue branches, and size constants in the C source. The
cross-product is pruned where C does not distinguish an axis. Randomized cases
within each row vary field values, lengths, and month/day/year values.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|---|
| 1 | `os_calloc` | ordinary nonzero element count and size; returned bytes are zero | [x] |
| 2 | `os_calloc` | zero count or zero element size (`0 × n`, `n × 0`, `0 × 0`) | [x] |
| 3 | `os_realloc` | `ptr == NULL`, nonzero size (allocation behavior) | [x] |
| 4 | `os_realloc` | allocated pointer resized smaller, same size, and larger with prefix preserved | [x] |
| 5 | `os_strdup` | empty and nonempty NUL-terminated strings | [x] |
| 6 | `merror` | substitutions that fit the 256-byte buffer and substitutions truncated at 255 bytes | [x] |
| 7 | `FreeAlertData` | zero-initialized structure with every owned field `NULL` | [x] |
| 8 | `FreeAlertData` | every owned string field populated, including empty and nonempty strings | [x] |
| 9 | `GetAlertData` | empty stream or arbitrary preamble with no `** Alert` marker | [x] |
| 10 | `GetAlertData` | marker candidates missing `:` or the post-ID space are skipped | [x] |
| 11 | `GetAlertData` | minimal accepted marker + date/location, EOF terminates one alert | [x] |
| 12 | `GetAlertData` | two complete alerts; second marker terminates first and remains for next call | [x] |
| 13 | `GetAlertData` | `CRALERT_MAIL_SET` clear; both `mail` and non-`mail` marker modes accepted | [x] |
| 14 | `GetAlertData` | `CRALERT_MAIL_SET` set; `mail` marker accepted and non-`mail` marker skipped | [x] |
| 15 | `GetAlertData` | marker has no `-` group delimiter, ordinary group, or leading spaces after `-` | [x] |
| 16 | `GetAlertData` | group contains `syscheck`; matching integrity line extracts filename and strips final byte | [x] |
| 17 | `GetAlertData` | group contains `syscheck`; first otherwise-unrecognized line does not match integrity prefix | [x] |
| 18 | `GetAlertData` | valid `Rule: ` line with zero/positive/negative numeric text and quoted comment | [x] |
| 19 | `GetAlertData` | each metadata prefix: source IP/port, destination IP/port, and user | [x] |
| 20 | `GetAlertData` | repeated group/string metadata fields replace the previous allocation | [x] |
| 21 | `GetAlertData` | ignored log lines and input lines below, at, and above the 1024-byte buffer boundary | [x] |
| 22 | `GetAlertData` | ignored flags `CRALERT_EXEC_SET`, `CRALERT_READ_FAILED`, unknown high bits, and `-1` | [x] |
| 23 | `Init_FileQueue` | regular mode, `alerts.log` absent; initializes date/name and succeeds with `fp == NULL` | [x] |
| 24 | `Init_FileQueue` | regular mode, file present, `CRALERT_READ_ALL` clear/set; starts at EOF/beginning | [x] |
| 25 | `Init_FileQueue` | `CRALERT_FP_SET`, supplied seekable stream, `CRALERT_READ_ALL` clear/set; retains stream and uses `<stdin>` name | [x] |
| 26 | `Init_FileQueue` | ignored flag bits and unknown bits preserve exact `flags` without changing queue behavior | [x] |
| 27 | `Read_FileMon` | supplied initialized stream already contains a complete alert; direct first-read success | [x] |
| 28 | `Read_FileMon` | supplied stream has no alert, refreshes through `alerts.log`, `timeout == 0` | [x] |
| 29 | `Read_FileMon` | `fp == NULL`, regular `alerts.log` is reopened; seek-to-end behavior yields no event | [x] |
| 30 | `driver` | `CRALERT_READ_ALL` set with complete `alerts.log`; end-to-end parsed alert | [x] |
| 31 | `driver` | `CRALERT_READ_ALL | CRALERT_MAIL_SET`; matching versus filtered marker | [x] |
| 32 | `driver` | `CRALERT_READ_ALL` clear; initial seek to EOF and zero-timeout result | [x] |
| 33 | `driver` | `CRALERT_FP_SET` with no supplied stream (driver's zeroed queue), with `READ_ALL` clear/set | [x] |
| 34 | `driver` | ignored and unknown flag bits combined with `CRALERT_READ_ALL` | [x] |

Cargo feature combinations: **default/no features only** (`Cargo.toml` has no
`[features]` table).
