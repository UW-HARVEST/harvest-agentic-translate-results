# Mismatches found while verifying the translation

The C in `c_src/` is the ground truth. Every entry below is a place where the
Rust program printed something different from the C program for the same stdin,
together with what caused it and how the Rust was changed. The C was never
touched.

How each was found: build both binaries, feed identical bytes on stdin, compare
stdout, stderr and the exit status (including the terminating signal). The
committed corpus lives in `tests/differential.rs`; the search that produced
these entries additionally used randomised fuzzing (structured 12-field inputs,
random token streams, random raw bytes) and an exhaustive sweep of the
out-of-bounds table index space of `stb_perlin_noise3_wrap_nonpow2`.

The reference for the operand-level questions below is the assembly gcc emits
for `c_src/src/main.c` as built by the supplied `CMakeLists.txt` — no
`CMAKE_BUILD_TYPE`, hence no optimisation flags and no FP contraction:

```
gcc -I c_src/src -S -o main.s c_src/src/main.c
```

---

## 1. `sum += ...` in the three fractal loops had its SSE operands swapped

Input `2 -1e+20 0.0773796436 14.8969174 1 4 2 2 -2 0 1 8`
(`stb_perlin_ridge_noise3`):

| | stdout |
|---|---|
| C | `-nan` |
| Rust (before) | `nan` |

`x = -1e20` overflows `stb__perlin_fastfloor` (`cvttss2si` yields `INT_MIN`,
then `ai-1` wraps to `INT_MAX`), so the octaves produce `±inf`, and `inf-inf` /
`0*inf` produce the x86 "indefinite" QNaN `0xffc00000`, which prints as `-nan`.
From that point on the *sign* of the NaN is observable.

`addss`/`subss`/`mulss` return the destination operand when both operands are
NaN, so the C-source operand that gcc puts in the destination register decides
the sign. gcc compiles `sum += r*amplitude*prev` as

```asm
movss  -24(%rbp), %xmm0     ; r
mulss  -16(%rbp), %xmm0     ; * amplitude
mulss  -12(%rbp), %xmm0     ; * prev
movss  -20(%rbp), %xmm1     ; sum
addss  %xmm1, %xmm0         ; destination is the PRODUCT, not sum
```

i.e. `product + sum`, not `sum + product`. The translation had
`fadd(sum, product)`. Fixed in `src/perlin.rs` for all three of
`stb_perlin_ridge_noise3`, `stb_perlin_fbm_noise3` and
`stb_perlin_turbulence_noise3`.

## 2. The last `addss` of `stb__perlin_grad` had its operands swapped

Found by reading the same assembly while fixing (1); it is reachable from every
`which` once an operand is a NaN. gcc compiles
`grad[0]*x + grad[1]*y + grad[2]*z` as

```asm
mulss ...  %xmm1            ; xmm1 = grad[0]*x
mulss ...  %xmm0            ; xmm0 = grad[1]*y
addss %xmm0, %xmm1          ; xmm1 = (grad[0]*x) + (grad[1]*y)
mulss ...  %xmm0            ; xmm0 = grad[2]*z
addss %xmm1, %xmm0          ; destination is grad[2]*z, source is the sum
```

The translation had `fadd(sum, grad[2]*z)`; it is now
`fadd(grad[2]*z, sum)`.

Also corrected for exactness in the same pass (both are NaN-neutral because the
other operand is a literal, so neither changed an output on its own):
`stb__perlin_ease` is `mulss 6.0f, a` and `addss 10.0f, (t*a)`, not
`a*6` / `(t*a)+10`.

## 3. `(float) fabs(r)` was routed through `double`

The translation computed `((r as f64).abs()) as f32`. gcc does not call `fabs`
at all — it expands it inline to a bit operation:

```asm
movss  .LC6(%rip), %xmm1    ; 0x7fffffff
andps  %xmm0, %xmm1
```

The round trip through `f64` is value-preserving, but Rust only guarantees that
a NaN cast to `f64` and back is *some* NaN, not that the payload survives. It is
now `f32::from_bits(r.to_bits() & 0x7fff_ffff)`, which is exactly `andps`.

## 4. Out-of-range `*_wrap` values: the C reads outside its tables

Input `5 -100.5 0.0773796436 -0.0000001 8 -2147483647 -4000 65535 0.5 1 1.25 20`:

| | stdout |
|---|---|
| C | `0.211310208` |
| Rust (before) | `0.211310118` |

`stb_perlin_noise3_wrap_nonpow2` does not clamp its indices:

```c
int x0 = px % x_wrap2;          /* range is (-|x_wrap2|, |x_wrap2|) */
if (x0 < 0) x0 += x_wrap2;      /* still negative when x_wrap2 < 0  */
r0 = stb__perlin_randtab[x0];   /* the array is 512 bytes           */
```

