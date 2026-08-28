# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-61Wh7J.so` (name comes from the parent
  directory name via `cmake_path(GET parent FILENAME project_name)`).
* Rust `.so`: `translation/target/release/libmaxnmin_lib.so`

Regenerate with:

```sh
nm -D --defined-only c_src/build/*.so                     | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/*.so       | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms   # must be EMPTY
```

(`translation/check_symbols.sh` does exactly this and exits non-zero on any diff.)

## Defined (exported) symbols

The C translation unit `c_src/src/lib.c` has 7 external functions; everything
else (`node_storage`, `node_count`) is `static` and therefore not exported.
There are no macro-generated symbols in this library.

| # | C symbol | signature (from `c_src/src/lib.c`) | in C `.so` | in Rust `.so` | status |
|---|----------|------------------------------------|-----------|--------------|--------|
| 1 | `add_node`              | `int add_node(int id, int parent_id, const char *name, double value)` | T | T | OK |
| 2 | `find_node_by_id`       | `Node *find_node_by_id(int id)`                                        | T | T | OK |
| 3 | `get_children_count`    | `int get_children_count(int parent_id)`                                | T | T | OK |
| 4 | `calculate_subtree_sum` | `double calculate_subtree_sum(int node_id)`                            | T | T | OK |
| 5 | `process_string`        | `int process_string(char *str)`                                         | T | T | OK |
| 6 | `safe_double_to_int`    | `int safe_double_to_int(double d)`                                      | T | T | OK |
| 7 | `maxnmin`               | `int maxnmin(int p1, int p2, int p3, int p4)` (the only symbol in `include/lib.h`) | T | T | OK |

**Missing from Rust `.so`: none.** No stubs were needed; every symbol has a real
translation in `translation/src/lib.rs` behind `#[unsafe(no_mangle)] extern "C"`.

## Non-exported C state (must be replicated, not exported)

| C declaration | Rust counterpart | notes |
|---|---|---|
| `static Node node_storage[MAX_NODES]` (100 × 80 B) | `static mut NODE_STORAGE: [Node; 100]` | `#[repr(C)]`, zero-initialised (BSS in both) |
| `static int node_count = 0`                          | `static mut NODE_COUNT: c_int`          | zero-initialised |

Both are file-local in C and crate-local in Rust — correctly *absent* from both
`nm -D --defined-only` listings.

`Node` layout verified identical (x86-64 SysV): `sizeof == 80`, `alignof == 8`,
offsets `id=0`, `parent_id=4`, `name=8`, `value=64`, `active=72`.

## Undefined (imported) symbols

| library | undefined symbols |
|---|---|
| C    | `strncpy`, plus the usual weak CRT hooks (`__cxa_finalize`, `__gmon_start__`, `_ITM_*`) |
| Rust | libc/libgcc only: `malloc`, `free`, `realloc`, `calloc`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`, `abort`, `__errno_location`, `getenv`, `getcwd`, `readlink`, `realpath`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `fstat64`, `stat64`, `mmap64`, `munmap`, `dl_iterate_phdr`, `syscall`, `pthread_key_*`, `pthread_setspecific`, `__tls_get_addr`, `_Unwind_*`, weak `gettid`/`statx`/`__cxa_thread_atexit_impl` + weak CRT hooks |

**0 missing / undefined non-libc symbols in the Rust `.so`** — `ldd` resolves to
`libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2` only (the `_Unwind_*` and
`pthread_*` imports come from the Rust `std` runtime that a `cdylib` links in,
not from untranslated code).

## Results

* `./check_symbols.sh` → *"OK: symbol diff (C -> Rust) is empty"* and
  *"OK: Rust .so has 0 undefined non-libc symbols"*.
* `tests/symbol_parity.rs` asserts the same three things from inside the test
  suite (`d1` = every C export present in Rust + the C export set is exactly the
  7 functions, `d2` = no non-libc undefined symbols, `d3` = all 7 resolve through
  `dlsym` in **both** libraries and are callable).
* Nothing was stubbed. No C source file was skipped: `c_src` contains exactly one
  translation unit (`src/lib.c`, 176 lines) and one header (`include/lib.h`,
  1 line), and every function in it is translated in `translation/src/lib.rs`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the complete set
of feature combinations is:

| combo | cargo invocation |
|---|---|
| default (empty)          | `cargo test --release` |
| `--no-default-features`  | `cargo test --release --no-default-features` |
| `--all-features`         | `cargo test --release --all-features` |

All three are byte-identical builds; `translation/check_features.sh` runs the
full differential suite under each of them.
