# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically, not from assumptions:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
nm -D --defined-only c_src/build/libdriver.so

# Rust
cargo build --no-default-features
nm -D --defined-only target/debug/libdriver.so
```

Toolchain of record: `cc (GCC) 11.5.0 20240719 (Red Hat 11.5.0-5)`,
`rustc 1.94.0`. The C build adds no `-O` flag, so the reference library is
`-O0` — this matters for `helperBad` (see below).

## Translation-unit inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | lines | translated to | status |
|---|---|---|---|
| `c_src/src/driver.c` | 68 | `src/driver.rs` | fully translated |
| `c_src/include/driver.h` | 28 | (declares `driver` only) | n/a (header) |

No C source file is skipped, so no symbol is missing because a module was
never translated.

## Defined (exported) symbols

| # | symbol | C `.so` | Rust `.so` | C linkage | notes |
|---|--------|---------|------------|-----------|-------|
| 1 | `printLine` | `T` @ 0x1139 | `T` | extern | `void printLine(const char *)` |
| 2 | `bad`       | `T` @ 0x1186 | `T` | extern | `void bad(void)` |
| 3 | `good`      | `T` @ 0x11ac | `T` | extern | `void good(void)` |
| 4 | `driver`    | `T` @ 0x11c5 | `T` | extern | `void driver(int)`; the only header-declared entry point |

**Missing from Rust `.so`: 0.**

```
$ comm -23 c.syms r.syms   # C-only symbols
(empty)
```

### Deliberately NOT exported (would be a parity *error* to export)

| C symbol | why absent from both `.so`s |
|---|---|
| `helperBad`    | `static char *helperBad()` — internal linkage. Present as a local `t` symbol in the C object, never in `nm -D`. Mirrored by a private `fn helperBad()` in Rust. |
| `helperGood1`  | `static char *helperGood1()` — internal linkage. Same treatment. |
| `charString.0` | the function-local `static char charString[]` inside `helperGood1`; a local data object. Mirrored by the private `HELPER_GOOD1_STRING` static. |

## Undefined symbols

C `.so` imports exactly one non-weak libc symbol:

```
U puts@GLIBC_2.2.5
w _ITM_deregisterTMCloneTable / _ITM_registerTMCloneTable / __cxa_finalize / __gmon_start__
```

`puts` — not `printf` — because GCC folds `printf("%s\n", line)` into
`puts(line)` even at `-O0`. Confirmed in the disassembly:

```
0000000000001139 <printLine>:
    1145:  cmpq   $0x0,-0x8(%rbp)
    114a:  je     1158 <printLine+0x1f>
    1153:  call   1040 <puts@plt>
```

The Rust `.so` also imports `puts@GLIBC_2.2.5` and calls it directly, so both
libraries write through the *same* libc `stdout` `FILE` stream with identical
buffering. The Rust `.so`'s remaining undefined symbols
(`_Unwind_*`, `malloc`, `memcpy`, `pthread_key_create`, `dl_iterate_phdr`, …)
are Rust `std`/unwinder support, all satisfied by `libc`/`libgcc_s`; none is an
unresolved *library* symbol.

## Codegen note: `helperBad` returns NULL, and that is ground truth

```c
static char *helperBad() { char charString[] = "helperBad string"; return charString; }
```

Returning the address of an automatic array is undefined behavior. GCC
documents that when it detects this it **substitutes a null pointer** for the
return value, and the reference library does exactly that:

```
000000000000115b <helperBad>:
    115f:  movabs $0x61427265706c6568,%rax   ; "helperBa" stored to the stack
    ...
    117f:  mov    $0x0,%eax                  ; ...but NULL is returned
    1185:  ret
