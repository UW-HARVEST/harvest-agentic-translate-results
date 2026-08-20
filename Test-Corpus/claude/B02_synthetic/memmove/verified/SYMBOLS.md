# SYMBOLS.md — exported-surface parity (Phase A / Phase D)

## How the two shared objects are produced

The upstream `c_src/CMakeLists.txt` only defines an **executable** target
(`add_executable(driver src/main.c src/lib.c)`), so no `.so` is emitted by
CMake.  `c_src/` is never modified; `build.rs` compiles the pristine sources
twice into `$OUT_DIR`:

| artefact | command | purpose |
|----------|---------|---------|
| `libcdriver.so` | `cc -O2 -shared -fPIC -o libcdriver.so c_src/src/lib.c` | reference `.so` loaded with `libloading` |
| `c_driver`      | `cc -O2 -o c_driver c_src/src/main.c c_src/src/lib.c`     | reference CLI (same as the CMake `driver` target) |

The Rust side is built as `crate-type = ["lib", "cdylib"]`, producing
`target/<profile>/libdriver.so`.  Integration tests materialise it on demand
(`cargo test` does not build non-test crate types) and then reach it **only**
through `dlopen`/`dlsym`.

## Defined (exported) symbols

`nm -D --defined-only` on each object, filtering out the linker/CRT bookkeeping
weak symbols (`_ITM_*`, `__gmon_start__`, `__cxa_finalize`, `_edata`, `_end`,
`__bss_start`, `_init`, `_fini`) and, on the Rust side, the language-runtime
symbols the Rust compiler always emits (`rust_eh_personality`,
`__rust_alloc*`, `_ZN*`/`_R*` mangled items — none of which are part of the
public C ABI).

| # | symbol | C `.so` | Rust `.so` | notes |
|---|--------|---------|------------|-------|
| 1 | `process_buffer` | `T` (0x1200) | `T` | `size_t process_buffer(uint8_t *buffer, size_t length, uint32_t flags, int param1, int param2)` |

**Missing from the Rust `.so`: none.**  The C translation unit `c_src/src/lib.c`
declares every helper `static`, so `process_buffer` is its *only* external
definition:

```c
static size_t compact_runs(uint8_t *buf, size_t len, uint8_t threshold);
static void   rotate_buffer(uint8_t *buf, size_t len, int offset);
static size_t remove_duplicates(uint8_t *buf, size_t len, int preserve_order);
static void   interleave_halves(uint8_t *buf, size_t len);
static void   reverse_segments(uint8_t *buf, size_t len, size_t seg_size);
```

Those five helpers are `static`/file-local and therefore **must not** appear in
either `.so`; they are reachable (and are exercised) only through
`process_buffer`'s flag bits.  The Rust translation mirrors this exactly: the
five helpers are private `fn`s in `src/lib.rs`, and `src/ffi.rs` exports the
single `#[no_mangle] pub unsafe extern "C" fn process_buffer`.

`c_src/src/main.c` contributes `main` to the *executable* only (it is not part
of the `.so`); its Rust counterpart is `src/main.rs`, verified separately by the
CLI-level differential test `tests/driver_cli.rs`.

## Undefined (imported) symbols

The C `.so` imports `memcpy@GLIBC_2.14` and `memmove@GLIBC_2.2.5`.  The Rust
`.so` imports both of those as well (plus the usual Rust std/unwinder set:
`malloc`, `free`, `abort`, `dl_iterate_phdr`, `_Unwind_*`, `pthread_key_*`, …).
There are **0 undefined non-libc symbols** in the Rust `.so` — every `U` entry
resolves against `libc`/`libgcc_s`, which `dlopen` confirms by loading the
object successfully in every test.

## Automated check

`tests/symbol_parity.rs` re-derives both lists with `nm -D` at test time and
fails if any symbol exported by the C `.so` is absent from the Rust `.so`.
