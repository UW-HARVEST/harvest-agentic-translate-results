# ERRORS.md — error / rejection surface of the C library (Phase C)

Mechanically derived from `c_src/src/stb_perlin.h` + `c_src/src/main.c`.

Grep summary of the whole C source:

```
$ grep -nE "return|assert|if *\(|\?|%|&" c_src/src/*.c c_src/src/*.h   # (condensed)
main.c:36        default: return NAN;                  <- the ONLY sentinel/error return
main.c:56        scanf("%d%f%f%f%d%d%d%d%f%f%f%d", …)  <- return value ignored; matching failures
stb_perlin.h:191 int ai = (int) a; return (a < ai) ? ai-1 : ai;
stb_perlin.h:220 float *grad = basis[grad_idx];        <- unchecked index
stb_perlin.h:239 unsigned int x_mask = (x_wrap-1) & 255;
stb_perlin.h:286 for (i = 0; i < octaves; i++)         <- ridge
stb_perlin.h:305 for (i = 0; i < octaves; i++)         <- fbm
stb_perlin.h:320 for (i = 0; i < octaves; i++)         <- turbulence
stb_perlin.h:363 int x_wrap2 = (x_wrap ? x_wrap : 256);
stb_perlin.h:369 if (x0 < 0) x0 += x_wrap2;            (x3: x, y, z)
stb_perlin.h:366 int x0 = px % x_wrap2;                (x3) + x1 = (x0+1) % x_wrap2
stb_perlin.h:377 r0 = stb__perlin_randtab[x0];         <- unchecked index (x6 reads)
```

There are **no** `assert`s, no `NULL` checks, no pointer parameters, no error
enums, no `errno` use and no `-1`/`NULL` returns anywhere in the library: the
only explicit rejection is `inner`'s `default: return NAN`. Everything else
below is a *degenerate-input* or *undefined-behaviour* path that the C code
takes silently — each row states the exact result the compiled C produces
(measured with `scripts/probe.py` against `target/cdiff/libc_driver.so`).

Legend for the last column: `=` the Rust `.so` must return the **same bits**;
`UB` the C behaviour is not reproducible by any conforming implementation
(build-layout-dependent garbage or a hardware trap) — the test then pins down
the *classification* instead of the value (see notes at the bottom).

