# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd translation && cargo build --release
```

Artifacts:

* C:    `c_src/build/libSieve.so`
* Rust: `translation/target/release/libSieve.so`

## Defined (exported) symbols

`nm -D --defined-only <so>`:

| symbol | C `.so` | Rust `.so` | notes |
|--------|---------|------------|-------|
| `sieve` | `T` | `T` | `void sieve(int)` — the only symbol declared in `c_src/include/sieve.h`. Rust exports it via `#[unsafe(no_mangle)] pub extern "C" fn sieve(val: c_int)`. |

There are no macro-generated symbols: `c_src/src/sieve.c` contains no
symbol-defining macros, and `sieve.h` only carries an include guard
(`SIEVE_H_`), which emits no symbol.

The C library has exactly one translation unit (`src/sieve.c`, per
`c_src/CMakeLists.txt`), so no C module was skipped by the translation — the
whole library is `sieve()`.

## Symbol diff

```
comm -23 <(nm -D --defined-only c_src/build/libSieve.so       | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only translation/target/release/libSieve.so | awk '{print $NF}' | sort -u)
```

Result: **empty** — 0 symbols exported by the C `.so` are missing from the
Rust `.so`.

## Undefined (imported) symbols

Measured, not assumed (`nm -D --undefined-only`):

| `.so` | needed libraries (`objdump -p`) | notable imports |
|-------|--------------------------------|-----------------|
| C | `libc.so.6` | `printf` |
| Rust | `libgcc_s.so.1`, `libc.so.6` | `printf`, plus the Rust standard library's own runtime needs: `_Unwind_*` (libgcc), `dl_iterate_phdr`, `mmap64`/`munmap`, `pthread_key_*`, `__tls_get_addr`, `open64`/`read`/`close`/`stat64`, `malloc`/`free`, `memcpy`/`memset`/`bcmp`, `abort` |

Every undefined symbol in the Rust `.so` carries a `@GLIBC_*` or `@GCC_*`
version tag (the only exceptions being the unversioned ELF placeholders
`_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable`, `__gmon_start__`,
which the C `.so` has too). So the Rust artifact resolves entirely against
glibc and libgcc — no third-party `.so` a consumer of the C library would not
already have. This is asserted by
`tests/symbols.rs::rust_so_resolves_only_against_libc_and_libgcc`.

The extra libgcc/std imports are the unavoidable cost of linking Rust's
standard library into a `cdylib`; they are not a behavioural difference. Note
that `_Unwind_*` is present even though `[profile.release] panic = "abort"` is
set, because the shipped `std` is precompiled with unwinding.

Importantly, the Rust `.so` **does** import libc `printf` — it does not use
Rust's `println!`. That is what makes the two libraries share one `stdout`
FILE, one buffer and one set of flush points, which the differential tests rely
on and verify directly (`tests/valid_paths.rs::row16_shared_pending_buffer`,
`row19_interleaved_c_and_rust`).

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
buildable configuration is the default one (`--no-default-features` is
equivalent to the default here). Verified by:

```sh
grep -n '\[features\]' translation/Cargo.toml   # no match
cargo build --release --no-default-features     # same single symbol
```

- [x] `nm -D` shows 0 missing symbols in the Rust `.so`, and 0 undefined
      symbols that resolve outside glibc/libgcc.

## Harness credibility (mutation check)

A differential suite that never fails proves nothing, so the harness was
validated against three deliberate mutants of `src/lib.rs`, each reverted
afterwards (`src/lib.rs` restored byte-identical):

| mutant | what it breaks | caught by |
|--------|----------------|-----------|
| `val % 10` → `val.rem_euclid(10)` | negative-input residue sign | 7 of 13 error-path tests, incl. `err05_negative_nine_no_early_stop` |
| move the check before the `printf` | drops the terminating line | 15 of 17 config rows |
| `wrapping_add` → `saturating_add` | the `INT_MAX` overflow wrap | `err03`, `err04`, `row12` |

