# Dynamic Symbol Surface

Generated from:

```text
nm -D --defined-only c_src/build/libdriver.so
```

The C shared object exports ten public symbols. The default Rust shared object
currently exports all ten with exact names.

| # | symbol | C source | Rust export |
|---|--------|----------|-------------|
| 1 | `add_task` | `src/task_manager.c` | present |
| 2 | `create_task_manager` | `src/task_manager.c` | present |
| 3 | `destroy_task_manager` | `src/task_manager.c` | present |
| 4 | `driver` | `src/driver.c` | present |
| 5 | `finalize_logger` | `src/logger.c` | present |
| 6 | `initialize_logger` | `src/logger.c` | present |
| 7 | `log_error` | `src/logger.c` | present |
| 8 | `log_info` | `src/logger.c` | present |
| 9 | `log_warning` | `src/logger.c` | present |
| 10 | `print_tasks` | `src/task_manager.c` | present |

Undefined entries reported by `nm -D` are runtime imports from glibc or ELF
toolchain weak symbols, not library exports: `atoi`, `fclose`, `fopen`,
`fprintf`, `free`, `fwrite`, `getenv`, `malloc`, `printf`, `puts`, `stderr`,
`strchr`, `strlen`, `strncpy`, `_ITM_deregisterTMCloneTable`,
`_ITM_registerTMCloneTable`, `__cxa_finalize`, and `__gmon_start__`.

Missing C exports in Rust: **0**
