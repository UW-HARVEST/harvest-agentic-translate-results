# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D` on both shared objects.

- C  `.so`: `c_src/build/libharvest-work-oJoEva.so` (built via CMake, no explicit
  `CMAKE_BUILD_TYPE` → default/unoptimized, `-lm`)
- Rust `.so`: `translation/target/release/libcontrast_ratio_lib.so`
  (`crate-type = ["cdylib"]`)

## C source inventory (completeness check)

The whole C library is one translation unit and one header. Every function
defined in the C source is accounted for below, so no module was skipped by the
translation step:

| C source | function | linkage | present in Rust? |
|---|---|---|---|
| `src/lib.c:5`  | `cbLuminance`      | `static` (internal) | yes — private `cbLuminance` |
| `src/lib.c:13` | `cbContrastRatio`  | `static` (internal) | yes — private `cbContrastRatio` |
| `src/lib.c:25` | `contrast_ratio`   | **external**        | yes — `#[no_mangle] extern "C"` |

`cbLuminance` and `cbContrastRatio` are `static`, therefore deliberately absent
from the dynamic symbol table of *both* libraries. They are exercised
transitively through `contrast_ratio`.

The header `include/lib.h` declares exactly one function and one type
(`cb_rgb_255`); types emit no symbols.

## Exported (defined) dynamic symbols

`nm -D --defined-only <so>`:

| symbol | C `.so` | Rust `.so` | status |
|---|---|---|---|
| `contrast_ratio` | `T` | `T` | **MATCH** |

Symbol diff (C exports minus Rust exports): **EMPTY** ✅
Symbol diff (Rust exports minus C exports): **EMPTY** ✅

No stubs, no `unimplemented!()`: `contrast_ratio` is a real translation of the C
body.

## Undefined (imported) symbols

`nm -D -u <so>`:

C `.so` imports 1 non-weak symbol:

- `pow@GLIBC_2.29`  ← from `libm`

Rust `.so` imports the same `pow@GLIBC_2.29` (the translation deliberately binds
`extern "C" { fn pow(x: f64, y: f64) -> f64; }` so the identical libm routine is
used and results are bit-identical), plus the ordinary Rust `std` runtime
imports, all of which are libc / libgcc-unwind:

`_Unwind_*` (libgcc), `__errno_location`, `__tls_get_addr`, `abort`, `bcmp`,
`calloc`, `close`, `dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`,
`lseek64`, `malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`,
`open64`, `posix_memalign`, `pthread_key_*`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `strlen`, `syscall`, `write`,
`writev`, and the weak `_ITM_*` / `__cxa_finalize` / `__cxa_thread_atexit_impl`
/ `__gmon_start__` / `gettid` / `statx`.

**Missing / undefined non-libc symbols in the Rust `.so`: 0** ✅

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default one. `--no-default-features` and the default build
are therefore the same code path (verified in Phase D by script).

## Completion gate (Phase D)

- [x] `SYMBOLS.md`: `nm -D` shows **0** missing symbols and **0** undefined
      non-libc symbols in the Rust `.so`. The symbol diff in both directions is
      empty. Asserted automatically by
      `phase_a_symbols::exported_symbols_match_exactly` and
      `phase_a_symbols::rust_so_has_no_unresolved_non_libc_symbols`, so the
      claim is re-checked on every test run rather than being a one-off
      observation.
- [x] Phase B: all **80** `CONFIGS.md` rows pass across randomized inputs,
      including two exhaustive sweeps of all 2^24 colors.
- [x] Phase C: all **8** `ERRORS.md` rows have a passing differential test, plus
      the generic-boundary test that structurally re-verifies the "no pointer /
      no length / no enum" claim against `c_src`.
- [x] All of the above hold under **every** build configuration: default and
      `--no-default-features`, each in debug and release (4 combinations,
      26 tests each). Driven by `./verify_all.sh`.

### Note on translation completeness

No C source was missing: `c_src` is a single translation unit whose only external
symbol is `contrast_ratio`, and it was already translated. Nothing was stubbed —
mutation testing (see below) confirms the Rust body is really doing the
computation and is really being compared against C.

### Mutation testing (proof the suite has detection power)

To confirm the differential tests are not vacuous, two deliberate bugs were
temporarily injected into `src/lib.rs` and the suite re-run; both were caught by
**10** independent tests, then the original code was restored and re-verified:

| mutation | description | detected |
|---|---|---|
| M1 | luminance dot product accumulated in `f64` then narrowed, instead of `f32` throughout | 10 tests FAILED ✅ |
| M2 | `pow` argument `(x+0.055)/1.055` computed in `f32` instead of promoting to `double` (a single extra rounding) | 10 tests FAILED ✅ |

M2 in particular is a sub-ULP-scale change, so the suite is sensitive to exactly
the class of precision slip that C-to-Rust float translation is prone to.
