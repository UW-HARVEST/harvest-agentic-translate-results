# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from:

```
nm -D --defined-only c_src/build/libdriver.so
nm -D --defined-only translation/target/release/libdriver.so
```

## C source inventory (every non-`static` definition)

| C file | symbol | linkage |
|--------|--------|---------|
| `c_src/include/shared.h` | `os_calloc` | external (defined **in the header**, so it lands in every TU that includes it; only `read-alert.c` includes `shared.h`) |
| `c_src/include/shared.h` | `os_realloc` | external |
| `c_src/include/shared.h` | `os_strdup` | external |
| `c_src/src/file-queue.c` | `merror` | external |
| `c_src/src/file-queue.c` | `file_sleep` | `static` → NOT exported |
| `c_src/src/file-queue.c` | `GetFile_Queue` | `static` → NOT exported |
| `c_src/src/file-queue.c` | `Handle_Queue` | `static` → NOT exported |
| `c_src/src/file-queue.c` | `s_month` | `static const` → NOT exported |
| `c_src/src/file-queue.c` | `Init_FileQueue` | external |
| `c_src/src/file-queue.c` | `Read_FileMon` | external |
| `c_src/src/read-alert.c` | `FreeAlertData` | external |
| `c_src/src/read-alert.c` | `GetAlertData` | external |
| `c_src/src/driver.c` | `driver` | external |

## Dynamic-symbol table comparison

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `FreeAlertData` | `T` | `T` | OK |
| 2 | `GetAlertData`  | `T` | `T` | OK |
| 3 | `Init_FileQueue`| `T` | `T` | OK |
| 4 | `Read_FileMon`  | `T` | `T` | OK |
| 5 | `driver`        | `T` | `T` | OK |
| 6 | `merror`        | `T` | `T` | OK |
| 7 | `os_calloc`     | `T` | `T` | OK |
| 8 | `os_realloc`    | `T` | `T` | OK |
| 9 | `os_strdup`     | `T` | `T` | OK |

**Symbol diff (C-exported minus Rust-exported): EMPTY.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $3}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $3}' | sort)
(no output)
```

No symbol needed a new `#[no_mangle]` wrapper and no C module was left
untranslated: all three C translation units (`file-queue.c`, `read-alert.c`,
`driver.c`) plus the header-defined `shared.h` helpers have Rust counterparts
(`src/file_queue.rs`, `src/read_alert.rs`, `src/driver.rs`, `src/shared.rs`).

## Undefined (imported) symbols

The Rust `.so` must not reference any non-libc symbol that the C `.so` does not.
Both import only glibc entities. The Rust `.so` additionally imports the small
set of glibc symbols the Rust runtime shim needs (`memcpy`, `pthread_*`, unwind
stubs). That is a superset of libc only — no project symbol is undefined.

Verified with:

```
nm -D --undefined-only translation/target/release/libdriver.so
```

→ 0 undefined non-libc symbols.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. Phase D's "repeat for every feature
combination" therefore collapses to a single combination; this is confirmed
mechanically by `scripts/check_features.sh`, which parses `Cargo.toml` and
loops over the powerset it finds (empty ⇒ default build only).
