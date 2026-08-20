# SYMBOLS.md — Public symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libdriver.so`   (cmake, default configuration)
* Rust`.so`: `target/debug/libdriver.so` and `target/release/libdriver.so`

Reproduce with:

```sh
nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only target/debug/libdriver.so | awk '$2=="T"{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms      # must be empty
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | source (C) | Rust impl |
|---|--------|---------|------------|------------|-----------|
| 1 | `get_os_arch`        | `T` | `T` | `c_src/src/lib.c:17`  | `src/lib.rs` `#[no_mangle] get_os_arch` |
| 2 | `w_regexec`          | `T` | `T` | `c_src/src/lib.c:32`  | `src/lib.rs` `#[no_mangle] w_regexec` |
| 3 | `parse_uname_string` | `T` | `T` | `c_src/src/lib.c:57`  | `src/lib.rs` `#[no_mangle] parse_uname_string` |

**Missing from Rust `.so`: 0.**
There is exactly one C translation unit (`c_src/src/lib.c`, the only entry in
`add_library(driver SHARED ...)`), and all three of its non-`static` functions
are translated and exported. No C source file was skipped, so no symbol needed
to be newly translated or wrapped.

Note: `get_os_arch` and `w_regexec` are *not* declared in the public header
`include/lib.h` (only `parse_uname_string` is), but they have external linkage
in C and therefore appear in the dynamic symbol table. They are part of the
verified surface and are driven directly by the differential tests.

## Undefined symbols (imports)

The C `.so` imports only libc symbols:

```
fprintf free malloc regcomp regexec regfree snprintf stderr strchr strdup strlen strstr
```

(`strchr` appears because gcc rewrites the single-character `strstr(s, "|")`
into `strchr`; it is not a distinct project symbol.)

The Rust `.so` imports the same libc set — `fprintf free malloc regcomp regexec
regfree snprintf stderr strdup strlen strstr` — plus the Rust runtime's own
libc/`_Unwind_*` needs (`memcpy`, `abort`, `dl_iterate_phdr`, panic-machinery,
…). Those are runtime support, not project symbols.

**Undefined non-libc project symbols in the Rust `.so`: 0.**

## ABI facts verified against this platform (`gcc` probe)

| item | C value | Rust declaration | ok |
|------|---------|------------------|----|
| `REG_EXTENDED` | `1` | `const REG_EXTENDED: c_int = 1` | yes |
| `REG_NOMATCH`  | `1` | (only compared against `0`)      | yes |
| `sizeof(regoff_t)` | `4` | `pub type regoff_t = c_int` | yes |
| `sizeof(regmatch_t)` | `8` | `#[repr(C)] { rm_so, rm_eo }` = 8 | yes |
| `sizeof(regex_t)` / align | `64` / `8` | opaque `[u8;256]`, align 16 | yes (over-sized on purpose) |
| `os_data` | 9 × `char *` = 72 bytes | `#[repr(C)]` 9 × `*mut c_char` | yes |

## Build configurations

`Cargo.toml` has **no `[features]` section**, so the only feature combination is
the empty/default one. `c_src/CMakeLists.txt` has no `option()`, no
`target_compile_definitions`, and no `#ifdef`-driven variants; the single
configuration is the default one. See `CONFIGS.md` for the *runtime*
configuration surface, which is where all the real variation lives.

Both Rust profiles are nevertheless verified, because they differ in
`debug-assertions`/`overflow-checks` (and `[profile.release] panic = "abort"`),
which can change behaviour on arithmetic that is well-defined in C:

* `dev`     (`cargo test`)
* `release` (`cargo test --release`)