so any `|wrap| > 512`, or a negative wrap, indexes `stb__perlin_randtab`,
`stb__perlin_randtab_grad_idx` and — through a `grad_idx` above 11 — `basis`
outside their bounds. The translation returned 0 for such reads. The C returns
whatever is at that address, and that value feeds back into the tables and
changes the printed result.

Formally undefined behaviour, but deterministic for the binary
`CMakeLists.txt` produces (a non-PIE ELF loaded at the fixed address
`0x400000`). `src/procimage.bin` is now the byte-exact image of the loaded pages
`[0x400000, 0x406000)`, reconstructed from the `PT_LOAD` headers, and
`src/mem.rs` resolves every table access through it. The symbol addresses come
from `nm -S c_src/build/driver`:

```text
0x405040  stb__perlin_randtab            512 bytes
0x405240  stb__perlin_randtab_grad_idx   512 bytes
0x405440  basis.0 (the static in stb__perlin_grad)  192 bytes
0x405500  __bss_start .. 0x405508 _end
0x406000  first unmapped page
```

`src/mem.rs` has unit tests that re-check the image against the tables
transcribed by hand from `c_src/src/stb_perlin.h`, so the blob cannot drift.

A verified sweep of the whole reachable index space (`x0`, `y0`, `z0` driven
individually from `0` to `±1300`, plus the regions around the page boundary and
the extremes) now agrees with the C on all 11517 inputs.

## 5. `INT_MIN % -1` traps: the C dies with SIGFPE

Input `5 -2147483648 0 0 -1 0 0 0 0 0 0 0`:

| | stdout | exit |
|---|---|---|
| C | *(empty)* | killed by SIGFPE (shell status 136) |
| Rust (before) | `0` | 0 |

`px % x_wrap2` compiles to `idivl`, which raises `#DE` when the quotient
overflows, i.e. for `INT_MIN % -1`. `px` is `INT_MIN` whenever
`stb__perlin_fastfloor` saturates (`nan`, `±inf`, `|x| >= 2^31`, or exactly
`-2147483648`). `src/mem.rs::crem_i32` now performs a real `idiv` in that case
so the process dies with the same signal.

## 6. Out-of-range indices past the last mapped page: the C dies with SIGSEGV

Input `5 1e9 0 0 2000000000 0 0 0 0 0 0 0`:

| | stdout | exit |
|---|---|---|
| C | *(empty)* | killed by SIGSEGV (shell status 139) |
| Rust (before) | `0` | 0 |

Once the index leaves `[0x400000, 0x406000)` the C faults. This is also
reachable without a huge wrap: a `grad_idx` byte of 192 or more makes
`basis[grad_idx]` land past `0x406000`, since `basis` sits only `0xbc0` bytes
below it. `src/mem.rs::segv` now issues a genuine null load so the Rust process
is killed the same way, with the same empty stdout.

---

## Known residual difference: 18 bytes the C reads are ASLR-randomised

`c_src` is *not* deterministic for a small set of inputs, so no translation can
match it there. Between `stb__perlin_randtab` and the start of the RW segment
sit words the dynamic linker fills in at load time with libc / ld.so / heap
addresses:

| address | contents | bytes that vary per run (relative to `randtab`) |
|---|---|---|
| `0x404ec0` | `.dynamic` `DT_DEBUG` | -383 .. -380 |
| `0x404fc8` | `.got` `__libc_start_main` | -118 .. -116 |
| `0x404ff0` | `.got.plt[1]` (link map) | -79 .. -76 |
| `0x404ff8` | `.got.plt[2]` (`_dl_runtime_resolve`) | -71 .. -68 |
| `0x405008` | `.got.plt` slot for `__isoc99_scanf` | -55 .. -52 |

`printf`'s slot at `0x405000` is still lazily unresolved while the noise runs,
so it keeps its file value; `scanf`'s has been resolved by then, so it does not.

For example `5 0 0 -783 1 1 -782 0 0 0 0 0` reads byte 4 of
`.got.plt[2]`; over 40 runs the C printed `0` thirty times and was killed by
SIGSEGV ten times. `src/procimage.bin` holds one plausible sample for those 18
bytes, chosen below 192 so that the derived `basis` index stays inside the
mapped pages — that is the majority outcome (a uniform byte is below 192 with
probability 3/4). The bytes that *are* stable (the low byte of each pointer, and
`0x7f` / `0x00` in the top three) are reproduced exactly.

`tests/differential.rs` deliberately contains no input that reads these 18
bytes, and the fuzzing scripts treat a C program that disagrees with itself
across repeated runs as "not a mismatch".

---

## Note on the previous test file

`tests/golden.rs` compared the Rust binary against hard-coded strings. It has
been replaced by `tests/differential.rs`, which runs the real C binary from
`c_src/` as a subprocess and compares stdout, stderr *and* the exit status,
because a golden string cannot catch a wrong exit status and cannot be checked
against the C at all. Its cases are all included in the new corpus. No test is
`#[ignore]`d, skipped or disabled.
