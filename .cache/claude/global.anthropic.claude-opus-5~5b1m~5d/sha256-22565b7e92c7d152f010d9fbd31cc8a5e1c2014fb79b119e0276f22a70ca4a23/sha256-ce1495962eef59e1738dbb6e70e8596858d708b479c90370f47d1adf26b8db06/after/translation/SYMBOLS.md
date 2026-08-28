# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-0OkA6N.so` (name comes from
  `cmake_path(GET parent FILENAME project_name)` in `c_src/CMakeLists.txt`, i.e.
  the *parent directory* of `c_src`, so it is environment dependent — the tests
  glob for `c_src/build/lib*.so`).
* Rust `.so`: `translation/target/{debug,release}/libenvy_lib.so`
  (`[lib] name = "envy_lib"`, `crate-type = ["cdylib"]`).

Regenerate with:

```sh
nm -D --defined-only c_src/build/lib*.so            | awk '$2=="T"{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libenvy_lib.so \
                                                    | awk '$2=="T"{print $3}' \
  | grep -v '^_ZN' | grep -v '^rust_' | sort > /tmp/r.syms
diff /tmp/c.syms /tmp/r.syms
```

## Exported (defined, `T`) symbols

`c_src/src/lib.c` declares five functions and none of them are `static`, so all
five have external linkage and all five must be exported by the Rust `.so`.

| # | C symbol | C signature (`c_src/src/lib.c`) | in C `.so` | in Rust `.so` | Rust definition |
|---|----------|----------------------------------|-----------|--------------|-----------------|
| 1 | `parse_env_numeric`   | `int parse_env_numeric(const char* env_name, int default_val)`        | ✅ `T` | ✅ `T` | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn parse_env_numeric` |
| 2 | `init_config_from_env`| `void init_config_from_env(struct ConfigFlags* flags)`                | ✅ `T` | ✅ `T` | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn init_config_from_env` |
| 3 | `perform_operation`   | `int perform_operation(int val1, int val2, struct ConfigFlags* flags)`| ✅ `T` | ✅ `T` | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn perform_operation` |
| 4 | `apply_bit_operations`| `int apply_bit_operations(int value, struct ConfigFlags* flags)`      | ✅ `T` | ✅ `T` | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn apply_bit_operations` |
| 5 | `envy`                | `int envy(int p1, int p2, int p3, int p4)` (the only one in `include/lib.h`) | ✅ `T` | ✅ `T` | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn envy` |

**Symbol diff: EMPTY.** No symbol is missing from the Rust `.so`, so no export
wrapper had to be added and no C module was left untranslated. `c_src` contains
exactly one translation unit (`src/lib.c`, 187 lines) and one header
(`include/lib.h`, 1 line); both are fully translated in `translation/src/lib.rs`.

There are no macro-generated exports (the only object-like macro is
`#define BUFFER_SIZE 256`, which generates no symbol) and no exported data
objects (`struct ConfigFlags` / `struct ProcessState` are types only; there are
no file-scope variables in the C source).

## Undefined (imported, `U`/`w`) symbols

The Rust `.so` must not import anything that is not libc / the platform
runtime. Both sides import the same libc entry points, which is what makes the
formatted output byte-identical:

| symbol | C `.so` | Rust `.so` | note |
|--------|---------|-----------|------|
| `getenv`   | U | U | same libc environment view |
| `atoi`     | U | U | delegated, so `atoi` UB/edge behaviour matches exactly |
| `strchr`   | U | U | |
| `printf`   | U | U | |
| `fprintf`  | U | U | |
| `snprintf` | U | U | |
| `stderr`   | U | U | same `FILE*` object |
| `puts`     | U | U | gcc rewrites the argument-less `printf("...\n")` calls into `puts`; Rust calls `printf` — the emitted bytes are identical, and `puts` is still imported by the Rust `.so` via `std` |
| `memcpy`   | (inlined by gcc) | U | 16-byte `struct ProcessState` copy |

Everything else undefined in the Rust `.so` (`_Unwind_*`, `malloc`, `mmap64`,
`pthread_key_create`, `dl_iterate_phdr`, …) belongs to the Rust standard
library / libgcc runtime, is weak, or is plain libc. **0 missing or undefined
non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
buildable configuration is the default one (`--no-default-features` and
`--all-features` produce the identical crate). `tests/feature_matrix.sh`
enumerates the feature list mechanically out of `Cargo.toml` and asserts the set
is empty, so this claim is checked rather than assumed.

## Status

**Symbol diff: EMPTY — verified mechanically, not by inspection.**
`tests/symbol_parity.rs` asserts on every run that

* the C `.so`'s defined-symbol set is exactly the five functions above (so a new
  C source file appearing untranslated would fail the build rather than pass
  silently),
* `c_syms.difference(&r_syms)` is empty,
* `ldd -r` reports no unresolved symbols in the Rust `.so`, and every `nm -D`
  undefined entry is weak or a known libc / platform-runtime import,
* all five symbols resolve through `dlsym` **and** produce non-stub results when
  actually called.

No `#[no_mangle]` wrapper had to be added and no C module was left untranslated:
`c_src` is a single 187-line translation unit plus a 1-line header, and all of it
is present in `src/lib.rs`.
