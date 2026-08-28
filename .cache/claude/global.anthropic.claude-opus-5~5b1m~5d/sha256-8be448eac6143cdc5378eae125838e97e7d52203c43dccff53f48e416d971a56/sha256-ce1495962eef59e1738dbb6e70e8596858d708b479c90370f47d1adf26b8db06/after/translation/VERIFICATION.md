# VERIFICATION.md — how this translation was verified, and what was fixed

## How to reproduce

```sh
cd translation
./run_all.sh          # builds the C .so, then every test × every feature combo × both profiles
./check_symbols.sh    # nm -D diff between the two .so files
./mutation_check.sh   # proves the differential suite actually detects divergences
```

Everything runs offline (`--offline`); `libloading 0.8` is the only dev-dependency.

## Test layout

All tests load **both** shared objects with `libloading` and call **only** through
`dlsym`-resolved `extern "C"` function pointers — the Rust crate's functions are
never called directly, so the `#[no_mangle]` export wrappers are under test too.

| file | phase | covers |
|------|-------|--------|
| `tests/common/mod.rs` | harness | `dlopen` of both `.so`s, the 22 typed signatures, the seeded xorshift RNG, bit-exact comparators |
| `tests/phase_b_scalars.rs` | B | `CONFIGS.md` rows 1–19, 57 |
| `tests/phase_b_predicates.rs` | B | rows 20–26 |
| `tests/phase_b_raycast.rs` | B | rows 27–45, 58 |
| `tests/phase_b_toplevel.rs` | B | rows 46–56 |
| `tests/phase_b_nan_payloads.rs` | B | rows 59–62 (distinct-NaN-payload stress) |
| `tests/phase_c_errors.rs` | C | all 44 `ERRORS.md` rows + null pointers + out-of-range enums |
| `tests/phase_d_symbols.rs` | D | `nm -D` parity, `dlsym` resolvability, struct layout |
| `tests/probe_castray_ub.rs` | — | diagnostic printout for the `c2CastRay` UB (not a gate) |

Every comparison is **bit-exact** (`f32::to_bits`, `c_int` equality) — never
epsilon-based — and each raycast pre-fills the `c2Raycast` out-parameter with the
sentinel `{t: 0xdeadbeef, n: (0xcafebabe, 0xfeedface)}` so "did it write `*out`
on the reject path?" is part of the compared state.

## Reference build

The C is built exactly as the task specifies:

```sh
cd c_src && mkdir -p build && cd build
cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```

`CMakeLists.txt` sets no `CMAKE_BUILD_TYPE`, so `CMAKE_C_FLAGS` is empty →
**gcc 11.5 at `-O0`**. Every function is a real, non-inlined call, and every
float operation is a single SSE scalar instruction. The Rust was matched against
`objdump -d` of that build, instruction for instruction.

## Divergences found and fixed

The prior translation already had the right structure and the right *values* for
ordinary inputs. What it got wrong was **`mulss`/`addss` operand order**, which
is observable whenever both operands are NaN: the instruction returns the
*destination* operand's payload, and gcc's `-O0` choice of destination is not
the source-order-obvious one.

| # | site | was | should be (per `objdump`) | caught by |
|---|------|-----|---------------------------|-----------|
| 1 | `c2Dot` y-lane product | `fmul(a.y, b.y)` | `fmul(b.y, a.y)` — gcc puts `b.y` in the destination | `cfg_03_c2dot_specials` |
| 2 | `c2Dot` sum | `fadd(x_prod, y_prod)` | `fadd(y_prod, x_prod)` — the *y* product is the `addss` destination | `cfg_04_05_c2len` |
| 3 | internal `add` (used by every caller of `c2Add`) | `fadd(a.x, b.x)` | `fadd(b.x, a.x)` — `b` is the destination in both lanes | `cfg_57_scalar_bitpattern_fuzz` |
| 4 | `c2Mulvs` y-lane | `fmul(b, a.y)` | `fmul(a.y, b)` — gcc uses a memory operand, so `a.y` is the destination in *both* lanes | `cfg_10_11_div` |
| 5 | `c2MulmvT` (internal **and** the exported wrapper, which had *different* wrong orders) | `fadd(fmul(a.x.x,b.x), fmul(a.x.y,b.y))` etc. | `fadd(fmul(b.y,a.x.y), fmul(a.x.x,b.x))` per row | `cfg_18_19_mulmvt` |
| 6 | `c2RaytoAABB` `out->t` (all four branches) | `fmul(tN, A.t)` | `fmul(A.t, tN)` — `A.t` is the `mulss` destination | `nan_payload_gen_ray` (t3 branch) |
| 7 | `c2RaytoCapsule` `y` | `fadd(yAp.y, prod)` | `fadd(prod, yAp.y)` | (equivalent — see below) |
| 8 | `c2RaytoCapsule` `out->t` | `fmul(t, A.t)` | `fmul(A.t, t)` | (equivalent — see below) |

