# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

## How this was produced

The C project (`c_src/CMakeLists.txt`) only declares `add_executable(driver ...)`,
so CMake produces **no** `.so`. To obtain a shared library the same two
translation units are compiled directly with `gcc -shared -fPIC` (nothing in
`c_src/` is modified — all output goes to `cbuild/`):

```sh
gcc -O2 -fPIC -shared -DOP=$op -DREPEAT=$rep -Ic_src/src c_src/src/mdcore.c \
    -o cbuild/libcmd_${op}_${rep}.so
```

`mdmain.c` is excluded from the `.so` because it defines `main` (it is built into
`cbuild/driver_${op}_${rep}` instead and compared end-to-end — see
`tests/driver_cli.rs`).

Rust side:

```sh
cd translation && cargo build --release --no-default-features --features $op,repeat_$rep
# -> translation/target/release/libdriver.so   (crate-type = ["cdylib"])
```

Comparison command (`cbuild/symdiff.sh`):

```sh
comm -23 <(nm -D --defined-only "$C_SO"    | awk '{print $NF}' | sort -u) \
         <(nm -D --defined-only "$RUST_SO" | awk '{print $NF}' | sort -u)
```

## Defined dynamic symbols

`nm -D --defined-only cbuild/libcmd_add_5.so` (identical symbol *set* for all 24
`(OP, REPEAT)` configurations — only the data these symbols point at changes):

| symbol | C type / nm class | source | exported by Rust `.so`? | Rust definition |
|--------|-------------------|--------|-------------------------|-----------------|
| `op_add`        | `T` (text) `int(int,int)`      | `mdcore.c:28` | yes | `src/mdcore.rs` `#[unsafe(no_mangle)] extern "C" fn op_add` |
| `op_sub`        | `T` (text) `int(int,int)`      | `mdcore.c:29` | yes | `src/mdcore.rs` `op_sub` |
| `op_mul`        | `T` (text) `int(int,int)`      | `mdcore.c:30` | yes | `src/mdcore.rs` `op_mul` |
| `helper_call`   | `T` (text) `int(int,int)`      | `mdcore.c:39` | yes | `src/mdcore.rs` `helper_call` |
| `helper_ptr`    | `T` (text) `int(int,int)`      | `mdcore.c:47` | yes | `src/mdcore.rs` `helper_ptr` |
| `use_generated` | `T` (text) `int(int)`          | `mdcore.c:54` | yes | `src/mdcore.rs` `use_generated` |
| `G_OP`          | `D` (`.data`, 8 B OBJECT) `int(*)(int,int)` | `mdcore.c:36` | yes | `src/mdcore.rs` `static mut G_OP` |
| `G_OP_NAME`     | `D` (`.data`, 8 B OBJECT) `const char*`     | `mdcore.c:37` | yes | `src/mdcore.rs` `static mut G_OP_NAME` |

**Symbol diff (C \ Rust): EMPTY — 0 missing symbols.** Verified for all 24
`(OP, REPEAT)` configurations by `cbuild/symdiff.sh`.

### Deliberately NOT exported (and correctly absent from Rust too)

| C entity | why not a dynamic symbol |
|----------|--------------------------|
| `accum_add` / `accum_sub` / `accum_mul` | `DEFINE_ACCUM` (`mdmacros.h:96`) declares it `static`, so it has internal linkage. Reachable only through `use_generated`. Rust models it as the private `mdmacros::accum`. |
| `main` | lives in `mdmain.c`, which is not part of the `.so`. |
| every macro in `mdmacros.h` (`STR`, `CAT`, `OP_FN`, `STEP_*`, `INIT_*`, `REP0..REP7`, `CHOOSE_REP`, `FOR_EACH`, `DO_LOOP`, `RUN_LOOP`, `DISPATCH_REP`, `DEFINE_ACCUM`, `ACCUM_FN`) | preprocessor-only; they never produce a linker symbol. Translated to `cfg`-selected consts/fns in `src/mdmacros.rs`. |

### Undefined (imported) symbols

C `.so` imports `printf@GLIBC_2.2.5` plus the usual weak CRT hooks
(`_ITM_deregisterTMCloneTable`, `_ITM_registerTMCloneTable`, `__cxa_finalize`,
`__gmon_start__`). The Rust `.so` statically links Rust `std`, so it imports a
different (larger) set of libc symbols. **Undefined-symbol sets are not required
to match** — only the *defined/exported* set is. There are 0 undefined non-libc
symbols in the Rust `.so` (checked with `ldd -r`).

## Section-placement note (a real divergence that was fixed)

`mdcore.c` declares `int (*G_OP)(int,int)` and `const char *G_OP_NAME` — neither
pointer *object* is `const`, so gcc puts both in writable `.data`, and an
external caller can legitimately do:

```c
int (**gop)(int,int) = dlsym(h, "G_OP");
*gop = dlsym(h, "op_sub");     /* works against the C .so */
```

The translation originally used immutable Rust `static`s. LLVM marks those
`constant`, so they landed in `.data.rel.ro`, which RELRO maps **read-only**
after relocation — the store above would have segfaulted. Changed to
`static mut` so both symbols land in `.data` (`WA`), matching C. Verified with
`readelf -SW` / `readelf -sW`: `G_OP` and `G_OP_NAME` are now 8-byte `OBJECT
GLOBAL` symbols in the `.data` section in both libraries. Exercised by
`tests/differential.rs::gop_is_writable_like_c`.

## Verification evidence

`cbuild/symdiff.sh`, run for both `--release` and debug profiles:

```
ok      OP=add REPEAT=0  (8 C symbols, 0 missing, 0 undefined)
...
ok      OP=mul REPEAT=7  (8 C symbols, 0 missing, 0 undefined)
```

24/24 configurations: **0 missing symbols, 0 undefined non-libc symbols**
(`ldd -r` on the Rust `.so`). No symbol is stubbed or `unimplemented!()` — every
one is a real translation of the corresponding `mdcore.c` definition, and every
one is exercised by the differential tests in `tests/`.
