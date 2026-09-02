# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```
nm -D --defined-only c_src/build/libharvest-work-Dg6S6m.so
nm -D --defined-only translation/target/release/libbetagamma_lib.so
```

## C `.so` exported symbols (all `T`, dynamic)

| # | symbol | C signature (`c_src/src/lib.c`) | present in Rust `.so`? |
|---|--------|---------------------------------|------------------------|
| 1 | `allocate_block` | `MemoryBlock* allocate_block(size_t count, int init_value)` | YES |
| 2 | `betagamma`      | `int betagamma(int, int, int, int)` (the only symbol in `include/lib.h`) | YES |
| 3 | `compute_hash`   | `int compute_hash(MemoryBlock *mb1, MemoryBlock *mb2)` | YES |
| 4 | `create_block`   | `DataBlock create_block(int id, const char *name, uint8_t flags)` | YES |
| 5 | `free_block`     | `void free_block(MemoryBlock *mb)` | YES |

There are no `static` functions, no macro-generated symbols, no exported data
objects, and no `#ifdef`-gated alternate definitions in `c_src/src/lib.c`, so
the list above is the complete C surface.

## Diff result

```
comm -23 c_syms.txt rust_syms.txt   ->   (empty)
```

**0 symbols missing from the Rust `.so`.** No wrappers had to be added and no C
module was left untranslated: `c_src` contains exactly one translation unit
(`src/lib.c`, per `CMakeLists.txt`), and all five of its external definitions
have real Rust implementations (no stubs, no `unimplemented!()`).

## Undefined symbols in the Rust `.so`

All `U`/`w` entries are libc / libgcc-unwind / TLS runtime imports:
`malloc`, `calloc`, `free`, `strcpy`, `memcpy`, `memmove`, `memset`, `strlen`,
`bcmp`, `realloc`, `posix_memalign`, `abort`, `__errno_location`, the
`_Unwind_*` family, `dl_iterate_phdr`, `pthread_key_*`, and the
`open64`/`read`/`write`/`stat64`/`mmap64` syscall wrappers pulled in by the Rust
`std` panic/backtrace machinery.

**0 missing/undefined non-libc symbols.**

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the only build
configurations are the default (empty feature set) and
`--no-default-features` — which are identical. Both are exercised by
`scripts/verify_all.sh`.

## Verification status

Re-checked after the final build (`scripts/verify_all.sh`):

```
$ comm -23 <(nm -D --defined-only c_src/build/*.so       | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only translation/target/release/libbetagamma_lib.so \
                                                          | awk '{print $NF}' | sort -u)
(empty)
```

* 5/5 C symbols exported by the Rust `.so`, exact names.
* 0 missing symbols; 0 unresolved non-libc symbols.
* Enforced in-test by `tests/phase_d_symbols.rs`, which also asserts the C
  surface has not changed and that **no export is a stub** — every symbol is
  additionally checked to produce the C's real observable effect
  (`phase_d_no_symbol_is_a_stub`), since a symbol that merely exists would
  satisfy `nm` while lying about behaviour.

### A caveat worth recording

Midway through verification, `translation/src/lib.rs` was replaced in the
working tree by a translation of a *different* C library (exporting `arity`,
`arity2`, `arity3`, `arity4`, `shift_array`, `process_string`, `apply_bitmask`,
`init_matrix`, `compare_allocations`) while `c_src/` remained unchanged. That
left **0 of 5** required symbols present. The file was restored to the
translation of *this* `c_src` and everything was re-verified from scratch. The
symbol-parity test in `phase_d_symbols.rs` detects exactly this class of
mismatch, and is why it asserts the expected C surface explicitly rather than
just diffing whatever two `.so` files happen to be present.
