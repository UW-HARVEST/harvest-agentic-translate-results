# Exported Symbol Surface

Derived with:

```text
nm -D --defined-only c_src/build/libdriver_c.so
```

The CMake project declares only an executable. The shared object was therefore
linked from the unchanged `c_src/src/main.c` with the same default
configuration and `-fPIC -shared`.

| # | C symbol | Rust status |
|---|----------|-------------|
| 1 | `parse_command` | [x] exported via C-ABI wrapper |
| 2 | `cmd_adduser` | [x] exported via C-ABI wrapper |
| 3 | `cmd_login` | [x] exported via C-ABI wrapper |
| 4 | `cmd_logout` | [x] exported via C-ABI wrapper |
| 5 | `cmd_whoami` | [x] exported via C-ABI wrapper |
| 6 | `cmd_listusers` | [x] exported via C-ABI wrapper |
| 7 | `cmd_createfile` | [x] exported via C-ABI wrapper |
| 8 | `cmd_readfile` | [x] exported via C-ABI wrapper |
| 9 | `cmd_writefile` | [x] exported via C-ABI wrapper |
| 10 | `cmd_deletefile` | [x] exported via C-ABI wrapper |
| 11 | `cmd_listfiles` | [x] exported via C-ABI wrapper |
| 12 | `cmd_set` | [x] exported via C-ABI wrapper |
| 13 | `cmd_get` | [x] exported via C-ABI wrapper |
| 14 | `cmd_unset` | [x] exported via C-ABI wrapper |
| 15 | `cmd_listvars` | [x] exported via C-ABI wrapper |
| 16 | `cmd_compare` | [x] exported via C-ABI wrapper |
| 17 | `cmd_compareN` | [x] exported via C-ABI wrapper |
| 18 | `cmd_startswith` | [x] exported via C-ABI wrapper |
| 19 | `cmd_match` | [x] exported via C-ABI wrapper |
| 20 | `cmd_help` | [x] exported via C-ABI wrapper |
| 21 | `cmd_debug` | [x] exported via C-ABI wrapper |
| 22 | `cmd_verbose` | [x] exported via C-ABI wrapper |
| 23 | `cmd_status` | [x] exported via C-ABI wrapper |
| 24 | `cmd_time` | [x] exported via C-ABI wrapper |
| 25 | `process_command` | [x] exported via C-ABI wrapper |
| 26 | `main` | [x] exported via C-ABI wrapper |

No C implementation is absent from the Rust translation. All missing symbols
require C-ABI export wrappers around the translated implementations.