The exported `c2Add` and `c2MulmvT` wrappers had hand-written bodies that
disagreed with the internal helpers the rest of the library calls; they now
delegate to the internal ones, which removes the possibility of the two drifting
apart. (At `-O0` the C really does `call c2Add@plt` from every caller, so the
internal and exported behaviour *must* be identical.)

### The `c2CastRay` missing `default:`

`c2CastRay` has no `default:` label and no `return` after its `switch`, so an
out-of-range `C2_TYPE` falls off the end of a non-`void` function. `gcc -O0`
compiles that literally:

```asm
c2CastRay:
    ...
    cmpl  $0x2,-0xc(%rbp)
    ja    274b            ; out of range → straight to the epilogue
    ...
274b: leave
274c: ret                 ; %eax NEVER written
```

so the value the caller sees is **whatever it left in `%eax`**, and `*out` is
untouched.

The previous translation returned `B as usize as u32`, i.e. the low 32 bits of
the `B` pointer. `tests/probe_castray_ub.rs` shows that is simply not what the C
does: the C returned `0x249bd63c` where the `B` pointer's low half was
`0x245fe350`. `0x249bd63c` is the load address of `c2CastRay` **itself** — rustc
emits `call *%rax` for a function pointer, so `%eax` on entry holds the callee's
own address.

The export is now a `#[unsafe(naked)]` stub with the same instruction semantics
as the C:

```rust
core::arch::naked_asm!(
    "cmp esi, 2",
    "ja 2f",
    "jmp {dispatch}",   // in range: tail-call, no argument shuffling needed
    "2:",
    "ret",              // out of range: %eax and *out left exactly as the caller had them
    dispatch = sym c2CastRay_dispatch,
)
```

`ERRORS.md` row 38 / `err_38_castray_out_of_range_type` asserts the part of this
that *can* hold for any caller:

1. `*out` is byte-identical (untouched) in both libraries;
2. neither dereferences `B` (a null `B` is safe in both);
3. both return their **incoming `%eax`** — checked as "either the two returns
   agree, or each equals the low 32 bits of its own entry address".

Point 3 is the meaningful equality: it proves the Rust reproduces the C's
`%eax`-preservation rather than substituting an invented constant. When the caller
uses `call *%rax`, each library necessarily returns its own address — that
residual difference is inherent to the UB and to the caller, not to the
translation, and no translation can remove it. A mutation that returns `0`
instead is caught by this test.

## Verification of the verification

`./mutation_check.sh` injects 58 wrong variants one at a time and requires the
suite to catch each: **49 killed, 9 survivors, 0 skipped**, and every survivor is
a documented *equivalent* mutant with a reachability proof in the script's header
comment. The interesting ones:

* `c2RaytoAABB`'s `out->t` in the `t0`/`t1`/`t2` branches — taking one of those
  branches requires `t0 >= t1 && t0 >= t2 && t0 >= t3`, an ordered comparison
  that is false for NaN, so `t0` is never NaN *in its own branch*. Only the final
  `else` (`t3`) can multiply two NaNs, and that mutant **is** killed, which proves
  the site is genuinely exercised rather than merely unreached.
* `c2RaytoCapsule`'s `out->t` and `y` — reaching the side-wall branch requires
  ordered comparisons on `yAp.x`, which forces the second operand to be non-NaN.
* `hit_i as f32 * t_i` — `hit_i as f32` is exactly `0.0` or `1.0`, never NaN.
* `fdiv(1.0, b)` vs `1.0f32 / b` — the same `divss`.
* `abs_ternary(dot(a,a))` — `dot(a,a)` is a sum of two squares, never negative
  and never `-0.0`, so the `abs` is a no-op.

Notably, before `tests/phase_b_nan_payloads.rs` existed, **17** mutations were
killed and 5 survived; the distinct-payload NaN pool is what closed that gap.
A fuzzer that reuses one `f32::NAN` value cannot see operand order at all.

## Configurations covered

`Cargo.toml` declares **no `[features]` table**, so the feature cross-product is
just `{default}` = `{--no-default-features}`. `run_all.sh` still enumerates it
mechanically (so it stays correct if features are ever added) and runs:

| profile | feature combo | result |
|---------|---------------|--------|
| release | default | 89 tests pass |
| release | `--no-default-features` | 89 tests pass |
| dev | default | 89 tests pass |
| dev | `--no-default-features` | 89 tests pass |

Both profiles matter because the tests `dlopen`
`target/<profile>/libgen_ray_lib.so`, so each profile's actual artifact is the
one under test.

## Known limits

* The comparison is against a **`-O0` gcc 11.5** build, as the task's build
  command produces. At `-O2` gcc may vectorise the two lanes (`mulps`), which can
  change *NaN payload* selection — never ordinary values, since gcc does not
  reassociate floating point without `-ffast-math` and the x86-64 baseline has no
  FMA contraction.
* `c2RaytoCapsule` dereferences `out` unconditionally, and `c2RaytoCircle` /
  `c2RaytoAABB` do so on their hit paths. A null `out` there is UB in the C, so
  it is not exercised by dereference; `generic_null_out_on_early_reject` instead
  tests null `out` on the paths where the C provably never touches it.
