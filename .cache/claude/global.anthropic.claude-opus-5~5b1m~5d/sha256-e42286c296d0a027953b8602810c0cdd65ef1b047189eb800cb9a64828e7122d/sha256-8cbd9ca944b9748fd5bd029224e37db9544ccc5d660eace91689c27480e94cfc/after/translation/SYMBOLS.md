# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-g4xO1U.so`
* Rust `.so`: `translation/target/release/libdataentry_lib.so`

## C source inventory

`c_src` contains exactly one translation unit (`src/lib.c`) and one public
header (`include/lib.h`). The header declares a single entry point:

```c
int dataentry(int a, int b, int c, int d);
```

All other functions in `lib.c` are `static` (internal linkage) and therefore
have *no* dynamic symbol; they are not part of the ABI surface and must not be
exported by the Rust `.so` either:

| C function | linkage | exported? |
|---|---|---|
| `find_entry` | `static` | no (file-local) |
| `process_name` | `static` | no (file-local) |
| `calculate_lookup` | `static` | no (file-local) |
| `create_entries` | `static` | no (file-local) |
| `modify_entries` | `static` | no (file-local) |
| `lookup_table` | `static` data | no (file-local) |
| `dataentry` | external | **yes** |

## Exported (defined) dynamic symbols

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `dataentry` | `T` (0x13fb) | `T` (0x11b00) | **MATCH** |

Symbol diff (`C defined` minus `Rust defined`, text/data symbols only): **empty**.

No symbol required a new `#[no_mangle]` wrapper and no C module was left
untranslated — `src/lib.c` is the whole library and every function in it
(including all six `static` helpers) is present in `translation/src/lib.rs`.

## Undefined (imported) symbols

Both objects import only libc / platform runtime symbols. Neither imports a
non-libc symbol that the other does not provide.

* C imports: `malloc`, `free`, `sprintf`, `strcpy`, `strlen`
  (+ weak `_ITM_*`, `__cxa_finalize`, `__gmon_start__`).
* Rust imports: `malloc`, `free`, `strlen`, `memcpy`, `memset`, `memmove`,
  `calloc`, `realloc`, `posix_memalign`, `abort`, `bcmp`, plus the Rust
  `std`/panic-runtime set (`_Unwind_*`, `dl_iterate_phdr`, `pthread_key_*`,
  `open64`/`read`/`write`/`stat64`/`mmap64`/…) and weak
  `_ITM_*`, `__cxa_*`, `__gmon_start__`, `gettid`, `statx`.

The extra Rust imports are the Rust standard-library/unwinder runtime, not
library-level dependencies; `sprintf`/`strcpy` are re-implemented in Rust
(`sprintf_entry_name`, `c_strcpy`, `c_strcpy_lit`) rather than imported.

### Gate

- [x] `nm -D` shows **0 missing** exported symbols in the Rust `.so`.
- [x] `nm -D` shows **0 undefined non-libc / non-Rust-runtime** symbols in the
      Rust `.so`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one (`--no-default-features` is equivalent). The
Phase D "every feature combination" requirement therefore collapses to the
single default configuration; this is verified in
`tests/feature_matrix.rs` / `check_features.sh`.
