# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libtranslated_rust.so

# Rust
cargo build --no-default-features
nm -D --defined-only target/debug/libmaxnmin_lib.so
```

Translation unit inventory (completeness check — no C file was skipped):

| C source file | translated to | status |
|---|---|---|
| `c_src/src/lib.c` (176 lines, the only source in `CMakeLists.txt`) | `src/lib.rs` | fully translated |
| `c_src/include/lib.h` (1 line, decl of `maxnmin` only) | n/a (declaration only) | n/a |

`CMakeLists.txt` lists exactly one source (`src/lib.c`), so the C `.so` surface is
completely covered. There is no un-translated module.

## Dynamic (exported) symbol table

| # | symbol | C `.so` | Rust `.so` | signature (from C) | Rust definition |
|---|--------|---------|-----------|--------------------|-----------------|
| 1 | `add_node` | `T` | `T` | `int add_node(int id, int parent_id, const char *name, double value)` | `src/lib.rs` `#[unsafe(no_mangle)] pub unsafe extern "C" fn add_node` |
| 2 | `find_node_by_id` | `T` | `T` | `Node *find_node_by_id(int id)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn find_node_by_id` |
| 3 | `get_children_count` | `T` | `T` | `int get_children_count(int parent_id)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn get_children_count` |
| 4 | `calculate_subtree_sum` | `T` | `T` | `double calculate_subtree_sum(int node_id)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn calculate_subtree_sum` |
| 5 | `process_string` | `T` | `T` | `int process_string(char *str)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn process_string` |
| 6 | `safe_double_to_int` | `T` | `T` | `int safe_double_to_int(double d)` | `#[unsafe(no_mangle)] pub extern "C" fn safe_double_to_int` |
| 7 | `maxnmin` | `T` | `T` | `int maxnmin(int a, int b, int c, int d)` | `#[unsafe(no_mangle)] pub unsafe extern "C" fn maxnmin` |

Counts: C exports **7**, Rust exports **7** — an exact match, in **both** the
`dev` and the `release` profile:

```
$ comm -23 c_syms.txt rust_syms.txt      # C symbols missing from Rust
(empty)
$ comm -13 c_syms.txt rust_syms.txt      # extra symbols exported by Rust
(empty)
```

`run_all.sh` re-runs this diff for every profile x feature combination and also
guards against a vacuous result (it fails if `nm` lists fewer than 7 symbols).

## Deliberately NOT exported (matches C `static` linkage)

| C declaration | linkage | Rust counterpart |
|---|---|---|
| `static Node node_storage[MAX_NODES];` | internal (`static`), absent from `nm -D` | `static mut NODE_STORAGE` (private, not `#[no_mangle]`) — correctly absent from Rust `nm -D` |
| `static int node_count = 0;` | internal (`static`), absent from `nm -D` | `static mut NODE_COUNT` (private) — correctly absent from Rust `nm -D` |

There are no macro-generated symbols in this library (the only `#define`s are the
object-like constants `MAX_NODES` and `MAX_NAME_LEN`).

## Undefined (imported) symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libmaxnmin_lib.so` lists **only** libc /
libgcc-unwind / GNU-toolchain symbols — there are **0 missing/undefined non-libc
symbols**:

* libc string routines the translation calls on purpose, mirroring `lib.c`:
  **`strncpy`** (from `add_node`) and **`strlen`** (from `process_string`) — see
  the "Divergences found and fixed" section of `ERRORS.md` for why these are
  delegated to libc rather than reimplemented
* libc: `malloc`, `calloc`, `realloc`, `free`, `posix_memalign`, `memcpy`,
  `memmove`, `memset`, `bcmp`, `abort`, `getenv`, `getcwd`, `realpath`,
  `readlink`, `open64`, `close`, `read`, `write`, `writev`, `lseek64`, `stat64`,
  `fstat64`, `statx`, `mmap64`, `munmap`, `syscall`, `gettid`,
  `__errno_location`, `__cxa_finalize`, `__cxa_thread_atexit_impl`,
  `dl_iterate_phdr`, `pthread_key_create`, `pthread_key_delete`,
  `pthread_setspecific`, `__tls_get_addr`
* libgcc unwinder (Rust panic machinery): `_Unwind_*@GCC_*`
* GNU toolchain weak hooks: `__gmon_start__`, `_ITM_registerTMCloneTable`,
  `_ITM_deregisterTMCloneTable`

`ldd` resolves fully against `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`.
(The C `.so` imports only `strncpy`, `__cxa_finalize`, and the two weak GNU hooks.)

## ABI surface shared across the boundary

`find_node_by_id` returns a `Node *` into the library's private static storage, so
the **struct layout is part of the exported ABI** and is verified byte-for-byte by
the differential tests (`layout_*` / field-offset probes):

| field | C offset | size | Rust `#[repr(C)] struct Node` |
|---|---|---|---|
| `int id` | 0 | 4 | `id: c_int` |
| `int parent_id` | 4 | 4 | `parent_id: c_int` |
| `char name[50]` | 8 | 50 | `name: [c_char; 50]` |
| *(padding)* | 58 | 6 | implicit |
| `double value` | 64 | 8 | `value: c_double` |
| `int active` | 72 | 4 | `active: c_int` |
| *(tail padding)* | 76 | 4 | implicit |
| **sizeof** | | **80** | **80** |

## Build configurations

`Cargo.toml` has **no `[features]` table**, and `c_src/CMakeLists.txt` declares no
`option()` / `target_compile_definitions`, and `lib.c` contains **no `#if` / `#ifdef`
/ `#ifndef`** at all. Therefore there is exactly **one** valid build configuration:

| # | Rust invocation | C equivalent |
|---|---|---|
| 1 | `cargo check/build/test --no-default-features` (== the default, empty feature set) | single `cmake` build of `src/lib.c` |

`--no-default-features` and the plain default build are the same configuration; both
are exercised by `./run_all.sh`, which additionally runs everything twice (once per
`cargo` profile, `dev` and `release`) because the two profiles produce genuinely
different machine code and one real divergence only reproduced under `release`.

## Verification status

- [x] `nm -D`: 0 C symbols missing from the Rust `.so` (7/7), 0 extra, in both profiles.
- [x] `nm -D --undefined-only` on the Rust `.so`: 0 unresolved non-libc symbols.
- [x] `ldd` resolves the Rust `.so` completely.
- [x] `sizeof(Node) == 80` and every field offset verified through the exported
      `find_node_by_id` pointer (`tests/smoke.rs`, `tests/configs_nodes.rs`).
- [x] All 54 differential tests pass in `dev` and in `release`.
