# SYMBOLS.md — Phase A: exported-surface map

Derived mechanically from `nm -D` on both shared objects.

* C   `.so`: `c_src/build/libharvest-work-ean4Zy.so` (built via `c_src/CMakeLists.txt`,
  no `CMAKE_BUILD_TYPE` → unoptimized)
* Rust `.so`: `translation/target/release/libbetagamma_lib.so`

Regenerate with:

```sh
nm -D --defined-only c_src/build/libharvest-work-ean4Zy.so   | awk '{print $3}' | sort > /tmp/c.syms
nm -D --defined-only translation/target/release/libbetagamma_lib.so | awk '{print $3}' | sort > /tmp/r.syms
comm -23 /tmp/c.syms /tmp/r.syms     # MUST be empty
```

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | C source | notes |
|---|--------|---------|------------|----------|-------|
| 1 | `create_block`   | T | T | `src/lib.c:40` | returns `DataBlock` (40 B) **by value** → x86-64 SysV memory/`sret` return |
| 2 | `allocate_block` | T | T | `src/lib.c:48` | returns `MemoryBlock*` from `malloc` |
| 3 | `free_block`     | T | T | `src/lib.c:67` | `void` |
| 4 | `compute_hash`   | T | T | `src/lib.c:74` | reads only the *addresses*/`data` fields; no deref of `data` |
| 5 | `betagamma`      | T | T | `src/lib.c:92`, declared in `include/lib.h:24` | the only symbol in the public header |

**Missing-from-Rust symbols: 0.** Nothing had to be added or translated;
`src/lib.rs` already carries all five implementations with
`#[unsafe(no_mangle)] pub unsafe extern "C"` wrappers, so no stubbing was
required.

Note that only `betagamma` appears in `include/lib.h`; the other four are
non-`static` definitions in `src/lib.c` and therefore have external linkage and
are part of the ABI surface. All four are treated as public entry points and are
tested directly through the `.so` (Phase B requirement: "including the
lowest-level ones").

## Non-exported C types (needed to call the ABI)

| type | layout | size / align |
|------|--------|--------------|
| `DataBlock`   | `int id; char name[32]; uint8_t flags;` | 40 / 4 (3 tail padding bytes) |
| `MemoryBlock` | `int *data; size_t size;`               | 16 / 8 |

Verified against the Rust `#[repr(C)]` definitions by
`tests/differential.rs::layout_parity`.

## Undefined (imported) symbols

The Rust `.so` must not pull in any non-libc dependency. Both objects import
`malloc`, `calloc`, `free`, `strcpy` from glibc — the Rust translation
deliberately calls the *platform* allocator rather than Rust's, because
`compute_hash` observes raw allocator addresses.

| object | imported non-libc symbols |
|--------|--------------------------|
| C      | none (only `malloc`/`calloc`/`free`/`strcpy` + `__cxa_finalize`/`__gmon_start__`/ITM weak stubs) |
| Rust   | none (glibc + `_Unwind_*` from libgcc, which is part of the platform runtime) |

`nm -D --undefined-only` on the Rust `.so` shows **0 missing/undefined non-libc
symbols**: every entry is glibc (`GLIBC_*` versioned), a libgcc unwinder
(`_Unwind_*@GCC_*`), or a weak optional hook.

## Cargo feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, so the crate has exactly one feature configuration. The
`--no-default-features` build is byte-identical in surface; it is still exercised
by `run_all.sh` to prove parity. The two build profiles (`dev` and
`release`, the latter with `panic = "abort"`) are treated as the second
configuration axis and both are verified.
