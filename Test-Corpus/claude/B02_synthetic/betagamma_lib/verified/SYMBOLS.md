# SYMBOLS.md — dynamic-symbol parity between the C `.so` and the Rust `.so`

## How this was produced

```sh
# C shared library
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libtranslated_rust.so

# Rust shared library
cargo build            # -> target/debug/libbetagamma_lib.so

nm -D --defined-only c_src/build/libtranslated_rust.so
nm -D --defined-only target/debug/libbetagamma_lib.so
```

The automated check lives in `tests/symbol_parity.rs` (it re-runs `nm -D` on both
objects at test time and asserts the C→Rust difference is empty).

## Build configurations

`c_src/CMakeLists.txt` builds a single target from a single translation unit
(`src/lib.c`), with **no** `-D` defines, **no** `option()`/`if()` blocks and no
conditionally-compiled sources.  `src/lib.c` contains **no `#ifdef`** at all
(verified with `grep -n '#if\|#ifdef\|#ifndef' c_src/src/lib.c` → no matches).
Therefore the C library has exactly **one** build configuration.

Mirroring that, `Cargo.toml` declares only an empty `default` feature, so the
complete set of valid feature combinations is:

| # | feature combination | cargo invocation |
|---|---------------------|------------------|
| 1 | *(none)* | `cargo check/test --no-default-features` |
| 2 | `default` (= empty) | `cargo check/test` |

Both are byte-for-byte the same code path; both are exercised by
`scripts/verify_all.sh`, and each is verified under **both** cargo profiles
(`dev` and `release`, the latter with `panic = "abort"`), because optimisation
level changes whether Rust's UB instrumentation is present — a real behavioural
axis for a library whose C original dereferences user-supplied pointers with no
null checks.

## Exported (defined, dynamic) symbols

C `.so` exports exactly five global text symbols.  All five are exported by the
Rust `.so` under the identical unmangled name via `#[unsafe(no_mangle)] pub
unsafe extern "C"`.

| # | symbol | C declaration | present in C `.so` | present in Rust `.so` | Rust item |
|---|--------|---------------|--------------------|------------------------|-----------|
| 1 | `create_block`   | `DataBlock create_block(int id, const char *name, uint8_t flags)` | ✅ `T` | ✅ `T` | `src/lib.rs::create_block` |
| 2 | `allocate_block` | `MemoryBlock *allocate_block(size_t count, int init_value)`        | ✅ `T` | ✅ `T` | `src/lib.rs::allocate_block` |
| 3 | `free_block`     | `void free_block(MemoryBlock *mb)`                                | ✅ `T` | ✅ `T` | `src/lib.rs::free_block` |
| 4 | `compute_hash`   | `int compute_hash(MemoryBlock *mb1, MemoryBlock *mb2)`            | ✅ `T` | ✅ `T` | `src/lib.rs::compute_hash` |
| 5 | `betagamma`      | `int betagamma(int a, int b, int c, int d)` (the only symbol in `include/lib.h`) | ✅ `T` | ✅ `T` | `src/lib.rs::betagamma` |

`nm -D --defined-only` on the C object (verbatim):

```
0000000000001169 T create_block
00000000000011db T allocate_block
000000000000128d T free_block
00000000000012ca T compute_hash
0000000000001333 T betagamma
```

### Symbols missing from the Rust `.so`

**None.**

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort) \
           <(nm -D --defined-only target/debug/libbetagamma_lib.so   | awk '{print $3}' | sort)
   (empty)
```

No C source file was left untranslated: `c_src/src/lib.c` is the only `.c` file
in the project and every one of its five function definitions (all of them
external-linkage) has a Rust counterpart with a `#[unsafe(no_mangle)]` wrapper.
There are no `static` (internal-linkage) helpers, no macro-generated symbols, no
global/`extern` data objects and no `__attribute__((constructor))` in the C
source, so nothing else can appear in the C dynamic symbol table.

### Undefined (imported) symbols

| symbol | imported by C `.so` | imported by Rust `.so` | note |
|--------|---------------------|------------------------|------|
| `malloc@GLIBC_2.2.5`  | ✅ | ✅ | Rust calls the *platform* allocator, not Rust's `GlobalAlloc`, because `compute_hash` observes raw allocator addresses. |
| `calloc@GLIBC_2.2.5`  | ✅ | ✅ | idem (zeroing behaviour matters for `count == 0` and for the `nmemb*size` overflow check). |
| `free@GLIBC_2.2.5`    | ✅ | ✅ | idem — blocks must be freeable by either library interchangeably. |
| `strcpy@GLIBC_2.2.5`  | ✅ | ✅ | The Rust `strcpy_raw` helper forwards to the **same** libc routine.  A hand-written Rust copy loop was tried first and diverged: for `create_block(id, NULL, flags)` the C library dies with `SIGSEGV` while an instrumented Rust deref aborts with `SIGABRT` ("null pointer dereference occurred").  Calling libc `strcpy` restores byte-identical behaviour in *both* the dev and release profiles. |
| `memcpy@GLIBC_2.14`   | ➖ | ✅ | Rust-only import.  `compute_hash` loads `mb->data` with a libc `memcpy` instead of `(*mb).data` for exactly the same reason as `strcpy` above: the C code has no null check there, so the load must be an uninstrumented machine load that faults with `SIGSEGV`. |
| `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__`, `__cxa_finalize` | `w` (weak) | — | toolchain/CRT artefacts, not part of the library's API surface. |

The Rust `.so` additionally exports/imports its own standard-library and
personality symbols (`rust_eh_personality`, `_Unwind_*`, `memcpy`, …).  Extra
Rust-internal symbols are harmless — the requirement is only that every symbol
the C `.so` exports is also exported by the Rust `.so`, which holds.

## Verification checklist

- [x] `nm -D` shows **0** C-exported symbols missing from the Rust `.so`.
- [x] `nm -D` shows **0** undefined non-libc symbols in the Rust `.so`
      (`ldd`/`nm -D -u` list only `libc`/`libgcc_s`/`ld-linux` providers).
- [x] Holds for feature combination 1 (`--no-default-features`).
- [x] Holds for feature combination 2 (`default`).
- [x] Holds for the `release` profile of both combinations.

Automated by `tests/symbol_parity.rs`
(`every_c_symbol_is_exported_by_rust`, `rust_so_has_no_unresolved_non_libc_symbols`)
and by the `symbol parity (nm -D)` step of `scripts/verify_all.sh`.
