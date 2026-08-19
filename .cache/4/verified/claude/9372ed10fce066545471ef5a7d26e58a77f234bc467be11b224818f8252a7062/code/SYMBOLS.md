# SYMBOLS.md — Symbol surface parity (Phase A / Phase D)

Derived mechanically from `nm -D` on both shared objects.

Build commands used:

```sh
# C
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libdriver.so

# Rust
cargo build            # -> target/debug/libdriver.so   (crate-type = ["cdylib"])
```

## Build-time configuration surface

* `Cargo.toml` has **no `[features]` section** at all. Therefore the complete
  set of valid feature combinations is exactly one: the empty set.
  `cargo check --no-default-features` == `cargo check` == the only combination.
* `c_src/CMakeLists.txt` defines a single target (`driver`, SHARED) from a
  single translation unit (`src/driver.c`). It sets **no** `target_compile_definitions`,
  no `option()`, and no conditional sources. There is exactly one C configuration.
* `c_src/src/driver.c` + `c_src/include/driver.h` contain **no** `#if` / `#ifdef`
  other than the `DRIVER_H_` include guard, so there is no preprocessor-selected
  code either.

* `Cargo.toml` *does* however define a `[profile.release]` (`panic = "abort"`).
  The cargo **profile** is therefore a real build-time configuration axis, and it
  turned out to be a load-bearing one: an unsound ABI assumption in the
  translation produced identical output under `debug` and **divergent** output
  under `release` (see "Findings" below). Both profiles are verified.

=> **1 feature combination × 2 cargo profiles = 2 build configurations.**
Phases B–C are run for both, driven by `./run_diff_tests.sh`. The feature
dimension is still invoked explicitly as `--no-default-features` to prove the
empty feature set is the whole set.

## Exported (defined, dynamic) symbols

`nm -D --defined-only` on the C `.so`, ignoring weak linker/ITM/glibc
housekeeping symbols (`_ITM_*`, `__cxa_finalize`, `__gmon_start__`), which are
emitted by the toolchain and are not part of the library API:

| # | C symbol | C type | Rust `.so` exports it? | Rust definition |
|---|----------|--------|------------------------|-----------------|
| 1 | `driver`           | `T` (global text) | YES | `src/lib.rs` — `#[unsafe(no_mangle)] pub unsafe extern "C" fn driver(data: c_int)` |
| 2 | `printHexCharLine` | `T` (global text) | YES | `src/lib.rs` — `#[unsafe(no_mangle)] #[allow(non_snake_case)] #[inline(never)] pub unsafe extern "C" fn printHexCharLine(charHex: c_int)` |

The Rust parameters are `c_int` rather than `c_char` **deliberately**: see
Finding 1 below. The two spellings are passed identically by the x86-64 SysV ABI
(one register), and the wrappers immediately mask to the low 8 bits, so the
behaviour is bit-for-bit identical to the C `char` prototype for every
conforming caller — and, unlike `c_char`, it stays identical under optimisation.

`printHexCharLine` is **not** declared in `include/driver.h` (only `driver` is),
but it has external linkage in `driver.c` (no `static`), so it *is* an exported
symbol of the C `.so` and must be exported by the Rust `.so` too. It is, and it
is covered by the differential tests as a first-class low-level entry point.

### Symbol diff

```
comm -23 <(c_exported_sorted) <(rust_exported_sorted)   # C-only symbols
```

Result: **empty**. 0 symbols missing from the Rust `.so`.

