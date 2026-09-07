# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C   : `c_src/build/libdriver.so`
* Rust: `translation/target/release/libdriver.so`

## Dynamic symbol table (defined, `T`)

| # | symbol | source (C) | in C `.so` | in Rust `.so` | Rust impl site |
|---|--------|------------|-----------|---------------|----------------|
| 1 | `merror`         | `src/file-queue.c:24`  | T | T | `src/file_queue.rs::merror` |
| 2 | `Init_FileQueue` | `src/file-queue.c:113` | T | T | `src/file_queue.rs::Init_FileQueue` |
| 3 | `Read_FileMon`   | `src/file-queue.c:143` | T | T | `src/file_queue.rs::Read_FileMon` |
| 4 | `os_calloc`      | `include/shared.h:13`  | T | T | `src/shared.rs::os_calloc` |
| 5 | `os_realloc`     | `include/shared.h:22`  | T | T | `src/shared.rs::os_realloc` |
| 6 | `os_strdup`      | `include/shared.h:31`  | T | T | `src/shared.rs::os_strdup` |
| 7 | `FreeAlertData`  | `src/read-alert.c:64`  | T | T | `src/read_alert.rs::FreeAlertData` |
| 8 | `GetAlertData`   | `src/read-alert.c:93`  | T | T | `src/read_alert.rs::GetAlertData` |
| 9 | `driver`         | `src/driver.c:6`       | T | T | `src/driver.rs::driver` |

**Missing from Rust: 0. Extra in Rust: 0.**

`static` C functions (`file_sleep`, `GetFile_Queue`, `Handle_Queue`) have
internal linkage and are correctly *not* exported by either object; they are
private `unsafe fn`s in `src/file_queue.rs`.

## Undefined symbols

Both objects import only libc / runtime symbols. The Rust object additionally
imports the Rust `std` runtime's libc/unwind set
(`_Unwind_*`, `malloc`, `memcpy`, `pthread_key_*`, `open64`, `mmap64`, …) plus
`strtol` (used to model glibc's inline `atoi`) and the `*64`/`statx` variants of
the `stat` family. **0 undefined non-libc/non-runtime symbols.**

Verify with:

```sh
diff <(nm -D --defined-only c_src/build/libdriver.so         | awk '{print $3}' | sort) \
     <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
```

## ABI: struct layout parity (x86-64 glibc)

Confirmed against a C `offsetof` probe.

| struct | size | field offsets |
|--------|------|---------------|
| `file_queue` | 440 | last_change 0, year 8, day 12, flags 16, mon 20, file_name 24, fp 288, f_status 296 |
| `alert_data` | 96  | rule 0, level 4, alertid 8, date 16, location 24, comment 32, group 40, srcip 48, srcport 56, dstip 64, dstport 72, user 80, filename 88 |
| `struct stat` | 144 | `st_mtim` at 88 |
| `struct tm`   | 56  | — |

## ABI notes / cross-object interop

`alert_data` and `file_queue` are plain `#[repr(C)]` PODs allocated with
`calloc`/`strdup`, so a struct produced by the C `.so` can be freed by the Rust
`.so` and vice versa. Tests exploit this to compare and then release results
through either library.
