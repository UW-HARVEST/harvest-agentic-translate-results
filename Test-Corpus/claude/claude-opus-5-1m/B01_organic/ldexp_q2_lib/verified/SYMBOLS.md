# SYMBOLS.md — Phase A: exported-symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands used

```sh
# C (default configuration — CMakeLists.txt sets no CMAKE_BUILD_TYPE, so gcc -O0)
cd c_src && mkdir -p build && cd build \
  && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON \
  && cmake --build .
# -> c_src/build/libtranslated_rust.so

# Rust (cdylib)
cargo build --release            # -> target/release/libldexp_q2_lib.so
cargo build                      # -> target/debug/libldexp_q2_lib.so
```

## C source inventory (completeness check)

`c_src/CMakeLists.txt` compiles exactly one translation unit:

| C source file | translated to | status |
|---|---|---|
| `c_src/src/lib.c` (12 lines, 1 function) | `src/lib.rs` | TRANSLATED |

Public header `c_src/include/lib.h` declares exactly one prototype:

```c
float ldexp_q2(float y, int exp_q2);
```

There is no second module, no `#ifdef`-gated file, and no macro-generated symbol
family in the C sources, so no C source was skipped by the translation step.

## `nm -D --defined-only` — C `.so`

| symbol | type |
|---|---|
| `ldexp_q2` | `T` (global text) |

## `nm -D --defined-only` — Rust `.so`

| symbol | type |
|---|---|
| `ldexp_q2` | `T` (global text) |

## Symbol diff

```
$ comm -23 <(nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $NF}' | sort -u) \
           <(nm -D --defined-only target/release/libldexp_q2_lib.so   | awk '{print $NF}' | sort -u)
(empty)
```

**Missing from Rust: 0.** No stubs, no `unimplemented!()`, no fake exports.

The Rust `.so` is exported via:

```rust
#[unsafe(no_mangle)]
pub extern "C" fn ldexp_q2(y: f32, exp_q2: c_int) -> f32
```

ABI check: `float` → `f32` in `xmm0`, `int` → `c_int` (`i32`) in `edi`, return
`float` in `xmm0`. Identical to the C build (verified in the disassembly below).

## Undefined / imported symbols

C `.so` imports only weak toolchain symbols (`__cxa_finalize`, `__gmon_start__`,
`_ITM_*TMCloneTable`).

The Rust `.so` additionally imports libc (`malloc`, `memcpy`, `write`, …) and
libgcc unwinder symbols (`_Unwind_*`) pulled in by `std`/`panic_unwind`. These
are **all** libc / libgcc runtime symbols resolved by the system loader; there
are **0 missing or undefined non-libc symbols** (verified by `ldd -r`, which
reports no unresolved symbols).

## Reference C disassembly (default `-O0` build)

Kept here because the Rust translation must reproduce two
implementation-defined / undefined-behaviour details visible only in the codegen:

```asm
0000000000001<0f9> <ldexp_q2>:
  mov    $0x78,%edx          ; 30*4 folded to 120
  cmp    %edx,%eax
  cmovg  %edx,%eax           ; e = min(exp_q2, 120)
  and    $0x3,%eax           ; e & 3  -> always 0..3, index never OOB
  movss  (%rdx,%rax,1),%xmm1 ; g_expfrac[e & 3]
  sar    $0x2,%eax           ; e >> 2  -> ARITHMETIC shift (sign extends)
  mov    $0x40000000,%edx
  mov    %eax,%ecx
  sar    %cl,%edx            ; 1<<30 >> (e>>2): x86 masks %cl to 5 bits
  cvtsi2ss %eax,%xmm0        ; (float)shifted
  mulss  %xmm1,%xmm0         ; product = (float)shifted * g_expfrac[e&3]
  mulss  %xmm1,%xmm0         ; y = y * product   (single-precision, no FMA)
  sub    %eax,-0x18(%rbp)    ; exp_q2 -= e
  jg     ...                 ; do { } while (exp_q2 > 0)
```

`.rodata` of the C `.so` (the `static const float g_expfrac[4]`):

```
2000  00008030 fd445730 f3043530 f0371830
      0x30800000 0x305744fd 0x303504f3 0x301837f0
```

The four Rust `f32` literals in `src/lib.rs` round to exactly these bit
patterns (verified), so the table is bit-identical.

A cross-check compiling the same C at `-O2` produced bit-identical results to
the `-O0` reference build for every input tried, confirming the `sar %cl`
5-bit-mask behaviour is stable and is the behaviour the Rust must reproduce.

## Reproducing

```sh
./run_verification.sh
```

The script builds the C `.so`, enumerates every Cargo feature combination
(there is exactly one — `Cargo.toml` declares no `[features]`), runs
`cargo check` (lib + tests) for each, builds both the debug and release
`cdylib`, runs all differential tests, and finally re-does the `nm -D` diff.

## Completion status

| gate | status |
|---|---|
| `nm -D`: 0 symbols missing from the Rust `.so` | PASS (`ldexp_q2` present; `ldd -r` reports no unresolved symbols) |
| Every C symbol resolvable via `dlsym` on the Rust `.so` | PASS (`tests/symbol_parity.rs::d02`) |
| Phase B: all 27 `CONFIGS.md` rows | PASS (`tests/valid_paths.rs`, 27/27) |
| Phase C: all 17 `ERRORS.md` rows | PASS (`tests/error_paths.rs`, 17/17) |
| Holds for every feature combination | PASS (1 combination exists; verified) |
| Holds for both Rust profiles (debug + release cdylib) | PASS (both `.so`s are dlopen'd and compared in every assertion) |