No symbol required translating additional C source: `src/driver.c` is the only
C translation unit and both of its functions were already translated.
No stubs / `unimplemented!()` were introduced.

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libdriver.so` lists only:

* glibc imports: `printf`, `malloc`, `calloc`, `realloc`, `free`,
  `posix_memalign`, `memcpy`, `memmove`, `memset`, `bcmp`, `strlen`,
  `read`, `write`, `writev`, `close`, `open64`, `lseek64`, `stat64`,
  `fstat64`, `statx`, `mmap64`, `munmap`, `getcwd`, `getenv`, `readlink`,
  `realpath`, `abort`, `syscall`, `__errno_location`, `__tls_get_addr`,
  `dl_iterate_phdr`, `gettid`, `pthread_key_create`, `pthread_key_delete`,
  `pthread_setspecific`, `__cxa_thread_atexit_impl`
* libgcc unwinder imports: `_Unwind_*`
* weak toolchain symbols: `_ITM_*`, `__gmon_start__`, `__cxa_finalize`

**0 missing/undefined non-libc symbols.** The only non-libc-family group is the
`_Unwind_*` family from `libgcc_s`, which is the standard Rust panic-unwinding
runtime and is satisfied at load time (verified: the test suite `dlopen`s the
Rust `.so` successfully, which would fail on any unresolved symbol because
`ldd -r`/lazy-binding resolution of `printf` and the unwinder must succeed).

Verified with `ldd -r` reporting no "undefined symbol" lines for either `.so`.

## Note on symbol interposition (matters for the differential harness)

The C `driver` calls `printHexCharLine` **through the PLT**, and the Rust
`driver` calls it **through the GOT**; in both `.so`s the callee has default
(exported, preemptible) visibility. If both libraries were loaded with
`RTLD_GLOBAL`, one library's `printHexCharLine` could interpose on the other's
`driver`, silently invalidating the comparison.

The tests therefore load each `.so` with `RTLD_LOCAL | RTLD_NOW`
(`libloading::os::unix::Library::open` with explicit flags) so that each
library's internal call binds to its own definition. See
`tests/common/mod.rs::load`.

## Findings (Phase D)

### Finding 1 — `char` argument was not truncated to the low byte (FIXED)

Found by row **E9/E10** of ERRORS.md, and only visible in the **release**
profile.

Both C entry points are declared `void f(char)`. On x86-64 SysV a narrow integer
argument occupies the low 8 bits of the argument register and the upper 24 bits
are *unspecified*. gcc never trusts them: it emits an explicit
`movsbl %dil, %esi` at **every** optimisation level (verified for `-O0`, `-O1`,
`-O2`, `-O3`, `-Os`).

The original translation declared the parameter as `c_char`, which makes rustc
attach LLVM's `signext i8` attribute — a *promise* that the caller already
sign-extended. With optimisations on, LLVM folded `sext(trunc(edi))` into a bare
`edi`:

```
gcc  printHexCharLine  (every -O):  movsbl %dil,%esi     <- low byte only
rust printHexCharLine  (release):   mov    %edi,%esi     <- WHOLE register
```

Observable divergence, `printHexCharLine` called with `int 0xdeadbe00`:

| | output |
|---|---|
| C (ground truth) | `00\n` |
| Rust release (before fix) | `deadbe00\n` |

**Fix:** the exported wrappers now take a `c_int` and truncate explicitly via
`char_arg()`, reproducing gcc's `movsbl %dil` in a way the optimiser cannot
remove. This is bit-for-bit identical for every conforming caller that passes a
real `char`, and now also matches gcc for callers that pass a wider value
through the same register.

After the fix the release codegen is instruction-for-instruction equivalent to
gcc `-O2`:

```
gcc  -O2  driver:            add $0x1,%edi ; movsbl %dil,%edi ; jmp printHexCharLine@plt
rust rel. driver:            inc %dil      ; movsbl %dil,%edi ; jmp *GOT(printHexCharLine)
gcc  -O2  printHexCharLine:  movsbl %dil,%esi ; ... ; jmp printf@plt
rust rel. printHexCharLine:  movsbl %dil,%esi ; ... ; jmp printf@plt
```

`#[inline(never)]` was added to `printHexCharLine` so that `driver` performs a
real call to the exported symbol (mirroring gcc's `jmp printHexCharLine@plt`)
instead of inlining `printf` into `driver`.

The reference C build is gcc: `CMAKE_C_COMPILER=/usr/bin/cc` -> GCC 11.5.0,
`CMAKE_BUILD_TYPE=` (empty, i.e. `-O0`), `CMAKE_C_FLAGS=` (empty).

### Note — the upper argument bits are ABI-unspecified, and C compilers disagree

Running the identical suite against a **clang**-built `-O2` C library shows clang
does *not* truncate (it uses `signext`, like the unfixed Rust did), so C
compilers genuinely disagree in this ABI-unspecified region. The Rust
translation matches **gcc**, which is what this project's `CMakeLists.txt`
builds with and therefore what "the C is the ground truth" means here. The
divergence only exists for callers that violate the `char` prototype; the
`c_char` domain itself is unaffected.

### Finding 2 — `cargo test` does not relink the cdylib (test-harness hazard)

`cargo test` compiles the library for the *test* profile but does **not** rebuild
`target/<profile>/libdriver.so`. A mutated `src/lib.rs` therefore kept passing
because the tests were loading a **stale** `.so`. This is a silent vacuous pass
that hides every real divergence.

Mitigated two ways:
* `tests/common/mod.rs::assert_fresh` aborts with `STALE Rust SHARED OBJECT` if
  the `.so` is older than anything in `src/` (verified to fire);
* `run_diff_tests.sh` always runs `cargo build` before `cargo test`.

### Finding 3 — fd-1 capture is contaminated by libtest under parallel execution

The natural way to capture what a library prints is to redirect fd 1 in-process
with `dup2`. Under `cargo test`'s **default parallel** execution that is subtly
broken: libtest writes its own progress text to fd 1 from other threads, and
those writes land inside the redirect window:

```
output mismatch for C10 driver(0x80)
  C   (49 bytes): "test c12_driver_exhaustive_domain ... ffffff81\nok"
  Rust(9 bytes):  "ffffff81\n"
```

Three tests failed this way on 3 of 3 parallel runs (29-31 of 35 passing,
varying run to run) — spurious diffs that look exactly like translation bugs. A
mutex cannot fix it, because the contaminating writer is libtest's own thread.

**Fix:** every capture now runs the library call in a **forked child** whose fd 1
points at a private temp file or pipe (`tests/common/mod.rs::capture`). The
parent's fd 1 is never touched and the child is single-threaded, so results are
identical under `--test-threads=1` and under full parallelism (verified: 35/35 on
5 consecutive parallel runs plus a single-threaded run).

Captures are **batched** — all values of a row run inside one child — so the
whole suite costs ~100 forks rather than ~20000. Diagnostic precision is
retained: `assert_same_over_values` maps the first differing output line back to
the exact input that caused it, e.g. for a mutation that diverges at only one of
256 values:

```
output mismatch for C12 driver exhaustive
  diverges at output line 127 => input #127 = Some(127)
  C   line: ffffff80
  Rust line: 7f
  C   context: 7f | ffffff80 | ffffff81
  Rust context: 7f | 7f | ffffff81
```

### Suite sensitivity (mutation testing)

To prove the differential tests are not vacuous, 11 mutations were injected into
`src/lib.rs`, each rebuilt and run in both profiles. Result: **every
behaviour-changing mutation was detected in both profiles**, including ones with
a single divergent input:

| mutation | detected (debug / release) |
|---|---|
| `driver` increments by 2 | 22 / 22 tests |
| `driver` does not increment | 22 / 22 |
| `driver` saturates instead of wrapping (differs only at `0x7F`) | 16 / 16 |
| zero-extend instead of sign-extend | 22 / 22 |
| `%2x` (padding dropped) | 21 / 21 |
| `%02X` (uppercase) | 26 / 26 |
| `%02x` (newline dropped) | 29 / 29 |
| `%02hhx` (differs only for negatives) | 22 / 22 |
| `printHexCharLine` export removed | 33 / 33 |
| argument masked to 7 bits | 22 / 22 |
| the real pre-fix `c_char` signature | passes debug / **fails release** |

(One further mutation, replacing `(raw as u32 & 0xff) as u8` with `raw as u8`,
was correctly *not* detected — a Rust `as u8` cast already truncates, so that
edit is semantically identical code rather than a behaviour change.)

The last row is why both profiles are verified: the genuine defect was invisible
in `debug`.
