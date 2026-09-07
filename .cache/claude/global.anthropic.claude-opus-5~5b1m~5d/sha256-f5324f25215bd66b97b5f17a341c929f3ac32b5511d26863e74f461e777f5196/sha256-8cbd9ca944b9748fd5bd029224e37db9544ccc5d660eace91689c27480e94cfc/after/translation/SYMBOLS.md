# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

Build commands used:

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

## C `.so` exported symbols (`c_src/build/libdriver.so`)

```
$ nm -D --defined-only c_src/build/libdriver.so
T get_os_arch
T parse_uname_string
T w_regexec
```

## Rust `.so` exported symbols (`translation/target/release/libdriver.so`)

```
$ nm -D --defined-only translation/target/release/libdriver.so | grep -v _ZN
T get_os_arch
T parse_uname_string
T w_regexec
```

(The Rust `.so` additionally exports the usual `rust_eh_personality` /
`_ZN…` std-internal symbols, plus `__rust_*` allocator shims. Those are
*extra* symbols, which is permitted; the requirement is that every C symbol
is present in Rust.)

## Parity table

| # | symbol | C `.so` | Rust `.so` | declared in | notes |
|---|--------|---------|------------|-------------|-------|
| 1 | `get_os_arch`       | `T` | `T` | `src/lib.c:17` (not in public header) | `char *get_os_arch(char *os_header)` — returns `strdup`'d arch or NULL |
| 2 | `w_regexec`         | `T` | `T` | `src/lib.c:32` (not in public header) | `int w_regexec(const char*, const char*, size_t, regmatch_t*)` |
| 3 | `parse_uname_string`| `T` | `T` | `include/lib.h:13` | `void parse_uname_string(char *uname, os_data *osd)` |

**Missing from Rust: NONE.** Symbol diff is empty.

```
$ diff <(nm -D --defined-only c_src/build/libdriver.so       | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libdriver.so | awk '$2=="T"{print $3}' | sort | grep -E '^(get_os_arch|parse_uname_string|w_regexec)$')
   # (empty)
```

## Undefined (imported) symbols

The C `.so` imports only libc/POSIX: `fprintf free malloc regcomp regexec
regfree snprintf stderr strchr strdup strlen strstr` (+ the standard
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` glue).

The Rust `.so` imports the identical libc/POSIX set (`fprintf free malloc
regcomp regexec regfree snprintf stderr strchr strdup strlen strstr`) plus
Rust-runtime libc/unwind symbols (`_Unwind_*`, `abort`, `memcpy`, `mmap64`,
`pthread_key_*`, …). **0 missing / 0 non-libc undefined symbols.**

## Exported types (ABI)

`os_data` (from `include/lib.h`) — 9 × `char *`, 72 bytes on x86-64, mirrored
by `#[repr(C)] pub struct os_data` in `src/lib.rs`. Field order verified
identical: `os_name, os_version, os_major, os_minor, os_codename, os_platform,
os_build, os_uname, os_arch`.

`regmatch_t` — `{ int rm_so; int rm_eo; }` (glibc `regoff_t` == `int` on this
platform), mirrored by `#[repr(C)] pub struct regmatch_t`.

## Cargo features

`translation/Cargo.toml` declares **no `[features]` table**, so the only
feature combination that exists is the default (empty) one. Phase D's
"every feature combination" therefore reduces to a single configuration;
this is verified explicitly by the loop in `check_features.sh`.
