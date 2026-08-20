# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared libraries.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
#   -> c_src/build/libdriver.so

# Rust
cargo build --offline            # -> target/debug/libdriver.so
```

## C `.so` — all `nm -D` entries

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U div@GLIBC_2.2.5
0000000000001119 T driver
                 U printf@GLIBC_2.2.5
```

`nm -D --defined-only c_src/build/libdriver.so` yields exactly one symbol.

## Defined (exported) symbol table

| # | C symbol | C type | Exported by Rust `.so` | Rust type | Status |
|---|----------|--------|------------------------|-----------|--------|
| 1 | `driver` | `T` (global text) | yes — `#[unsafe(no_mangle)] pub extern "C" fn driver` | `T` | ✅ present |

`w` (weak) entries — `_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`,
`__gmon_start__`, `__cxa_finalize` — are toolchain/CRT artifacts, not library
API. They are *undefined weak* references in the C `.so`, not definitions, so
they are not part of the surface the Rust `.so` must define. (`__cxa_finalize`
and `__gmon_start__` also appear as undefined weak refs in the Rust `.so`.)

`U` entries — `div`, `printf` — are imports from glibc, not exports.

* `printf` — the Rust translation imports the very same `printf@GLIBC_2.2.5`,
  so formatting and the process-wide `stdout` buffering are shared.
* `div` — the Rust translation does *not* import glibc's `div`; it inlines the
  equivalent `cltd; idiv` sequence (see `src/lib.rs::c_div`). Verified
  equivalent by disassembly:

  ```
  # glibc div(), as called by C driver: idiv-based, truncates toward zero
  # Rust c_div():
  11e7a: cltd
  11e7b: idiv %esi
  ```

  This reproduces glibc's `div(3)` for every well-defined input *and*
  reproduces its `SIGFPE` traps (see `ERRORS.md` rows 1–2). glibc's
  `div` contains a `if (numer >= 0 && result.rem < 0)` fix-up branch for
  platforms that truncate toward −∞; on x86-64 `idiv` truncates toward zero,
  so that branch is dead and inlining `idiv` is faithful. `CONFIGS.md` row 12
  covers this branch explicitly.

## Symbol diff (Phase D gate)

```sh
comm -23 <(nm -D --defined-only c_src/build/libdriver.so | awk '{print $3}' | sort -u) \
         <(nm -D --defined-only target/debug/libdriver.so | awk '{print $3}' | sort -u)
```

Output: **empty** — 0 C-exported symbols missing from the Rust `.so`.

## Unresolved-symbol check

`ldd -r target/debug/libdriver.so` reports **no** undefined symbols. All `U`
entries in the Rust `.so` resolve to libc/libgcc (`printf`, `malloc`, `memcpy`,
`_Unwind_*`, `pthread_*`, …) — i.e. 0 missing/undefined *non-libc* symbols.

## Completeness of translation

`c_src` contains exactly one translation unit (`src/driver.c`, 31 lines, one
function) and one public header (`include/driver.h`, declaring only `driver`).
`CMakeLists.txt` compiles only `src/driver.c`. No C source file was skipped, so
there is no missing module to translate.

## Phase D — completion gate

Reproduce everything with `./verify_all.sh` (builds the C library, enumerates the
feature power set, runs `cargo check` + `cargo build` + `cargo test` for each
combination in both the dev and release profiles, then diffs the symbols).

Last run:

```
=== Building C reference library ===        [ok] c_src/build/libdriver.so
=== Enumerating feature combinations ===    no [features] declared -> 1 combination
=== combo=<empty> profile=dev ===           [ok] check / build / test (38 tests)
=== combo=<empty> profile=release ===       [ok] check / build / test (38 tests)
=== Default invocation ===                  [ok] cargo check (default features)
=== Symbol parity (nm -D) ===               [ok] debug: 0 missing   [ok] release: 0 missing
=== Unresolved symbols in the Rust .so ===  [ok] none
=== Summary ===                             ALL CHECKS PASSED
```

### Release profile is verified too, deliberately

`[profile.release]` sets `panic = "abort"` and enables optimisation, which is the
one configuration where an `asm!` block could plausibly be optimised away and the
`SIGFPE` traps lost. It is not. Disassembly of `target/release/libdriver.so`:

```
0000000000011c80 <driver>:
   11c80:  mov    %edi,%eax
   11c82:  cltd
   11c83:  idiv   %esi          <-- trap preserved at -O3
   11c85:  lea    ...,%rdi      <-- format string
   11c8c:  mov    %eax,%esi
   11c8e:  xor    %eax,%eax
   11c90:  jmp    *...          <-- tail call to printf@GLIBC_2.2.5
```

and all 38 tests — including the two `SIGFPE` rows — pass in release.

### Final checklist

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing C symbols and 0 undefined non-libc
      symbols in the Rust `.so` (dev and release).
- [x] Phase B: all 21 `CONFIGS.md` rows pass across randomized inputs
      (26 tests, ~37 000 differential calls per profile).
- [x] Phase C: both `ERRORS.md` rows have passing error-path differential tests
      asserting the identical signal (12 tests), plus the generic FFI boundaries.
- [x] Every feature combination (the single empty one, reached both via default
      features and via `--no-default-features`) verified, in both profiles.
- [x] Nothing in `c_src/` was modified; `src/lib.rs` needed no behavioural fix
      and is byte-identical to the translation under test.
