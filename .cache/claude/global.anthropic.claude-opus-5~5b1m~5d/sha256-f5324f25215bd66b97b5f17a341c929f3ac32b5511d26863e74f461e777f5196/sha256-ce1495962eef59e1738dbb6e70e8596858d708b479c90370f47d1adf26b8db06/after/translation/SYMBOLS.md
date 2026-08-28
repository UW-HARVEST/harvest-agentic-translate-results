# SYMBOLS.md — Public symbol surface (Phase A)

Derived mechanically from `nm -D` on both shared objects.

Build commands:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` defined dynamic symbols (`nm -D --defined-only c_src/build/libdriver.so`)

```
00000000000011c9 T get_os_arch
0000000000001378 T parse_uname_string
00000000000012ca T w_regexec
```

## Rust `.so` defined dynamic symbols (`nm -D --defined-only translation/target/release/libdriver.so`)

```
0000000000012070 T get_os_arch
00000000000121d0 T parse_uname_string
0000000000012820 T w_regexec
```

## Parity table

| # | symbol | in C `.so` | in Rust `.so` | source of the C definition | note |
|---|--------|-----------|---------------|-----------------------------|------|
| 1 | `get_os_arch`        | T | T | `c_src/src/lib.c:17`  | non-`static`, therefore exported |
| 2 | `w_regexec`          | T | T | `c_src/src/lib.c:32`  | non-`static`, therefore exported |
| 3 | `parse_uname_string` | T | T | `c_src/src/lib.c:57`  | the only symbol declared in `include/lib.h` |

**Symbol diff (C-defined minus Rust-defined): EMPTY.**

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so     | awk '{print $NF}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

No C source file/module was left untranslated: `c_src/src/lib.c` is the only
translation unit in `c_src/CMakeLists.txt` and all three of its non-`static`
functions have real (non-stub) Rust implementations. There are no
macro-generated symbols in this library.

## Types on the ABI boundary

| type | C definition | Rust definition | verified |
|------|--------------|-----------------|----------|
| `os_data` | `include/lib.h:1-11`, 9 × `char *` | `#[repr(C)] pub struct os_data`, 9 × `*mut c_char` | 72 bytes on x86_64 |
| `regmatch_t` | `<regex.h>`, 2 × `regoff_t` | `#[repr(C)]` 2 × `c_int` | probe: `sizeof(regoff_t)=4`, `sizeof(regmatch_t)=8`, `offsetof(rm_eo)=4` |
| `regex_t` | `<regex.h>`, 64 bytes / align 8 | opaque `[u8; 128]`, align 16 (over-sized, never inspected) | probe: `sizeof(regex_t)=64 alignof=8` |
| `REG_EXTENDED` | `1` | `1` | probe |

## Undefined (imported) symbols

The C `.so` imports only libc: `fprintf free malloc regcomp regexec regfree
snprintf stderr strchr strdup strlen strstr` (`strchr` is GCC's rewrite of
`strstr(s, "|")`).

The Rust `.so` imports the same libc set plus the Rust standard library's own
runtime imports (`_Unwind_*`, `__errno_location`, `abort`, `calloc`,
`dl_iterate_phdr`, `mmap64`, `pthread_key_*`, `memcpy`, …). All are libc /
`libgcc_s` symbols resolved by the dynamic loader; **0 missing/undefined
non-libc symbols**.

## Enforcement

Symbol parity is not just a one-off command; it is a test:

* `tests/phase_d_symbols.rs::every_c_symbol_is_exported_by_the_rust_so` —
  runs `nm -D --defined-only` on both `.so`s and fails on any missing name. It
  also pins `cs.len() == 3`, so if the C's public surface ever grows the test
  fails rather than silently passing.
* `tests/phase_d_symbols.rs::every_symbol_is_dlsym_able_from_both_so_files` —
  resolves all three symbols with their real signatures and smoke-calls each.
* `tests/phase_d_symbols.rs::rust_so_has_no_undefined_non_libc_symbols` —
  asserts the Rust `.so` does not *import* `get_os_arch`, `w_regexec` or
  `parse_uname_string`, i.e. it genuinely defines them rather than being a thunk
  that forwards to the C library.
* `scripts/verify_gate.sh` re-runs the `nm -D` diff for the **debug and release**
  `.so` and fails unless both diffs are empty.

## Build reproducibility

`libloading` (the only dev-dependency) is vendored into `vendor/`, and
`.cargo/config.toml` sets `net.offline = true` with a vendored-source
replacement, so `cargo test` works with no network access and no pre-populated
registry cache.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (the empty/default feature set). `scripts/check_features.sh`
enumerates features from `Cargo.toml` and loops over every combination; with an
empty feature set it degenerates to the single default build, which is the
configuration validated in Phases B–D.