| # | function | trigger (the exact invalid input/condition) | expected C result | class | test |
|---|----------|---------------------------------------------|-------------------|-------|------|
| E1 | `inner` | `which = -1` (below the switch range) | `NAN` = `0x7fc00000` | `=` | `phase_c_errors::e1_e4_inner_which_out_of_range` |
| E2 | `inner` | `which = 6` (one past the last case) | `NAN` = `0x7fc00000` | `=` | same |
| E3 | `inner` | `which = INT_MAX` | `NAN` = `0x7fc00000` | `=` | same |
| E4 | `inner` | `which = INT_MIN` | `NAN` = `0x7fc00000` | `=` | same |
| E5 | `inner` | any other out-of-range `which` (randomised, incl. values whose low bits alias a valid case) | `NAN` = `0x7fc00000` | `=` | `phase_c_errors::e5_inner_which_random_out_of_range` |
| E6 | `stb_perlin_ridge_noise3` | `octaves = 0` → loop body never runs | `sum` initialiser `0.0f` = `0x00000000` | `=` | `phase_c_errors::e6_e11_octaves_non_positive` |
| E7 | `stb_perlin_ridge_noise3` | `octaves < 0` (incl. `INT_MIN`) | `0.0f` | `=` | same |
| E8 | `stb_perlin_fbm_noise3` | `octaves = 0` | `0.0f` | `=` | same |
| E9 | `stb_perlin_fbm_noise3` | `octaves < 0` (incl. `INT_MIN`) | `0.0f` | `=` | same |
| E10 | `stb_perlin_turbulence_noise3` | `octaves = 0` | `0.0f` | `=` | same |
| E11 | `stb_perlin_turbulence_noise3` | `octaves < 0` (incl. `INT_MIN`) | `0.0f` | `=` | same |
| E12 | `stb__perlin_fastfloor` (all noise entry points) | coordinate is `NaN`: `(int)NaN` → `cvttss2si` indefinite `INT_MIN`, and `a < ai` is false | `px = INT_MIN`; result `NaN` `0x7fc00000` | `=` | `phase_c_errors::e12_e16_fastfloor_out_of_int_range` |
| E13 | `stb__perlin_fastfloor` | coordinate `= +inf` | `px = INT_MIN` → `x-px = +inf` → `NaN` `0xffc00000` | `=` | same |
| E14 | `stb__perlin_fastfloor` | coordinate `= -inf` | `px = INT_MIN` → `NaN` `0xffc00000` | `=` | same |
| E15 | `stb__perlin_fastfloor` | coordinate `>= 2^31` (e.g. `1e30`) | `px = INT_MIN` → `+inf` `0x7f800000` | `=` | same |
| E16 | `stb__perlin_fastfloor` | coordinate `<= -2^31` (e.g. `-1e30`) | `px = INT_MIN` → `NaN` `0xffc00000` | `=` | same |
| E17 | `stb_perlin_noise3*` | `x_wrap = INT_MIN` → `x_wrap-1` is signed overflow | gcc wraps: `INT_MAX & 255 = 255` | `=` | `phase_c_errors::e17_e19_wrap_mask_edges` |
| E18 | `stb_perlin_noise3*` | wrap argument that is **not** a power of two (violates the documented contract) | no rejection: `(w-1)&255` is used as the mask as-is | `=` | same |
| E19 | `stb_perlin_noise3*` | `x_wrap = 1` → mask `0` (single-cell noise) | index `0` everywhere | `=` | same |
| E20 | `stb_perlin_noise3_seed` | `seed` outside `0..=255` (e.g. `256`, `-1`, `INT_MAX`) | silently truncated: `(unsigned char)seed` | `=` | `phase_c_errors::e20_seed_truncation` |
| E21 | `inner` (`which = 5`) | `seed` outside `0..=255` passed to the `unsigned char` parameter | truncated by the prototype | `=` | same |
| E22 | `stb_perlin_noise3_wrap_nonpow2` | `x_wrap`/`y_wrap`/`z_wrap` `= 0` | replaced by `256` (`w ? w : 256`) | `=` | `phase_c_errors::e22_nonpow2_zero_wrap_is_256` |
| E23 | `stb_perlin_noise3_wrap_nonpow2` | `px < 0` → `x0 = px % w` is negative → `x0 += w` | corrected into `0..w-1`, in-bounds | `=` | `phase_c_errors::e23_nonpow2_negative_px` |
| E24 | `stb_perlin_noise3_wrap_nonpow2` | negative wrap with `px >= 0`: `px % (-w)` stays `>= 0` (C truncation towards zero), no correction | index `0..|w|-1`; in-bounds while `|w| <= 256` | `=` | `phase_c_errors::e24_nonpow2_negative_wrap_positive_px` |
| E25 | `stb_perlin_noise3_wrap_nonpow2` | wrap `> 256` so a table index lands in `512..1024` | reads the adjacent `stb__perlin_randtab_grad_idx` (deterministic, both C builds have the same `.data` layout) | `=` | `phase_c_errors::e25_nonpow2_index_into_grad_table` |
| E26 | `stb_perlin_noise3_wrap_nonpow2` | negative wrap with `px < 0`: `x0 += w` makes `x0` **negative** → `stb__perlin_randtab[negative]` | reads `.got.plt`/padding in front of `.data`: relocated pointers, so ASLR-dependent | `UB` | `phase_c_errors::e26_e27_deep_oob_is_not_reproducible` |
| E27 | `stb_perlin_noise3_wrap_nonpow2` | wrap so large that an index passes the end of `basis` (`>= 1216` bytes after `randtab`) | reads whatever the loader mapped after `.data` (differs between the executable and the `.so`) or `SIGSEGV` | `UB` | same |
| E28 | `stb_perlin_noise3_wrap_nonpow2` | `x_wrap = -1` **and** `px = INT_MIN` → `INT_MIN % -1` | `SIGFPE`, core dump (verified in a subprocess) | `UB` | `phase_c_errors::e28_int_min_mod_minus_one_traps_in_c` |
| E29 | `stb_perlin_noise3_wrap_nonpow2` | `x_wrap = INT_MIN`/`INT_MAX`: `px % INT_MIN == px`, and for `px < 0` the correction `x0 += INT_MIN` overflows | gcc wraps the addition; reproducible whenever the resulting index stays in the modelled window (always the case for `px >= 0`, never for `px < 0`, which then falls under E27) | `=` | `phase_c_errors::e29_nonpow2_int_min_wrap` |
| E30 | `stb__perlin_grad` | `grad_idx` outside `0..=11` (only reachable through an out-of-bounds gradient-table read) | reads past `basis[12][4]` → garbage or `SIGSEGV` | `UB` | `phase_c_errors::e26_e27_deep_oob_is_not_reproducible` |
| E31 | `main` | stdin is empty (immediate `EOF`) → `scanf` returns `EOF` | all twelve variables keep their `0` initialisers → `inner(0, 0,0,0, 0,0,0, 0, 0,0,0, 0)` printed with `%.9g` | `=` | `driver_cli::e31_e40_scanf_rejections` |
| E32 | `main` | first token is not a number (`"abc"`) → matching failure on the 1st `%d` | as E31 | `=` | same |
| E33 | `main` | matching failure on conversion *k* (`k = 2..12`, e.g. `"0 1 2 x …"`) | the first *k-1* variables are assigned, the rest keep `0` | `=` | same |
| E34 | `main` | `%d` value out of `long` range (`"99999999999999999999999"`) | `strtol` saturates to `LONG_MAX`, then the `long` is stored into an `int` → `-1` | `=` | same |
| E35 | `main` | `%d` value below `LONG_MIN` (`"-99999999999999999999999"`) | saturates to `LONG_MIN` → stored as `0` | `=` | same |
| E36 | `main` | `%f` sees only a prefix of `inf`/`nan` (`"in"`, `"na"`, `"infin"`) | the prefix is consumed, the conversion fails | `=` | same |
| E37 | `main` | `%f` sees `"0x"` with nothing usable behind it (`"0x"`, `"0x "`, `"0xp1"`, `"0xz"`) | `"0x"` is consumed and the conversion fails (glibc rejects a subject sequence of exactly `0x`) | `=` | same |
| E37b | `main` | `%f` sees `"0x."` / `"0x.p1"` / `"0x.g"` — a radix point but no hex digit | the conversion **succeeds** with `0.0`: glibc's collector accepts the `.` and `strtof` then converts the leading `0` (and the `p` is *not* consumed, because no hex digit preceded it) | `=` | same + `cscan::glibc_tests::float_tokens_match_glibc` |
| E38 | `main` | `%f` exponent marker without digits (`"1e"`, `"1e+"`) | value `1.0` is assigned (exponent ignored), marker consumed | `=` | same |
| E39 | `main` | `%f` decimal-exponent overflow / underflow (`"1e400"`, `"1e-400"`) | `ERANGE`: `+inf` / `0` | `=` | same |
| E40 | `main` | `%f`/`%d` sign with no digits (`"-"`, `"+"`) | sign consumed, conversion fails | `=` | same |
| E41 | `main` | more than twelve tokens on stdin | extra input ignored, `scanf` stops after the 12th conversion | `=` | same |
| E42 | `main` | `which` field itself invalid (`which = 7` read successfully) | `inner` returns `NAN` → prints `nan` | `=` | same |
| E43 | `main` | result is `-NaN` (sign bit set, e.g. `which=0`, `x=inf`) | `printf("%.9g")` prints `-nan` | `=` | same |