```

So `bad()` passes NULL to `printLine`, the NULL guard fires, and **`bad()`
prints nothing**. `src/driver.rs::helperBad` returns `ptr::null_mut()`, which
reproduces this byte-for-byte without committing UB in Rust. This is verified
differentially, not assumed — see `tests/error_paths.rs::err_e2_bad_is_silent`.

## Feature combinations (Phase A enumeration)

`Cargo.toml` has **no `[features]` table**, no optional dependencies, and
`grep -rn 'feature *=' src/` finds no `cfg(feature = ...)`. The C side has no
`#ifdef` other than the `DRIVER_H_` include guard and no compile definitions in
`CMakeLists.txt`. The complete set of valid feature combinations is therefore
exactly one:

| # | combination | `cargo check --no-default-features --features <combo>` |
|---|---|---|
| 1 | `<none>` (empty feature set == default) | clean, 0 warnings |

## Optimization-level robustness of the `helperBad` ground truth

The `-O0` reference is what CMake builds, but the NULL substitution is not an
`-O0` artifact. Compiling `c_src/src/driver.c` at every level and linking a
driver program that calls `bad(); driver(0); puts("[bad+driver0 done]"); good();`
gives the same answer everywhere:

| C build | `bad()` output | `driver(0)` output | `good()` output |
|---|---|---|---|
| `-O0` | *(nothing)* | *(nothing)* | `helperGood1 string` |
| `-O1` | *(nothing)* | *(nothing)* | `helperGood1 string` |
| `-O2` | *(nothing)* | *(nothing)* | `helperGood1 string` |
| `-O3` | *(nothing)* | *(nothing)* | `helperGood1 string` |
| `-Os` | *(nothing)* | *(nothing)* | `helperGood1 string` |

So `helperBad() -> ptr::null_mut()` matches the C library regardless of how the
C is compiled. (These extra builds were produced in `$TMPDIR`; nothing under
`c_src/` was modified.)

## Internal call linkage parity

Both libraries route their *internal* calls through the dynamic linker, so
interposition semantics match:

```
C     bad:    call 1060 <printLine@plt>
Rust  bad:    call *0x3bd3c(%rip)      # GOT slot for printLine
Rust  driver: call *0x3bd18(%rip)      # GOT slots for bad / good
```

Because of this, the harness dlopen()s both objects with `RTLD_LOCAL`; otherwise
one library's `good()` could call the *other* library's `printLine` and every
differential test would compare an implementation against itself.
`tests/symbol_parity.rs::d5_the_two_libraries_resolve_independently` asserts the
isolation on every run, and it was confirmed experimentally: prefixing only the
Rust `printLine` output yields

```
DIVERGENCE [C12 good()]
  C   (19 bytes): helperGood1 string\n
  Rust(30 bytes): RUSTPREFIX\nhelperGood1 string\n
```

i.e. the C library is completely unaffected by the Rust object.

## Negative controls (the tests are not vacuous)

Each mutation was applied to `src/driver.rs`, rebuilt, and the suite re-run;
`src/driver.rs` was then restored byte-identically (`cmp` clean).

| # | mutation of the Rust translation | detected? | first failing rows |
|---|---|---|---|
| M1 | `driver`: `useGood != 0` → `useGood == 1` | yes | `err_g6`, `err_g7` (and `C20`/`C21`) |
| M2 | `helperBad` returns a real string instead of NULL | yes | `err_e2`, `err_e3`, `err_g6` |
| M3 | `helperGood1` string off by one character | yes | `err_g6`, `err_g7` |
| M4 | `printLine` skips empty strings | yes | `err_e4` |
| M5 | `printLine` truncates the payload to its first byte | yes | `err_g1`…`err_g5` |
| M6 | edit `src/` **without** rebuilding the `.so` | yes | `STALE ARTIFACT` guard |

M6 matters: `crate-type = ["cdylib"]` means integration tests never link the
library, so **`cargo test` alone does not rebuild `libdriver.so`** (verified:
`touch src/driver.rs && cargo test --no-run` leaves
`target/debug/deps/libdriver.so` untouched). Before the freshness guard existed,
mutation M1 was silently *missed* because the stale `.so` was loaded. Always run
`./verify.sh`, or `cargo build && RUST_TEST_THREADS=1 cargo test`.
