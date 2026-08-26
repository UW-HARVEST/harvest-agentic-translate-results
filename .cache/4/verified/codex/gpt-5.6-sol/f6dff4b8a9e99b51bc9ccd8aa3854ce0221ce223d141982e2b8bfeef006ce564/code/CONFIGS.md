# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no options
or conditional compilation. There is exactly one valid combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (empty feature set) | default `driver` shared target | [x] |

## Runtime and Input Configurations

The rows below come from branches in the public implementations and the
fixed-width fields in the public headers. Randomized domains within a row cover
values that take the same control-flow path.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `initialize_logger` | `LOG_FILE` unset; create/append `default.log` | [x] |
| 2 | `initialize_logger` | `LOG_FILE` set to an existing empty file | [x] |
| 3 | `initialize_logger` | `LOG_FILE` set to an existing nonempty file; append without truncation | [x] |
| 4 | `log_info` | logger not initialized; null, empty, and randomized nonempty messages are ignored | [x] |
| 5 | `log_warning` | logger not initialized; null, empty, and randomized nonempty messages are ignored | [x] |
| 6 | `log_error` | logger not initialized; null, empty, and randomized nonempty messages are ignored | [x] |
| 7 | `log_info` | logger initialized; empty and randomized nonempty/long messages | [x] |
| 8 | `log_warning` | logger initialized; empty and randomized nonempty/long messages | [x] |
| 9 | `log_error` | logger initialized; empty and randomized nonempty/long messages | [x] |
| 10 | `finalize_logger` | logger not initialized | [x] |
| 11 | `finalize_logger` | logger initialized; write final record and close stream | [x] |
| 12 | `create_task_manager` | `MAX_TASKS` unset; default capacity `10`, count `0` | [x] |
| 13 | `create_task_manager` | `MAX_TASKS` set to randomized positive decimal capacities | [x] |
| 14 | `create_task_manager` | `MAX_TASKS` set to zero-equivalent input (`0`, empty, or nonnumeric via `atoi`) | [x] |
| 15 | `add_task` | below capacity; description lengths `0..254`, randomized priorities including negative/zero/positive | [x] |
| 16 | `add_task` | below capacity; description length exactly `255` | [x] |
| 17 | `add_task` | below capacity; description length greater than `255`, truncated and NUL-terminated at byte 255 | [x] |
| 18 | `add_task` | at/above capacity, including zero capacity; no state change | [x] |
| 19 | `print_tasks` | valid manager with zero tasks | [x] |
| 20 | `print_tasks` | valid manager with one task | [x] |
| 21 | `print_tasks` | valid manager with multiple tasks and randomized descriptions/priorities | [x] |
| 22 | `destroy_task_manager` | valid empty manager | [x] |
| 23 | `destroy_task_manager` | valid manager containing one or many tasks | [x] |
| 24 | `driver` | successful logger; empty input string (zero tasks) | [x] |
| 25 | `driver` | successful logger; one nonempty line with no newline | [x] |
| 26 | `driver` | successful logger; multiple nonempty newline-separated lines | [x] |
| 27 | `driver` | successful logger; leading/consecutive/trailing newlines (empty interior tasks; no task after trailing newline) | [x] |
| 28 | `driver` | successful logger; task length at and above 255 bytes | [x] |
| 29 | `driver` | successful logger; input has more lines than positive `MAX_TASKS` capacity | [x] |
| 30 | `driver` | successful logger; `MAX_TASKS` is zero-equivalent, so every parsed line is rejected by `add_task` | [x] |
| 31 | `driver` | custom `LOG_FILE` and randomized valid `MAX_TASKS`, input shape drawn from rows 24-29 | [x] |
