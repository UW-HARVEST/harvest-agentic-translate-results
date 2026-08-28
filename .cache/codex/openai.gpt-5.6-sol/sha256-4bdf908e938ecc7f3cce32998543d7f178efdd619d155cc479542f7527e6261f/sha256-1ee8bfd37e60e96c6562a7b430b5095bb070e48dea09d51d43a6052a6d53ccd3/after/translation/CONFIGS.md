# Configuration Surface

Derived from the public headers, the additional exported `driver` entry point,
and every option/state/input-shape branch in `c_src/src/*.c`. `LOG_FILE` has
unset, valid custom, and invalid states; the invalid state is in `ERRORS.md`.
`MAX_TASKS` has unset/default (`10`) and present/`atoi` states. The rows below
are the pruned cross-product of states that the C code treats differently.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `initialize_logger` | `LOG_FILE` unset: open and append to `default.log` | [x] |
| 2 | `initialize_logger` | `LOG_FILE` set to a valid custom path: open and append there | [x] |
| 3 | `log_info` | logger uninitialized; message is ignored | [x] |
| 4 | `log_info` | logger initialized; empty and nonempty messages emit `[INFO]` records | [x] |
| 5 | `log_warning` | logger uninitialized; message is ignored | [x] |
| 6 | `log_warning` | logger initialized; empty and nonempty messages emit `[WARNING]` records | [x] |
| 7 | `log_error` | logger uninitialized; message is ignored | [x] |
| 8 | `log_error` | logger initialized; empty and nonempty messages emit `[ERROR]` records | [x] |
| 9 | `finalize_logger` | logger uninitialized; no-op | [x] |
| 10 | `finalize_logger` | logger initialized; append final record and close the stream | [x] |
| 11 | `create_task_manager` | `MAX_TASKS` unset: `max_tasks == 10`, count zero, allocated task array | [x] |
| 12 | `create_task_manager` | `MAX_TASKS` set to positive values parsed by `atoi` | [x] |
| 13 | `create_task_manager` | `MAX_TASKS` set to zero or a string parsed by `atoi` as zero; zero-sized task allocation succeeds | [x] |
| 14 | `add_task` | count below capacity; empty description, arbitrary `int` priority | [x] |
| 15 | `add_task` | count below capacity; description lengths `1..254`, arbitrary bytes excluding NUL, boundary priorities | [x] |
| 16 | `add_task` | count below capacity; description length exactly 255 | [x] |
| 17 | `add_task` | count below capacity; description length 256 or greater is truncated to 255 bytes and NUL-terminated | [x] |
| 18 | `add_task` | count exactly at capacity; task is rejected without mutation | [x] |
| 19 | `add_task` | count greater than capacity; task is rejected without mutation | [x] |
| 20 | `print_tasks` | manager contains zero tasks | [x] |
| 21 | `print_tasks` | manager contains one task, including empty/short/boundary-length descriptions and boundary priority values | [x] |
| 22 | `print_tasks` | manager contains many tasks; preserves insertion order, one-based display indexes, descriptions, and priorities | [x] |
| 23 | `destroy_task_manager` | empty manager while logger is uninitialized | [x] |
| 24 | `destroy_task_manager` | populated manager while logger is initialized; frees storage and emits destruction record | [x] |
| 25 | `driver` | empty input; default `MAX_TASKS`; prints an empty task list | [x] |
| 26 | `driver` | one nonempty line without trailing newline; capacity available | [x] |
| 27 | `driver` | one line with trailing newline; trailing empty segment is not added | [x] |
| 28 | `driver` | multiple lines; priorities increment from one and output preserves order | [x] |
| 29 | `driver` | consecutive newlines; interior empty segments become tasks | [x] |
| 30 | `driver` | line length 255 and line length greater than 255; stored output follows `add_task` truncation | [x] |
| 31 | `driver` | `MAX_TASKS=0`; all nonempty-loop iterations are rejected and output stays empty | [x] |
| 32 | `driver` | positive `MAX_TASKS` smaller than line count; excess lines rejected while later loop priorities still advance | [x] |
| 33 | `driver` | positive `MAX_TASKS` equal to line count | [x] |
| 34 | `driver` | positive `MAX_TASKS` greater than line count | [x] |
| 35 | `driver` | valid custom `LOG_FILE`; end-to-end records append to the selected file | [x] |
