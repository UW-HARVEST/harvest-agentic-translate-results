# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cd translation && cargo build --release
# -> translation/target/release/libdriver.so
```

## C source inventory (completeness check)

The whole C library is two files, both fully accounted for:

| C file | functions defined | translated in Rust? |
|--------|-------------------|---------------------|
| `c_src/src/driver.c` | `printHexCharLine`, `driver` | yes — `translation/src/lib.rs` |
| `c_src/include/driver.h` | declares `driver` (no code) | yes (ABI mirrored) |

`c_src/CMakeLists.txt` lists exactly one source file (`src/driver.c`), so there
is no untranslated module. No symbol below is a stub; both are real
translations of the corresponding C bodies.

## Defined (exported) dynamic symbols

`nm -D --defined-only`:

| # | symbol | C `.so` | Rust `.so` | C signature | Rust item |
|---|--------|---------|------------|-------------|-----------|
| 1 | `driver`           | `T` | `T` | `void driver(char)`           | `#[unsafe(no_mangle)] pub extern "C" fn driver(c_char)` |
| 2 | `printHexCharLine` | `T` | `T` | `void printHexCharLine(char)` | `#[unsafe(no_mangle)] pub extern "C" fn printHexCharLine(c_char)` |

**Missing from Rust `.so`: none.** The symbol diff (C-exported minus
Rust-exported) is EMPTY.

```
$ comm -23 <(nm -D --defined-only c_src/build/libdriver.so       | awk '{print $NF}' | sort) \
           <(nm -D --defined-only translation/target/release/libdriver.so | awk '{print $NF}' | sort)
(no output)
```

The Rust `.so` additionally exports nothing else of its own; both `T` symbols
above are the complete public surface.

## Undefined (imported) symbols

C `.so` imports: `printf@GLIBC_2.2.5` (plus the standard weak
`_ITM_*` / `__cxa_finalize` / `__gmon_start__` glibc boilerplate).

The Rust `.so` imports `printf@GLIBC_2.2.5` as well — the translation
deliberately calls the *same* libc `printf` with the *same* format string so
that byte output and stdio buffering semantics are identical. Its remaining
undefined symbols are all libc / libgcc-unwinder runtime support
(`malloc`, `memcpy`, `write`, `_Unwind_*`, `dl_iterate_phdr`, …) pulled in by
the Rust standard library.

**0 missing / undefined non-libc symbols in the Rust `.so`.** Verified with
`ldd`, which resolves only `libgcc_s.so.1`, `libc.so.6`, `ld-linux-x86-64.so.2`
and `linux-vdso.so.1`.

## Divergence found and fixed: interposability of the internal call

Matching symbol *names* is not the whole ABI. In the C library
`printHexCharLine` is a non-`static` global, so gcc compiles `driver`'s call to
it as `call printHexCharLine@plt` — an **interposable** call. An `LD_PRELOAD`ed
(or otherwise globally-scoped) replacement therefore takes over the callee that
`driver` uses:

```
$ LD_PRELOAD=shim.so ./probe c_src/build/libdriver.so
SHIM(66)          <- the C driver called the preloaded definition
```

The original Rust translation called `printHexCharLine` by name. In `--release`
LLVM inlined it — LLVM assumes ELF symbols are never interposed, and rustc has
no `-fsemantic-interposition` — so the property was silently lost (the `debug`
build happened to keep it, which is exactly why the test suite now runs against
*both* profiles):

| library | plain | with `LD_PRELOAD=shim.so` |
|---------|-------|---------------------------|
| C | `42` | `SHIM(66)` |
| Rust release (before fix) | `42` | `42`  ← **divergence** |
| Rust debug (before fix) | `42` | `SHIM(66)` |
| Rust release (after fix) | `42` | `SHIM(66)` |
| Rust debug (after fix) | `42` | `SHIM(66)` |

**Fix** (`src/lib.rs`): `driver` now reaches the callee through a
`static` function pointer initialised from an `extern` re-declaration of the
symbol, read with `read_volatile` so LLVM cannot fold it back to the local
definition. The linker emits a real dynamic relocation against the symbol, so
the dynamic loader resolves it exactly like the C's PLT slot:

```
$ readelf -r target/release/libdriver.so | grep printHex
00000004a678  003400000001 R_X86_64_64  0000000000011740 printHexCharLine + 0
```

Regression test: `sym_internal_call_is_interposable_like_the_c` in
`tests/phase_d_symbols.rs` (checks every built profile, and asserts the probe is
non-vacuous by requiring the C library's own behaviour to change under
preload). Mutant `M9` in `mutation_check.sh` confirms the test fails on the
naive direct-call version.

## Divergence found and fixed: argument-register truncation

The x86-64 psABI leaves the **upper 24 bits of the argument register
unspecified** for a sub-word parameter, so gcc's callee does not trust them. For
`void printHexCharLine(char)` at `-O0` it emits:

```
mov    %edi,%eax
mov    %al,-0x4(%rbp)      <- keep the LOW BYTE only
movsbl -0x4(%rbp),%eax     <- sign-extend it
```

The original Rust translation declared the export as `extern "C" fn(c_char)`.
That makes rustc attach LLVM's `signext` attribute, i.e. *assume the caller
already sign-extended*. In `--release` LLVM then dropped the truncation
entirely:

```
printHexCharLine:  mov %edi,%esi      <- forwards all 32 bits, no truncation
```

So a caller passing `0x000000ff` (perfectly legal to write in C as
`((void(*)(int))printHexCharLine)(255)`, and what a naive language binding does)
got:

| | C | Rust release (before fix) |
|---|---|---|
| `printHexCharLine` as `fn(int)`, arg `255` | `ffffffff` | `ff` ← **divergence** |

**Fix** (`src/lib.rs`): both exports now take `c_int` and truncate explicitly
(`charHex as c_char`), reproducing gcc's `mov %al` for every possible register
value. Release codegen is now `movsbl %dil,%esi` — truncate then sign-extend,
exactly the C. This is indistinguishable from the `char` prototype for
well-behaved callers.

Caught by: `cfg_row07_print_full_width_register` / `err_row6_…` (CONFIGS row 7
and 17, ERRORS row 6) — but **only when the suite runs against the release
artifact**, which is why `run_all.sh` now runs every combination against both
profiles. Mutant `M10` in `mutation_check.sh` guards it.

Note that the same change to `driver` is a *provably equivalent* mutant (`E1`):
`driver`'s `+ 1` forces 8-bit codegen (`inc %dil; movsbl %dil,%edi`) with or
without the explicit cast. The cast is kept so the property is guaranteed rather
than incidental.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, therefore the
only configuration is the default one. `--no-default-features` and the default
build are the same compilation. (Verified: `grep -A5 '\[features\]' Cargo.toml`
returns nothing.)