## Generic FFI boundary cases (covered even though the C has no checks)

| # | condition | expected | test |
|---|-----------|----------|------|
| G1 | every `int` parameter at `INT_MIN` / `INT_MAX` / `0` / `-1` | same bits from both `.so`s | `phase_c_errors::g1_int_extremes` |
| G2 | out-of-range "enum" value crossing the FFI boundary: `which` is a plain `int` in C, so *any* of the 2^32 values is a legal argument — randomised over all of them | `NAN` for everything outside `0..=5`, the matching case otherwise | `phase_c_errors::e5_inner_which_random_out_of_range`, `phase_b_inner::c38_inner_each_case` |
| G3 | `unsigned char seed` parameter receiving `0`, `255` and (through `inner`) a truncated `int` | same bits | `phase_c_errors::e20_seed_truncation` |
| G4 | every `float` parameter at `±0.0`, `±inf`, `NaN` (quiet, payload `0`), `FLT_MIN`, `FLT_MAX`, smallest subnormal, `-1`, `1` | same bits | `phase_c_errors::g4_float_extremes` |
| G5 | random 32-bit patterns reinterpreted as `float` (includes signalling NaNs and every subnormal) for all seven noise entry points | same bits | `phase_c_errors::g5_random_bit_patterns` |
| G6 | `octaves` large enough for `frequency`/`amplitude` to overflow to `inf`/underflow to `0` (`64`, `300`) | same bits | `phase_b_fractal::c22_ridge_extreme_octaves`, `phase_b_fractal::c26_fbm_extreme`, `phase_b_fractal::c29_turbulence_extreme` |
| G6b | `octaves` large enough for `frequency`/`amplitude` to overflow (`16`, `32`, `64`, `130`, `300`) | same bits | `phase_b_fractal::c22_ridge_extreme_octaves`, `c26_fbm_extreme`, `c29_turbulence_extreme` |
| G7 | no pointer/length parameters exist in this API (nothing to null-check): every prototype in `stb_perlin.h` takes `float`/`int`/`unsigned char` by value, and `main` takes none -- so there is no null-pointer, zero-length or oversized-length row to write | n/a | documented (verified against the prototypes) |
| G8 | the `scanf`/`printf` emulation compared **directly against glibc** (the C program's own implementation) over 40 000 randomised tokens and 400 000 float values, checking the converted value, the number of characters consumed (`%n`) and the formatted text | identical | `cscan::glibc_tests::*`, `cfmt::glibc_tests::*` |

## Notes on the `UB` rows

`stb_perlin_noise3_wrap_nonpow2` is the only function that can index the
permutation table out of bounds, which happens exactly when a wrap argument is
outside `1..=256` (`0` means `256`). The `.data` layout of *both* C builds is

```
stb__perlin_randtab +0 (512) | stb__perlin_randtab_grad_idx +512 (512) | basis +1024 (192)
```

so indices in `0..1024` are reproducible and `src/stb_perlin.rs::read_table_mem`
models exactly that window (rows E23–E25, E29 are therefore bit-exact `=` rows).
Anything outside that window reads bytes that are *not* part of the program's
data (`.got.plt` entries subject to ASLR in front of it, ELF section headers or
unmapped pages behind it), i.e. the C result differs between the executable and
the shared object built from the very same source, and can also `SIGSEGV`. Those
rows (E26, E27, E30) are pinned as follows instead of by value:

* the C `.so` is *shown* to disagree with the C executable for such an input
  (so no implementation could match both), and
* the Rust `.so` is shown to stay memory-safe and to return a value (it reads
  `0` outside the modelled window) instead of panicking.

Row E28 (`INT_MIN % -1`) is a hardware trap: the test runs the C call in a
child process and asserts it dies with `SIGFPE`, and that the Rust `.so`
returns normally (`wrapping_rem` → `0`).
