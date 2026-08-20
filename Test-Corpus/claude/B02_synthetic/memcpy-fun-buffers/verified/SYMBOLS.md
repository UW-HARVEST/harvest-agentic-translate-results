# SYMBOLS.md — symbol parity between the C and the Rust shared object

## How the two shared objects are produced

`c_src/CMakeLists.txt` only declares `add_executable(driver src/main.c)`, so the
project ships a single translation unit and *no* library target. For symbol
comparison the same translation unit is additionally compiled as a shared
object (this does not modify anything under `c_src/`):

```
gcc -shared -fPIC -O2 -o libdriver_c.so c_src/src/main.c
cargo build --release            # -> target/release/libdriver.so   (crate-type = cdylib)
cargo build --release            # -> target/release/driver          (the executable)
```

The test suite builds the C `.so` itself with `gcc` at run time (see
`tests/common/mod.rs::c_so_path`) and loads **both** objects with `libloading`,
resolving every symbol through `dlsym`. Nothing in `c_src/` is modified.

## Defined (exported) symbols

`nm -D --defined-only` on the C `.so` yields exactly 16 symbols. All 16 are
exported by the Rust `.so` under the identical name:

| # | C symbol (`nm -D`) | C signature | present in Rust `.so` | Rust item |
|---|--------------------|-------------|------------------------|-----------|
| 1 | `calculate_checksum` | `uint32_t (const uint8_t *, size_t)` | yes | `lib_impl::calculate_checksum` |
| 2 | `validate_buffer` | `bool (const buffer_t *)` | yes | `lib_impl::validate_buffer` |
| 3 | `init_buffer_array` | `buffer_array_t *(int)` | yes | `lib_impl::init_buffer_array` |
| 4 | `free_buffer_array` | `void (buffer_array_t *)` | yes | `lib_impl::free_buffer_array` |
| 5 | `buffer_copy` | `int (const buffer_t *, buffer_t *)` | yes | `lib_impl::buffer_copy` |
| 6 | `buffer_reverse` | `int (buffer_t *)` | yes | `lib_impl::buffer_reverse` |
| 7 | `buffer_merge` | `int (const buffer_t *, const buffer_t *, buffer_t *)` | yes | `lib_impl::buffer_merge` |
| 8 | `buffer_split` | `int (const buffer_t *, size_t, buffer_t *, buffer_t *)` | yes | `lib_impl::buffer_split` |
| 9 | `buffer_interleave` | `int (const buffer_t *, const buffer_t *, buffer_t *)` | yes | `lib_impl::buffer_interleave` |
| 10 | `buffer_rotate` | `int (buffer_t *, int)` | yes | `lib_impl::buffer_rotate` |
| 11 | `buffer_conditional_copy` | `int (const buffer_t *, buffer_t *, uint8_t, bool)` | yes | `lib_impl::buffer_conditional_copy` |
| 12 | `buffer_copy_strided` | `int (const buffer_t *, buffer_t *, int)` | yes | `lib_impl::buffer_copy_strided` |
| 13 | `process_buffer_array` | `int (buffer_array_t *, operation_t, int)` | yes | `lib_impl::process_buffer_array` |
| 14 | `read_buffer` | `int (buffer_t *)` | yes | `lib_impl::read_buffer` |
| 15 | `write_buffer` | `void (const buffer_t *)` | yes | `lib_impl::write_buffer` |
| 16 | `main` | `int (int, char **)` | yes | `lib.rs::main` -> `lib_impl::c_main` |

Nothing was stubbed: every one of the 16 symbols is a full translation of the
corresponding C function body. No `unimplemented!()`/`todo!()` exists anywhere
in `src/` (verified by `grep`). The whole C translation unit was translated —
no C file/module was skipped, so no "absent implementation" case arose.

### Why `main` lives in `src/lib.rs`

The C `.so` exports `main`, so the Rust `.so` must too. A `#[no_mangle] extern
"C" fn main` in the library would collide with the executable's own `main`
symbol if the executable linked the library, so the translation is arranged as:

* `src/lib_impl.rs` — the entire translation (symbols 1–15 plus `c_main`).
* `src/lib.rs` — `mod lib_impl;` + the `#[no_mangle] extern "C" fn main`
  wrapper. This is the `rlib`/`cdylib`.
* `src/main.rs` — pulls `lib_impl` in with `#[path = "lib_impl.rs"]` (so it does
  *not* link the library target) and provides the Rust `fn main` shim.

Both targets therefore share one single copy of the translated source and can
never drift apart.

## Extra symbol exported by the Rust `.so` (superset, not a gap)

| Rust symbol | purpose |
|-------------|---------|
| `driver_reset_exported_stdin` | Resets the push-back state of the process-wide reader used by the exported `read_buffer`. The C side reaches the same effect with `freopen(path, "r", stdin)`, which has no Rust equivalent that a test can call. Test-support only; it is not part of the C API. |

Symbol parity requires C ⊆ Rust, so an extra Rust symbol is not a failure.

## Undefined symbols

`nm -D --undefined-only libdriver_c.so`:

```
w _ITM_deregisterTMCloneTable      w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5       w __gmon_start__
U __isoc99_scanf@GLIBC_2.7         U fprintf@GLIBC_2.2.5
U free@GLIBC_2.2.5                 U fwrite@GLIBC_2.2.5
U malloc@GLIBC_2.2.5               U memcpy@GLIBC_2.14
U printf@GLIBC_2.2.5               U putchar@GLIBC_2.2.5
U stderr@GLIBC_2.2.5
```

All are libc/toolchain symbols. The Rust `.so`'s undefined set is likewise
libc-only (`malloc`, `free`, `memcpy`, `write`, `pthread_*`, …).

**Result: 0 missing symbols, 0 undefined non-libc symbols.** Enforced
mechanically by `tests/symbol_parity.rs`, which shells out to `nm -D` on both
objects and asserts the C-defined set is a subset of the Rust-defined set.
