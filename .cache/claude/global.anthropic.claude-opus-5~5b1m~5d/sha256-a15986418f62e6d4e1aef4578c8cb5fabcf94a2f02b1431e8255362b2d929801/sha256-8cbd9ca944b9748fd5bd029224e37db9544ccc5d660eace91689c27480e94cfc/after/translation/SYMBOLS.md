# SYMBOLS.md — Phase A: exported-symbol surface

## Source of truth

`c_src/CMakeLists.txt` builds ONE shared library from ONE translation unit:

```cmake
add_library(${project_name} SHARED src/lib.c)
```

`${project_name}` is derived from the parent directory name, so the artifact is
`c_src/build/libharvest-work-JC6ixj.so`.

## C `.so` dynamic symbols (defined)

```
$ nm -D --defined-only c_src/build/libharvest-work-JC6ixj.so
00000000000011a3 T to_barycentric
```

Exactly one public symbol. `nm` also reports the usual linker-synthesised
entries (`_init`, `_fini`, `__cxa_finalize@GLIBC_2.17`,
`__gmon_start__`, `_ITM_*`) which are toolchain artifacts, not library API.

The other three functions in `c_src/src/lib.c` — `lm_v2`, `lm_sub2`,
`lm_dot2` — are declared `static` (internal linkage) and therefore are
**not** part of the exported surface. They are reproduced in Rust as private
`fn`s (NOT `#[no_mangle]`), which is the faithful match: exporting them would
be a surface *mismatch*.

## Rust `.so` dynamic symbols (defined)

```
$ cd translation && cargo build --release
$ nm -D --defined-only target/release/libto_barycentric_lib.so
0000000000011690 T to_barycentric
```

## Parity table

| # | C symbol | kind | present in Rust `.so` | Rust definition | notes |
|---|----------|------|-----------------------|-----------------|-------|
| 1 | `to_barycentric` | `T` (global text) | YES | `#[unsafe(no_mangle)] pub extern "C" fn to_barycentric` in `src/lib.rs` | signature `lm_vec2(lm_vec2, lm_vec2, lm_vec2, lm_vec2)` |

### Internal-linkage C functions (must NOT be exported by either side)

| # | C symbol | C linkage | exported by C `.so` | exported by Rust `.so` | Rust definition |
|---|----------|-----------|---------------------|------------------------|-----------------|
| i1 | `lm_v2`   | `static` | no | no | private `fn lm_v2` |
| i2 | `lm_sub2` | `static` | no | no | private `fn lm_sub2` |
| i3 | `lm_dot2` | `static` | no | no | private `fn lm_dot2` |

## Diff result

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-JC6ixj.so \
          | awk '{print $3}' | grep -v '^$' | sort) \
       <(nm -D --defined-only translation/target/release/libto_barycentric_lib.so \
          | awk '{print $3}' | grep -v '^$' | sort)
```

- Missing from Rust: **0**
- Undefined non-libc symbols in Rust `.so`: **0** (`nm -D -u` lists only
  glibc/`ld.so` imports: `memcpy`, `__stack_chk_fail`-class, `_ITM_*`,
  `__gmon_start__`, `__cxa_finalize`, `__rust_*` is absent because the crate is
  a `cdylib` with `panic = "abort"`).
- No whole C module was skipped: `src/lib.c` is the only C source file and all
  four of its functions (1 public + 3 static) are translated. No stubs, no
  `unimplemented!()`.

## ABI notes verified

- `lm_vec2` = `{ float x, y; }` = 8 bytes, `align 4`. Under the x86-64 SysV
  ABI this is a single eightbyte of class SSE, so each `lm_vec2` argument is
  passed packed in the low half of one XMM register (`xmm0`..`xmm3` for the
  four parameters) and the return value comes back packed in `xmm0`.
  `#[repr(C)]` + `extern "C"` reproduces this exactly; confirmed by the
  disassembly of both `.so`s and by the differential tests, which pass the
  struct by value across the real FFI boundary via `libloading`.

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only
configurations are `--no-default-features` and the default (identical), plus
the `dev`/`release` profiles. There is no build script and no `#[cfg]` in
`src/lib.rs`, so a single code path serves every configuration. The
differential test suite is run against both profiles and both feature
invocations (see `run_all.sh`).

## Binaries

Neither `c_src/CMakeLists.txt` (`add_library` only, no `add_executable`) nor
`translation/Cargo.toml` (`[lib]` only, no `[[bin]]`, no `src/main.rs`) builds a
driver executable, so the "compare binary stdout" gate is not applicable.
