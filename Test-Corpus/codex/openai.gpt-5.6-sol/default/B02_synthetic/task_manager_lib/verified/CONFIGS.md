# Configuration surface

Mechanically derived axes:

- Logger state: closed/open; `LOG_FILE` absent/present; info/warning/error.
- Manager capacity: `MAX_TASKS` absent, positive, zero, or nonnumeric zero.
- Manager occupancy: empty, below capacity, exactly full, over-capacity call.
- Description shape: empty, short, 255 bytes, and more than 255 bytes.
- Driver input shape: empty, one line, leading/consecutive/trailing newlines,
  many lines below/at/above capacity, and long lines.
- Priority/data values: ordinary and full signed-`int` boundary values.

There are no Cargo features and no C `#ifdef` behavior branches.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `initialize_logger` | `LOG_FILE` absent; writable default path | [x] |
| 2 | `initialize_logger` | `LOG_FILE` present; randomized writable path | [x] |
| 3 | `log_info`, `log_warning`, `log_error` | logger closed; randomized nonempty messages | [x] |
| 4 | `log_info`, `log_warning`, `log_error` | logger open; empty, short, and randomized messages | [x] |
| 5 | `finalize_logger` | logger closed | [x] |
| 6 | `finalize_logger` | logger open after randomized log calls | [x] |
| 7 | `create_task_manager` | `MAX_TASKS` absent, yielding capacity 10 and count 0 | [x] |
| 8 | `create_task_manager` | positive `MAX_TASKS`: 1 and randomized values greater than 1 | [x] |
| 9 | `create_task_manager` | `MAX_TASKS=0` and nonnumeric values parsed by `atoi` as zero | [x] |
| 10 | `add_task` | below capacity; empty/short/random descriptions of 0–254 bytes | [x] |
| 11 | `add_task` | below capacity; description exactly 255 bytes | [x] |
| 12 | `add_task` | below capacity; descriptions of 256 bytes and randomized larger sizes, truncated to 255 bytes | [x] |
| 13 | `add_task` | below capacity; priorities include `INT_MIN`, negative, zero, positive, and `INT_MAX` | [x] |
| 14 | `add_task` | manager exactly at capacity; extra randomized task rejected without state change | [x] |
| 15 | `print_tasks` | empty manager | [x] |
| 16 | `print_tasks` | one task, including description and priority boundaries | [x] |
| 17 | `print_tasks` | many randomized tasks in insertion order | [x] |
| 18 | `destroy_task_manager` | valid manager with zero or positive capacity, empty or populated | [x] |
| 19 | `driver` | empty task string | [x] |
| 20 | `driver` | one randomized task without newline | [x] |
| 21 | `driver` | leading newline, creating an empty first task | [x] |
| 22 | `driver` | consecutive newlines, creating empty interior tasks | [x] |
| 23 | `driver` | trailing newline, with no extra final empty task | [x] |
| 24 | `driver` | randomized line count below configured capacity | [x] |
| 25 | `driver` | line count exactly equal to configured capacity | [x] |
| 26 | `driver` | line count above capacity; later additions rejected while priorities keep incrementing | [x] |
| 27 | `driver` | line length exactly 255 and randomized lengths greater than 255, exercising truncation | [x] |
| 28 | `driver` | `MAX_TASKS=0` and nonnumeric zero; every parsed task is rejected | [x] |
| 29 | `driver`, logger API | both default and explicit `LOG_FILE` selection | [x] |
