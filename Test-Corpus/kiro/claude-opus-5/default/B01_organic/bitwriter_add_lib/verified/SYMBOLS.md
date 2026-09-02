# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

## Build commands

```
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-0ZGyJF.so   (name derives from parent dir name
#    via cmake_path(GET parent FILENAME project_name) in CMakeLists.txt)

cd translation && cargo build --release
# -> translation/target/release/libbitwriter_add_lib.so
```

## C `.so` exported symbols (`nm -D --defined-only`)

```
00000000000010f9 T bitwriter_add
```

Total: **1** exported text symbol.

## Rust `.so` exported symbols (`nm -D --defined-only`)

```
00000000000116a0 T bitwriter_add
```

Total: **1** exported text symbol.

## Parity table

| # | symbol          | in C `.so` | in Rust `.so` | status |
|---|-----------------|------------|---------------|--------|
| 1 | `bitwriter_add` | yes (`T`)  | yes (`T`)     | OK     |

**Symbols missing from Rust `.so`: 0.**
**Undefined non-libc symbols in Rust `.so`: 0.**

There are no macro-generated symbols: `include/lib.h` declares no
function-like macros and no namespace-renaming macros, so the linker name
equals the source-level name. `src/lib.c` contains exactly one function
definition, and it is the one declared in the header — no whole C module was
skipped by the translation, so no additional C source needed translating.

Verified with:

```
diff <(nm -D --defined-only ../c_src/build/libharvest-work-0ZGyJF.so \
        | awk '{print $3}' | sort) \
     <(nm -D --defined-only target/release/libbitwriter_add_lib.so \
        | awk '{print $3}' | sort)
```

## ABI surface

`struct tflac_bitwriter` (from `include/lib.h`), layout confirmed by compiling
a `offsetof`/`sizeof` probe against the real header with the same compiler:

```
size=32 align=8 val=0 bits=8 pos=12 len=16 tot=20 buffer=24
```

No tail or interior padding. The Rust `#[repr(C)] struct tflac_bitwriter`
matches this layout exactly, so the full 32-byte object can be compared
byte-for-byte after each call.

## Cargo features

`translation/Cargo.toml` declares **no `[features]` table**, so the only
feature configuration that exists is the default (empty) one. `--features`
combinations beyond the default are therefore vacuous for this crate; Phase D
records the enumeration that proves this.

## Phase D result

`tests/phase_d_symbols.rs` enforces all of the above as tests:

* `d1_every_c_symbol_is_exported_by_rust` — the set difference
  (C exports − Rust exports) must be empty. **PASSES.**
* `d2_rust_so_has_no_undefined_non_libc_symbols` — every undefined symbol in
  the Rust `.so` (with `@GLIBC_x.y` version suffixes stripped) must be defined
  by a library that `ldd` actually resolves, and `ldd` must report no
  "not found". The allowed set is enumerated mechanically from `ldd` + `nm`,
  not hand-written. **PASSES** (50 undefined, all provided by glibc/ld.so
  except the three weak `__gmon_start__` / `_ITM_*TMCloneTable` symbols that
  are optional by design).
* `d3_struct_layout_matches_c_abi` — re-asserts size/align/offsets. **PASSES.**

Shell equivalent, run by `verify.sh`:

```
$ diff <(nm -D --defined-only c_src/build/libharvest-work-0ZGyJF.so \
          | awk '{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libbitwriter_add_lib.so \
          | awk '{print $3}' | sort)
# (no output)  ->  symbol diff EMPTY
```

## Profile note

The harness loads the **release** cdylib by default, since that is the shipped
artifact (`cargo build --release`; `[profile.release] panic = "abort"`).
`verify.sh` additionally re-runs every suite against the **debug** cdylib via
`RUST_SO=`. The two agree on every input except `ERRORS.md` row E13
(`bw == NULL`), where rustc's debug-profile `ub_checks` turn the C's unchecked
store through a null pointer into a Rust panic — `SIGABRT` instead of the C's
`SIGSEGV`. That is a property of the debug profile's inserted checks, not of the
translation; the release artifact faults identically to C.
