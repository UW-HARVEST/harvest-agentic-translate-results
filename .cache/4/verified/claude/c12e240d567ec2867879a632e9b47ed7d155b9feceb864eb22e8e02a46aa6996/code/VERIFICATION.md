# VERIFICATION.md — differential verification of the C→Rust translation

The C code in `c_src/` is the ground truth. Everything below compares the Rust
translation against it **through shared objects loaded with `libloading`**, i.e.
through the `#[no_mangle] extern "C"` exports, plus the two driver executables
for the `main` path.

## What is compared, and how

| layer | C side | Rust side | how |
|-------|--------|-----------|-----|
| library functions | `target/cdiff/libc_driver.so` (`cc -shared -fPIC c_src/src/main.c`) | `libstb_perlin_cli.so` (`crate-type = ["cdylib", "rlib"]`) | both `dlopen`ed by the tests, results compared with `f32::to_bits()` |
| `main` (driver) | `target/cdiff/c_driver` — the executable of `c_src/CMakeLists.txt` | `target/<profile>/driver` | same stdin, `stdout` bytes **and** exit status compared |
| `main` **as an exported symbol** | `main` in `libc_driver.so` | `main` in `libstb_perlin_cli.so` | `src/bin/so_main_runner.rs` `dlopen`s the library and calls `main` |
| `scanf`/`printf` emulation | **glibc itself** (`sscanf`, `snprintf` via FFI) | `cscan::Scanner`, `cfmt::format_g` | value, `%n` consumption and formatted text compared |

Nothing in `c_src/` was modified. The C shared object is produced next to the
cmake executable in `target/cdiff/` by `scripts/build_c_so.sh`, which the tests
run automatically (with an advisory lock, because `cargo test` runs its test
binaries in parallel).

## Reproducing

```bash
cargo test                       # 89 tests, builds every artefact it needs
cargo test --release             # same, release profile
scripts/check_features.sh check  # cargo check for every feature combination
scripts/check_features.sh test   # cargo test  for every feature combination
scripts/symbol_diff.sh [debug|release]   # nm -D parity check
python3 scripts/fuzz.py 150000 1500      # extra large randomised sweep
python3 scripts/cmp.py <fn> <args...>    # compare a single call
python3 scripts/probe.py <so> <fn> <args...>  # run one call in a child process
```

## Results

### Phase A — surface

* `SYMBOLS.md` — 9 exported symbols in the C `.so`, all 9 exported by the Rust
  `.so` with identical names; 0 undefined non-libc symbols.
* `ERRORS.md` — 44 `E` rows (every rejection/degenerate/UB path the C source
  actually contains) + 9 generic `G` rows. The library has no error codes,
  asserts, null checks or pointer parameters at all; `inner`'s
  `default: return NAN` is its only sentinel.
* `CONFIGS.md` — 53 `C` rows (the pruned cross-product of wrap options, seed
  shapes, coordinate shapes, fractal parameters, `which` cases and driver input
  shapes) + 4 build-configuration rows.

### Phase B — valid paths

All 53 `CONFIGS.md` rows pass. Every row uses a fixed-seed `splitmix64` PRNG
with hundreds to tens of thousands of inputs per row; results are compared
bit-for-bit. Notable per-row volumes:

```
C1..C18   noise3_internal / noise3 / noise3_seed   ~70 000 comparisons
C19..C29  ridge / fbm / turbulence                 ~50 000 comparisons
C30..C37  wrap_nonpow2 (in-window inputs only)     ~72 000 comparisons
C38..C40  inner                                    ~45 000 comparisons
C41..C53  main: 1 000+ process pairs, 230 float spellings x 6 slots,
          40 000 scanner tokens, 400 000+ printf values
```

### Phase C — error paths

All `ERRORS.md` rows have a passing differential test:

* E1–E5 `inner` with `which` outside `0..=5` (including `INT_MIN`, `INT_MAX` and
  20 000 random out-of-range values): both return exactly `0x7fc00000`.
* E6–E11 `octaves <= 0` in all three fractal functions: both return `+0.0`.
* E12–E16 `NaN`/`±inf`/`|x| >= 2^31` coordinates through `cvttss2si`'s
  indefinite value.
* E17–E19 wrap masks at `INT_MIN` (signed overflow), non-powers of two, `1`.
* E20–E21 `int`→`unsigned char` seed truncation.
* E22–E25, E29 the `wrap_nonpow2` corner cases that stay inside the
  deterministic `.data` window (including indices that read the *gradient*
  table through `randtab`: 22 815 such comparisons in C35 alone).
