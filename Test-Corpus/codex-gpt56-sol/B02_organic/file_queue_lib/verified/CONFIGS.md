# Configuration Surface

## Build-Time Configurations

Neither `Cargo.toml` nor `c_src/CMakeLists.txt` defines a feature/option axis.
There is exactly one valid combination:

| # | Cargo invocation | CMake configuration | status |
|---|------------------|---------------------|--------|
| B01 | `--no-default-features --features ''` | default (all three C sources) | [x] |

## Runtime and Input Configurations

The meaningful flag axes are `MAIL_SET` (parser filter), `READ_ALL` (initial
seek), and `FP_SET` (caller-owned stream versus `alerts.log`). `EXEC_SET`,
`READ_FAILED`, and unknown bits are accepted but unused. Rows below are the
pruned cross-product of branches actually distinguished by the C source.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| C01 | `os_calloc` | one element and many elements; successful allocations are fully zeroed | [x] |
| C02 | `os_realloc` | `ptr == NULL`, then grow and shrink an existing allocation while preserving the common prefix | [x] |
| C03 | `os_strdup` | empty and non-empty NUL-terminated byte strings | [x] |
| C04 | `merror` | format template with file name, signed error number, and error text; emitted bytes include one newline | [x] |
| C05 | `FreeAlertData` | zeroed `alert_data` and a structure with every owned string field populated | [x] |
| C06 | `GetAlertData` | empty stream and non-alert prelude lines | [x] |
| C07 | `GetAlertData` | `MAIL_SET` clear; header mode begins with both `mail` and non-`mail` text | [x] |
| C08 | `GetAlertData` | `MAIL_SET` set; matching `mail` header followed by a complete alert | [x] |
| C09 | `GetAlertData` | flags contain only unused `EXEC_SET`, `READ_FAILED`, and unknown bits | [x] |
| C10 | `GetAlertData` | header has no group delimiter `-`, has an empty group, or has multiple leading spaces after `-` | [x] |
| C11 | `GetAlertData` | ordinary group versus a group containing `syscheck` | [x] |
| C12 | `GetAlertData` | valid date/location line split at the first space following a colon | [x] |
| C13 | `GetAlertData` | valid `Rule: ` line with randomized signed/unsigned-looking decimal text and quoted comments | [x] |
| C14 | `GetAlertData` | each recognized optional field absent/present: source IP/port, destination IP/port, and user | [x] |
| C15 | `GetAlertData` | recognized string fields and rule line repeated; the last parsed value wins | [x] |
| C16 | `GetAlertData` | ordinary unrecognized log lines in state 2 | [x] |
| C17 | `GetAlertData` | syscheck group followed immediately by matching integrity-change line; filename strips its final byte | [x] |
| C18 | `GetAlertData` | syscheck group followed by a nonmatching line, then a matching line; one-shot syscheck state yields no filename | [x] |
| C19 | `GetAlertData` | complete alert ending at EOF | [x] |
| C20 | `GetAlertData` | two or more alerts; first call rewinds at next header and subsequent calls return later alerts | [x] |
| C21 | `GetAlertData` | lines around the `fgets` limit: 1022, 1023, and longer-than-1023 bytes | [x] |
| C22 | `Init_FileQueue` | `FP_SET` clear, `alerts.log` exists, `READ_ALL` clear: opens fixed path and seeks to EOF | [x] |
| C23 | `Init_FileQueue` | `FP_SET` clear, `alerts.log` exists, `READ_ALL` set: opens fixed path and retains position at start | [x] |
| C24 | `Init_FileQueue` | caller regular-file `fp`, `FP_SET` set, `READ_ALL` clear: retains stream and seeks to EOF | [x] |
| C25 | `Init_FileQueue` | caller regular-file `fp`, `FP_SET | READ_ALL` set: retains stream at start | [x] |
| C26 | `Init_FileQueue` | month indices `0..11`, arbitrary day/year, and ignored flag bits; fields and `Jan`...`Dec` bytes match | [x] |
| C27 | `Read_FileMon` | initialized `READ_ALL` queue with a complete first alert; returns it immediately | [x] |
| C28 | `Read_FileMon` | existing stream at EOF, successful reopen, and `timeout == 0` | [x] |
| C29 | `Read_FileMon` | first parser call returns an alert before timeout; `timeout == 0`, `1`, and larger values are observationally identical | [x] |
| C30 | `driver` | fixed `alerts.log`, `READ_ALL` set, complete alert at start; end-to-end parsed result | [x] |
| C31 | `driver` | fixed `alerts.log`, `READ_ALL` clear; initial seek-to-end means existing content is not returned | [x] |
| C32 | `driver` | `MAIL_SET | READ_ALL`; matching versus nonmatching header mode | [x] |
| C33 | `driver` | unused/unknown flag bits combined with `READ_ALL`; parsed result is unchanged | [x] |

Public entry points represented: `os_calloc`, `os_realloc`, `os_strdup`,
`merror`, `FreeAlertData`, `GetAlertData`, `Init_FileQueue`, `Read_FileMon`,
and `driver`.
