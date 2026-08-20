# SYMBOLS.md — exported-symbol surface (Phase A / Phase D)

Derived mechanically from `nm -D` on the built shared objects.

* C  : `c_src/build/liblong.so`   (built with `cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
* Rust: `target/{debug,release}/liblong.so` (`cargo build [--release] --no-default-features`)

## 1. Raw `nm -D -S --defined-only` output

### C `liblong.so`

```
0000000000004060 0000000000100000 B array
00000000000011f4 00000000000000ae T long_exec
0000000000001139 00000000000000bb T perform_expensive_operations
```

### Rust `liblong.so` (debug)

```
000000000004faa0 0000000000100000 B array
0000000000012470 00000000000002bf T long_exec
0000000000012730 0000000000000252 T perform_expensive_operations
```

### Rust `liblong.so` (release)

```
000000000004fa80 0000000000100000 B array
0000000000011d80 0000000000000149 T long_exec
0000000000011ed0 000000000000007d T perform_expensive_operations
```

Both `array` addresses are 32-byte aligned, matching gcc's alignment of the
1 MiB C `.bss` array (`0x4060`). The Rust translation therefore declares the
storage as `#[repr(C, align(32))] pub struct Array(pub [c_int; 256 * 1024])`;
before that fix the object was only 4-byte aligned, which is observable by a
consumer of the exported data symbol (aligned vector loads). Asserted by
`tests/symbols.rs::array_object_alignment_matches`.

## 2. Symbol parity table

| # | C symbol | C kind | C size | Rust symbol | Rust kind | Rust size | status |
|---|----------|--------|--------|-------------|-----------|-----------|--------|
| 1 | `array`                        | `B` (.bss data) | `0x100000` = 1048576 B = 262144 × `int` | `array` (`#[unsafe(no_mangle)] pub static mut array: [c_int; 256*1024]`) | `B` | `0x100000` | **match** |
| 2 | `long_exec`                    | `T` (text) | `0xae` | `long_exec` (`#[unsafe(no_mangle)] pub extern "C" fn`) | `T` | `0x2bf` / `0x169` | **match** (size differs — codegen only) |
| 3 | `perform_expensive_operations` | `T` (text) | `0xbb` | `perform_expensive_operations` (`#[unsafe(no_mangle)] pub extern "C" fn`) | `T` | `0x252` / `0x7d` | **match** |

**Missing from Rust: none.** The C library consists of exactly one translation
unit (`c_src/src/long.c`, the only file listed in `c_src/CMakeLists.txt`), and
all three of its externally-visible definitions
(`array`, `perform_expensive_operations`, `long_exec`) are present in
`src/clong.rs` with the same linker names. No module of the C source was
skipped, so no additional translation work was required and **no stubs were
added**.

Note that the C header `c_src/include/long.h` only declares `long_exec`; the
other two symbols are exported because `perform_expensive_operations` is a
non-`static` function definition and `int array[ARRAY_SIZE];` is a file-scope
tentative definition with external linkage. Both are therefore part of the ABI
surface and are verified by the differential tests.

## 3. Undefined (imported) symbols

C `liblong.so`, `nm -D -u`:

```
U printf@GLIBC_2.2.5
U rand@GLIBC_2.2.5
U srand@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable
w _ITM_registerTMCloneTable
w __cxa_finalize@GLIBC_2.2.5
w __gmon_start__
```

The Rust `.so` imports the same three *semantically relevant* libc symbols —
`printf`, `rand`, `srand` — i.e. the translation deliberately reuses the
platform PRNG and the platform `printf` instead of re-implementing them, which
is what makes the output byte-identical. (Checked by
`tests/symbols.rs::rust_so_imports_libc_prng_and_printf`.)

The remainder of the Rust `.so`'s undefined symbols are the standard Rust
runtime/`std` imports (`_Unwind_*`, `malloc`, `memcpy`, `pthread_key_create`,
…) plus the same weak ITM/gmon symbols. All of them are libc / libgcc
symbols; there are **0 missing or undefined non-libc symbols**.

## 4. Verification

`tests/symbols.rs` re-derives this table at test time by shelling out to
`nm -D` on both shared objects and asserting

* every symbol defined by the C `.so` is defined by the Rust `.so` with the
  same name **and** the same `nm` type letter
  (`c_and_rust_export_the_same_symbols`, `both_rust_profiles_match_the_c_surface`),
* the Rust `.so` exports nothing beyond the C surface,
* the `array` data symbol has byte-identical size (`0x100000`) and at least the
  same alignment (32 bytes) in both (`array_object_alignment_matches`),
* the Rust `.so` imports the same libc `srand`/`rand`/`printf` the C uses
  (`rust_so_imports_libc_prng_and_printf`),
* nothing is unresolved: `ldd -r` reports no `undefined symbol` for either
  library (`rust_so_has_no_unresolved_symbols`).

Result: **0 missing symbols, 0 undefined non-libc symbols**, for both the debug
and the release Rust profile.
