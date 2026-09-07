# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared libraries.

## C `.so` (`c_src/build/libdriver.so`)

Global defined symbols, excluding weak/libc-provided linker artifacts
(`_init`, `_fini`, `__bss_start`, `_edata`, `_end`, and `w`-class weak refs):

| # | symbol | type | source |
|---|--------|------|--------|
| 1 | `decode_base64` | `T` (text, global) | `c_src/src/lib.c` (declared in `c_src/include/lib.h`) |

`static` helpers in the C source are **not** exported and therefore are not
part of the ABI surface:

| helper | C linkage | exported? |
|--------|-----------|-----------|
| `decode(char c)` | `static unsigned char` | no |
| `is_base64(char c)` | `static int` | no |

## Rust `.so` (`translation/target/release/libdriver.so`)

| # | symbol | type | Rust item |
|---|--------|------|-----------|
| 1 | `decode_base64` | `T` (text, global) | `#[unsafe(no_mangle)] pub unsafe extern "C" fn decode_base64` |

## Diff

```
$ comm -3 <(c_syms) <(rust_syms)
<empty>
```

* Symbols in C but **missing** from Rust: **0**
* Undefined non-libc symbols in the Rust `.so`: **0**. Every undefined import
  resolves to glibc or libgcc: the four the translation itself calls
  (`calloc`, `malloc`, `free`, `strlen` — the same four the C `.so` imports)
  plus the Rust std runtime's own glibc/`_Unwind_*` imports
  (`memcpy`, `mmap64`, `pthread_key_create`, `abort`, …). Verified with
  `nm -D --undefined-only target/release/libdriver.so`.

**Status: PARITY REACHED — 0 missing symbols.**

## Detection-power evidence (why "symbols match" is not the whole story)

Two findings from validating the harness itself:

1. **`cargo test` does NOT rebuild a `crate-type = ["cdylib"]` artifact.** The
   test binaries never link it, so a plain `cargo test` silently validates a
   *stale* `.so`. An injected bug initially "passed" 35/35 for this reason.
   Fixed two ways: `verify.sh` always runs `cargo build` before `cargo test`,
   and `tests/differential.rs::assert_so_fresh` refuses to run if `src/lib.rs`
   is newer than the `.so`.
2. **Mutation sweep** (`mutation_check.sh`): 17 plausible translation bugs
   injected into `src/lib.rs`; **15 caught** (6–31 failing tests each). The 2
   survivors — `b2 & 0xf → 0x1f` and `b3 & 0x3 → 0x7` — are *semantically
   equivalent mutants*, not blind spots: Rust's `<<` on `u8` discards bits
   shifted past bit 7 without any overflow check, so the extra mask bits land at
   bit ≥ 8 and vanish. Confirmed equivalent under both `release` and `debug`.

### Width-safety proof (C `int` promotion vs Rust `u8` arithmetic)

The C computes the output bytes in promoted `int` and truncates on store to
`unsigned char *p`; the Rust computes them directly in `u8`. These agree for all
inputs because `decode()` returns at most 63:

| expression | C max (int) | fits in `u8`? |
|------------|-------------|---------------|
| `(b1 << 2) \| (b2 >> 4)` | `63<<2 \| 3` = 255 | yes |
| `((b2 & 0xf) << 4) \| (b3 >> 2)` | `240 \| 15` = 255 | yes |
| `((b3 & 0x3) << 6) \| b4` | `192 \| 63` = 255 | yes |

No intermediate exceeds 255, so no truncation ever occurs on either side — this
is why the `debug` profile (Rust arithmetic-overflow checks **on**) passes too.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table** and no optional
dependencies, therefore the only build configuration is the default one.
`--no-default-features` is equivalent to the default build. `verify.sh`
enumerates the `[features]` table mechanically (powerset loop) and runs the full
suite for every resulting combination in **both** the `release` and `debug`
profiles — 4 runs total, all passing with an empty symbol diff. The `debug`
profile is included deliberately: it turns on Rust's arithmetic-overflow checks,
which C does not have.

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(driver SHARED ...)` — there is
no `add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]` (no `[[bin]]`, no `src/main.rs`). There is therefore
no driver binary and no stdout comparison to perform.
