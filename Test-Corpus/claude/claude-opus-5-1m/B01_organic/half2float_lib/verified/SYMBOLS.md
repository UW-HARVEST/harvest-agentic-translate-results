# SYMBOLS.md — Exported-symbol parity (Phase A / Phase D)

## Source inventory

The C build (`c_src/CMakeLists.txt`) globs exactly one translation unit into the
shared library:

```
add_library(${project_name} SHARED src/lib.c)
```

| C file | lines | translated to Rust? |
|--------|-------|---------------------|
| `c_src/src/lib.c`     | 376 | yes → `src/lib.rs` |
| `c_src/include/lib.h` | 3   | yes (only declaration: `float half2float(uint16_t h);`) |

No other `.c` / `.h` files exist, so no module was skipped.

## Public header surface

`c_src/include/lib.h` declares exactly one function:

```c
float half2float(uint16_t h);
```

There are no namespace-renaming macros (`#define foo NAMESPACE(foo)`), so the
source-level name **is** the final linker symbol.

## `nm -D` comparison

C `.so`: `c_src/build/libtranslated_rust.so`
Rust `.so`: `target/release/libhalf2float_lib.so`

### C — defined dynamic symbols (`nm -D --defined-only`)

| addr | type | symbol |
|------|------|--------|
| `00000000000010f9` | `T` | `half2float` |

### C — weak/undefined entries (`nm -D`, toolchain-generated, NOT part of the API)

| type | symbol | note |
|------|--------|------|
| `w` | `_ITM_deregisterTMCloneTable` | libc/compiler-runtime weak stub |
| `w` | `_ITM_registerTMCloneTable`   | libc/compiler-runtime weak stub |
| `w` | `__cxa_finalize@GLIBC_2.2.5`  | libc |
| `w` | `__gmon_start__`              | libc |

### Rust — defined dynamic symbols (`nm -D --defined-only`)

| addr | type | symbol |
|------|------|--------|
| `0000000000013dd0` | `T` | `half2float` |

### Symbol diff

```
comm -23 <C defined non-libc symbols> <Rust defined non-libc symbols>   ->   (empty)
```

| C symbol | present in Rust `.so`? | action |
|----------|------------------------|--------|
| `half2float` | **yes** (`T half2float`) | none needed |

**Missing symbols: 0. Undefined non-libc symbols in the Rust `.so`: 0.**

Nothing was stubbed or faked: `half2float` in `src/lib.rs` is a full
translation of the C body plus all three lookup tables.

## Static (private) C data — must be reproduced, not exported

These are `static` in C, so they are intentionally **not** in `nm -D`. They are
still part of the observable behaviour and were verified element-by-element
against the C source (see `tests/table_parity.rs` / the mechanical diff in
Phase A):

| C object | size | Rust counterpart | element-wise equal |
|----------|------|------------------|--------------------|
| `static uint32_t m__mantissa[2048]` | 2048 × u32 | `static M__MANTISSA: [u32; 2048]` | yes (2048/2048) |
| `static uint16_t m__offset[64]`     | 64 × u16   | `static M__OFFSET: [u16; 64]`      | yes (64/64) |
| `static uint32_t m__exponent[64]`   | 64 × u32   | `static M__EXPONENT: [u32; 64]`    | yes (64/64) |

## Build-time configuration surface

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` has no
`option()` / `add_definitions()` / `#ifdef`-driven variants. Therefore the
complete set of valid feature combinations is a single one:

| # | feature combination | command |
|---|---------------------|---------|
| 1 | *(none / default — the empty set)* | `cargo check --no-default-features` / `cargo test --no-default-features` |

`grep -n features Cargo.toml` → no match. `grep -c '#if' c_src/src/lib.c` → 0.

`verify_all_features.sh` enumerates the `[features]` power set mechanically and
runs `cargo check`, the `nm -D` diff and the whole differential suite for each
combination; with no features declared that is the single default combination.

## Divergence found and fixed (parameter-truncation ABI)

The one real behavioural divergence the differential tests uncovered was **not**
in the tables or the arithmetic but in the *export wrapper's* parameter type.

The C callee truncates its `uint16_t` parameter before using it:

```asm
half2float:
    mov    %edi,%eax
    mov    %ax,-0x14(%rbp)      ; only the low 16 bits are kept
    movzwl -0x14(%rbp),%eax
    shr    $0xa,%ax             ; => n is always 0..63
```

The Rust export was declared `extern "C" fn(h: u16)`, which lets the optimiser
assume the upper register bits are already clear. In the `release` build LLVM
then used the **full `%edi`**:

```asm
    mov    %edi,%ecx
    shr    $0xa,%ecx            ; n up to 0x3FFFFF -- and the M__OFFSET
    movzwl (%rdx,%rcx,2),%edi   ; bounds check was optimised away
```

so a caller passing a wider value (legal in C — any `int` converts to the
`uint16_t` parameter) made the Rust build read `M__OFFSET` out of bounds and
abort (`index out of bounds: the len is 2048 but the index is 29299`), where the
C simply returns the value for `h & 0xFFFF`. Rows E7/E8 of `ERRORS.md` catch it.

Fix (in `src/lib.rs`): the exported wrapper takes `c_uint` and truncates with
`as u16`, reproducing the C prologue exactly; the translated body moved into
`half2float_impl(h: u16)`. The machine-level signature is unchanged (a
`uint16_t` argument is passed in the same 32-bit register), so ordinary callers
are unaffected. The release build now emits `and $0x3f,%ecx`, clamping `n` to
`0..63` just as the C does.

## Verification record

| check | result |
|-------|--------|
| `nm -D` defined-symbol sets, C vs Rust (`release`) | **identical** (`diff` empty) |
| `nm -D` defined-symbol sets, C vs Rust (`debug`)   | **identical** |
| Missing symbols | **0** |
| `ldd -r` unresolved symbols in C / Rust `.so` | **0** (test `d2`) |
| Lookup-table elements verified through the `.so` | 2048 + 64 + 64 = **2176 / 2176** (test `d3`) |
| Phase B rows (`CONFIGS.md` C1–C20) | **20 / 20 passing** |
| Phase C rows (`ERRORS.md` E1–E10) | **10 / 10 passing**, plus generic boundaries |
| Exhaustive differential coverage | all **65 536** `uint16_t` inputs, bit-identical |
| Rust build profiles compared against C | **both** `debug` (overflow + bounds checks on) and `release` (`panic = "abort"`) |
| Feature combinations verified | **1 / 1** (the only valid one) |
| Compiler warnings (`cargo check --all-targets`) | **0** |

### Harness self-validation (mutation testing)

The differential suite was checked for vacuity by injecting deliberate faults
into `src/lib.rs` and confirming each is caught (then restoring the file
byte-identically):

| injected mutation | tests failed |
|-------------------|--------------|
| `m__offset[n]` replaced by the constant `0x400` | 8 |
| single `m__mantissa` element off by one (`0x387fc000` → `0x387fc001`) | 5 |
| `m__exponent[31]` given its "linear" value `0x0f800000` instead of `0x47800000` | 9 |
| `m__exponent[32]` sign bit dropped (`0x80000000` → `0`) | 7 |
| `h >> 10` changed to `(h >> 9) & 0x3f` | 10 |
| export wrapper reverted to `fn(h: u16)` (no truncation) | aborts in `error_paths` (E7) |

6 / 6 mutants killed.
