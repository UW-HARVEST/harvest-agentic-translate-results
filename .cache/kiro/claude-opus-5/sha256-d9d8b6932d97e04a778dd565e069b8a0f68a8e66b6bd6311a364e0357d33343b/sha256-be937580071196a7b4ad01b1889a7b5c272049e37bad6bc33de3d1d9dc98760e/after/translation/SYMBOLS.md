# SYMBOLS.md — dynamic-symbol parity, C `.so` vs Rust `.so`

## How this table was produced

The C build (`CMakeLists.txt`) only declares `add_executable(driver ...)`, so
there is no `.so` target to reuse. `build_c.sh` compiles the *same* translation
unit that contributes every non-`main` symbol — `src/mdcore.c` — with the same
`CMAKE_C_FLAGS` (`-DOP=<op> -DREPEAT=<n>`) plus `-fPIC -shared`:

```
./build_c.sh add 5      # -> cbuild/add_5/libmdcore.so, cbuild/add_5/driver
```

Rust side:

```
cd translation && cargo build --release --no-default-features --features add,5
                  # -> translation/target/release/libdriver.so
```

Both lists come from `nm -D --defined-only`. `symbol_parity.sh` regenerates and
diffs them for every feature combination.

## Defined (exported) symbols

| # | symbol | kind | C `.so` | Rust `.so` | source |
|---|--------|------|---------|-----------|--------|
| 1 | `op_add`        | `T` text | yes | yes | `mdcore.c:28` |
| 2 | `op_sub`        | `T` text | yes | yes | `mdcore.c:29` |
| 3 | `op_mul`        | `T` text | yes | yes | `mdcore.c:30` |
| 4 | `G_OP`          | `D` data | yes | yes | `mdcore.c:36` — `int (*)(int,int)` |
| 5 | `G_OP_NAME`     | `D` data | yes | yes | `mdcore.c:37` — `const char *` |
| 6 | `helper_call`   | `T` text | yes | yes | `mdcore.c:39` |
| 7 | `helper_ptr`    | `T` text | yes | yes | `mdcore.c:47` |
| 8 | `use_generated` | `T` text | yes | yes | `mdcore.c:54` |

Diff (C − Rust): **empty**. Diff (Rust − C): **empty**.

### Deliberately *not* exported (must stay private in both)

| symbol | why |
|--------|-----|
| `accum_add` / `accum_sub` / `accum_mul` | `DEFINE_ACCUM` (`mdmacros.h:95`) emits `static int CAT(accum_, op)(int n)`. `static` ⇒ internal linkage, absent from `nm -D` in C. The Rust counterpart `mdcore::accum` is a private `fn` with no `#[no_mangle]`, so it is likewise absent. Confirmed: `nm -D` on the C `.so` shows no `accum_*`. |
| `main` | lives in `mdmain.c`, which is only linked into the `driver` executable, not the shared library. |
| everything in `mdmacros.h` (`STEP_*`, `REP0..REP7`, `DISPATCH_REP`, `FOR_EACH`, `DO_LOOP`, `STR`, `CAT`, `INIT_*`, `OP_FN`, `ACCUM_FN`) | preprocessor-only; no object-file presence at all. The Rust translation keeps the equivalents (`mdmacros::step`, `rep`, `run_loop`, `dispatch_rep`, `do_loop`, `INIT`, `OP_FN`, `REPEAT`, `OP_NAME_C`) as ordinary module items with mangled names, matching C's zero exported symbols for these. |

### Undefined (imported) symbols

| C `.so` | note |
|---------|------|
| `printf@GLIBC_2.2.5` (`U`) | from `mdcore.c`'s three `printf` calls. |
| `__cxa_finalize`, `__gmon_start__`, `_ITM_registerTMCloneTable`, `_ITM_deregisterTMCloneTable` (`w`) | weak toolchain/CRT hooks, not part of the API. |

The Rust `.so` imports its own (larger) set of libc symbols because it links the
Rust standard library for the `format!`/`std::io` output path. Per the completion
gate only *non-libc* undefined symbols matter. `symbol_parity.sh` checks this
mechanically rather than with a hand-written allowlist: it collects the defined
symbols of `libc.so.6`, `libgcc_s.so.1` and `ld-linux-x86-64.so.2` and asserts
that every *strong* undefined symbol of the Rust `.so` is present in that set.
Result: **0** unresolved non-platform symbols for all 24 configurations.
`ldd` confirms the only shared-library dependencies are `libgcc_s.so.1`,
`libc.so.6` and `ld-linux-x86-64.so.2`.

## Section placement of the two data exports

`nm -D` type letters agree (`D` on both sides), but that is not sufficient on its
own — the *writability* of the section matters for a caller that stores to the
symbol. `readelf -SW` / `readelf -sW`:

| symbol | C `.so` section | Rust `.so` section |
|--------|-----------------|--------------------|
| `G_OP`      | `.data` (`WA`) | `.data` (`WA`) |
| `G_OP_NAME` | `.data` (`WA`) | `.data` (`WA`) |

The `const` in `const char *G_OP_NAME` qualifies the *pointee*, so the global
itself is mutable and gcc places it in `.data`. Declaring the Rust counterpart as
an immutable `static` put it in `.data.rel.ro` instead, which the loader turns
read-only under RELRO; a caller's store then faulted where C succeeded. Both
globals are therefore `static mut` in `mdcore.rs`. `ERRORS.md` row 21
(`err_row21_data_exports_are_writable`) pins this — with the immutable `static`
the test process dies with SIGSEGV.

## Verification status

- [x] every symbol in the C `.so` is exported by the Rust `.so`, exact spelling
- [x] no extra non-libc exports on the Rust side
- [x] both data exports land in the same, writable, ELF section as in C
- [x] holds for all 26 feature combinations (see `symbol_parity.sh` output)
- [x] Rust `.so` has 0 unresolved non-platform undefined symbols