* E26, E27, E30 the genuinely undefined deep out-of-bounds reads: the test
  *demonstrates* that the C executable and the C shared object built from the
  same source disagree (e.g. `5 1.5 2.5 300.5 5 7 400 …` → exe `-0.25`, `.so`
  `-1.5498521e26`), so no implementation can match both, and shows the Rust
  library stays memory-safe.
* E28 `INT_MIN % -1`: the C code dies with `SIGFPE` (asserted via
  `ExitStatus::signal()` in a child process); Rust returns.
* E31–E43 every `scanf` rejection shape and the `-nan` printing path.

### Phase D — parity and configurations

```
$ scripts/symbol_diff.sh debug     → missing from Rust .so: (none)
$ scripts/symbol_diff.sh release   → missing from Rust .so: (none)
$ scripts/check_features.sh check  → ALL FEATURE COMBINATIONS OK
$ scripts/check_features.sh test   → 89 tests passed in each of
                                     --no-default-features / --all-features / default
$ cargo test --release             → 89 tests passed
$ python3 scripts/fuzz.py 150000 1500
    1 075 630 library calls compared, 0 divergences
    1 500 driver inputs compared,     0 divergences
```

## Divergences found and fixed (Rust side only, C untouched)

1. **NaN payload/sign propagation** (`src/stb_perlin.rs`). x86 `addss/subss/mulss`
   return the *destination* operand when it is a NaN, so which of two different
   NaN payloads survives depends on the operand order the compiler picked. gcc's
   order (read off `objdump -d` of the C library) differs from the Rust
   backend's in `stb__perlin_lerp` (`a + (b-a)*t` keeps the **product** as the
   destination) and in `stb__perlin_grad` (`g0*x + g1*y + g2*z` keeps **`g2*z`**
   as the destination of the second addition, so a NaN in `z` outranks one in
   `x`, which outranks one in `y`), as well as in the `sum += …` accumulations of
   the three fractal functions. This was user-visible: the driver printed
   `nan` where C printed `-nan`, e.g. for the input `0 nan 1 -nan 0 0 0 0 0 0 0 0`.
   Fixed by making the operand order explicit (`sse_add`/`sse_sub`/`sse_mul`,
   which reproduce the hardware's NaN selection and quieting).
2. **`scanf("%f")` hex-float acceptance** (`src/cscan.rs`). glibc collects the
   subject sequence and then calls `strtof`, rejecting only a bare `0x`.
   The translation rejected `0x.`, `0x.p1`, `0x.g`, … which glibc *accepts*
   with the value `0` (and it must not consume the `p` when no hex digit
   preceded it). Visible in the driver: `1 0x. 2.25 3.125 …` printed
   `0.111367986` in C and `0` in Rust. Fixed by re-implementing glibc's state
   machine, and the hex mantissa is now rounded once (sticky bit) instead of
   through a `f64` intermediate that could round twice.
3. **Test-infrastructure completeness** (not a behaviour bug): the crate had no
   library target at all, so *none* of the C symbols were exported. Added
   `[lib] crate-type = ["cdylib", "rlib"]` with one `#[no_mangle] extern "C"`
   wrapper per exported C function (including `inner` and `main`), and moved the
   driver body into `src/driver.rs` so the binary and the library share it.

## Known, deliberate non-equivalence

`stb_perlin_noise3_wrap_nonpow2` indexes `stb__perlin_randtab` out of bounds
whenever a wrap argument leaves `1..=256` (`0` meaning `256`). Inside the
1024-byte window that both C builds lay out identically (`randtab` followed by
`randtab_grad_idx`) the Rust translation reproduces C exactly. Beyond it the C
program reads bytes that are not part of its data (relocated `.got.plt` entries
in front of `.data`, ELF section headers or unmapped pages behind it), so its
result differs between the executable and the shared object built from the same
source and can be a `SIGSEGV`; `INT_MIN % -1` even traps with `SIGFPE`. The Rust
translation reads `0` outside the window and never traps. `ERRORS.md` rows E26,
E27, E28 and E30 document this and the corresponding tests pin down the
classification (C builds disagree / C traps, Rust stays safe) instead of a value.

## Completion gate

- [x] `SYMBOLS.md`: `nm -D` shows 0 missing symbols and 0 undefined non-libc
      symbols in the Rust `.so` (checked for the debug **and** release profile).
- [x] Phase B: every row of `CONFIGS.md` passes across randomised inputs.
- [x] Phase C: every row of `ERRORS.md` has a passing error-path differential
      test.
- [x] Every feature combination (`--no-default-features`, `--all-features`,
      default — the crate declares no `[features]`, so these are the complete
      cross-product) plus the release profile pass all 89 tests.
