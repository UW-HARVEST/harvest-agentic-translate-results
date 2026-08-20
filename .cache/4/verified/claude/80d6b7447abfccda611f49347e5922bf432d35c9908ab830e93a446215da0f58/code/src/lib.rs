//! Rust translation of the C library in `c_src/` — a stripped-down `cute_c2`
//! 2D collision-detection library.
//!
//! Built as a `cdylib`, this crate exports exactly the same 46 dynamic symbols as the
//! C shared object (verified with `nm -D`: the diff is empty in both directions) and
//! is byte-for-byte behaviour-compatible with it.
//!
//! # Module map
//!
//! | module | contents |
//! |---|---|
//! | [`types`] | all `repr(C)` structs, layout-verified against GCC |
//! | [`fp`] | explicit SSE NaN-propagation for commutative `+`/`*` |
//! | [`math`] | scalar / vector / rotation primitives |
//! | [`shapes`] | plane extraction, AABB corners, proxies, support map, edge normals |
//! | [`gjk`] | simplex reduction, witness points, `c2GJK` |
//! | [`manifold`] | clipping helpers and the per-shape-pair manifold generators |
//! | [`api`] | `c2Collide`, `ptr_from_parts`, `omni_manifold` |
//!
//! The five `static` helpers in the C source (`c2Clip`, `c2SidePlanes`,
//! `c2SidePlanesFromPoly`, `c2KeepDeep`, `c2Incident`) are private here too, matching
//! the C library's dynamic symbol table.
//!
//! # Achieving bit-exact floating point
//!
//! `c_src/CMakeLists.txt` sets no `CMAKE_BUILD_TYPE` and no optimisation flags, so
//! the reference `.so` is compiled at **`-O0`**. That is the build every claim below
//! was read off, by disassembling `c_src/build/libtranslated_rust.so` with
//! `objdump -d` — not inferred from the source.
//!
//! * All arithmetic is `f32`, associated exactly as the C source associates it. GCC
//!   does not reassociate floating point without `-ffast-math`, and neither toolchain
//!   enables FMA at the x86-64 baseline (SSE2 only), so the arithmetic agrees exactly.
//! * Commutative `+` / `*` go through [`fp::add`] / [`fp::mul`], which take the SSE
//!   **destination** operand first. On x86, when both operands of `addss`/`mulss` are
//!   NaN the result is the *destination* operand quieted, so the compiler's otherwise
//!   invisible choice of destination register selects which NaN payload survives. At
//!   `-O0` GCC's choice is not "always the left operand": it falls out of the
//!   load order, and several sites end up reversed. Each one is recorded in a comment
//!   at its call site next to the instructions it came from. The notable ones:
//!
//!   | function | C source | GCC `-O0` destination operands |
//!   |---|---|---|
//!   | [`math::c2Mulvs`] | `a.x *= b` | `mul(a.x, b)` — the component, not the scalar |
//!   | [`math::c2Add`] | `a.x + b.x` | `add(b.x, a.x)` — **reversed** |
//!   | [`math::c2Dot`] | `a.x*b.x + a.y*b.y` | `add(mul(b.y, a.y), mul(a.x, b.x))` — the two `mulss` pick *opposite* operands, and the `addss` destination is the **y** product |
//!   | [`math::c2Det2`] | `a.x*b.y - a.y*b.x` | `mul(b.y, a.x) - mul(b.x, a.y)` |
//!   | [`math::c2Mulrv`] | see source | x lane `mul(b.x, a.c) - mul(b.y, a.s)`, y lane `add(mul(a.s, b.x), mul(b.y, a.c))` |
//!   | [`math::c2MulrvT`] | `-a.s * b.x + a.c * b.y` | kept as an `xorps` negate plus `addss`, **not** folded into a subtraction: `add(mul(-a.s, b.x), mul(b.y, a.c))` |
//!   | `gjk::c2Witness`, `gjk::c2L` | `den * s->a.u` | `mul(u, den)` — the weight is the destination |
//!   | `gjk::c23` interior | `uABC + vABC + wABC` | strictly left-associated: `add(add(uABC, vABC), wABC)` |
//!   | `manifold::c2CircletoCircleManifold` etc. | `A.r + B.r` | `add(B.r, A.r)` — **reversed** |
//!   | `manifold::c2AABBtoAABBManifold` | `eA.x + eB.x` | `add(eA.x, eB.x)` — source order |
//!   | `manifold::c2CapsuletoPolyManifold` | `m->depths[i] += A.r` | `add(A.r, depths[i])` — **reversed** |
//!
//!   Non-commutative ops (`-`, `/`) need no helper: their destination is forced to the
//!   left operand, so plain `-` and `/` already agree.
//! * `c2Maxv` / `c2Minv` / `c2Absv` use raw comparisons, not `f32::max` / `f32::min` /
//!   `f32::abs`: C's ternaries return the *second* operand when the first is NaN, and
//!   return `-0.0` unchanged, whereas the Rust library functions do neither.
//! * Every exported function is `#[inline(never)]`, mirroring the C library's
//!   structure (at `-O0` GCC really does emit `call c2Dot@plt` from `c2Len`). With the
//!   destination operands pinned explicitly this is no longer load-bearing for
//!   correctness, but it keeps the Rust call graph aligned with the C one.
//! * Array indexing that C leaves unchecked is done with raw pointer offsets, so no
//!   bounds check can panic across the FFI boundary where C would happily read on.
//!
//! # Bugs are reproduced, not fixed
//!
//! * `c2MakeProxy` and `ptr_from_parts` have no `C2_TYPE_POLY` case and no `default`,
//!   and `c2Collide` silently ignores polygons. See [`gjk::c2GJK`] and
//!   [`api::ptr_from_parts`].
//! * `omni_manifold` leaks both shapes it allocates.
//! * `c2CapsuletoPolyManifold` reads `B->verts[-1]` whenever every candidate face
//!   distance is NaN, because `index` keeps its `~0` initialiser. That read is
//!   reproduced with a raw offset. Where the `c2Poly` is the library's *own* local —
//!   `c2AABBtoCapsuleManifold` — [`manifold`] pins GCC's frame layout with a
//!   `#[repr(C)]` struct so the same bytes are read (`A.max.y` followed by `p.count`).
//!
//! # Where the C library is not a function of its inputs
//!
//! `c2GJK` declares `c2Proxy pA, pB;` uninitialised and `c2MakeProxy` writes nothing
//! for `C2_TYPE_POLY`, so **every polygon path reads uninitialised stack**. Measured
//! against the compiled C library:
//!
//! * From a debug harness it reads back the two halves of a stack address, so the
//!   returned distance is nonsense and moves with ASLR.
//! * From a fresh minimal C driver — and from a release harness — the garbage
//!   `pB.count` makes `c2Support` walk off the end of the array and the process dies
//!   with **SIGSEGV (exit 139), reproducibly**.
//!
//! No translation can match that, so [`gjk::c2GJK`] zero-initialises both proxies.
//! This reproduces the "virgin, zero-filled stack page" case exactly: a POLY operand
//! behaves as a single point at the origin with radius 0 — deterministic and
//! non-crashing. The affected entry points are `c2GJK` with a POLY/out-of-range type,
//! `c2CapsuletoPolyManifold`, `c2AABBtoCapsuleManifold`, and `c2Collide` /
//! `omni_manifold` for the `(AABB, CAPSULE)` and `(CAPSULE, AABB)` pairs.
//!
//! `tests/probe_uninit.rs` characterises this (running the deliberately-UB call in a
//! child process so a SIGSEGV cannot take the suite down), and the differential tests
//! zero the stack immediately before both the C and the Rust call, which forces the C
//! side into the same state and makes those paths comparable byte-for-byte after all.
//! That is the only way the five `static` helpers `c2Clip`, `c2SidePlanes`,
//! `c2SidePlanesFromPoly`, `c2KeepDeep` and `c2Incident` can be reached at all.
//!
//! `ptr_from_parts` has one further, unreproducible case: for `C2_TYPE_POLY` control
//! falls off the end of a non-`void` function, so GCC returns a stale callee-saved
//! register. This crate returns `NULL`. It is unobservable — `c2Collide` has no POLY
//! case either, so the pointer is never dereferenced.
//!
//! # Verification
//!
//! See `SYMBOLS.md` (symbol parity), `CONFIGS.md` (valid-input configuration matrix),
//! `ERRORS.md` (rejection / degenerate-path matrix) and `VERIFICATION.md` (results).
//! `./run_all_features.sh` enumerates every Cargo feature combination — the crate
//! declares none, so there is exactly one — and runs the whole suite under each.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
// The C source assigns struct fields statement by statement, and this translation
// mirrors that order because the order is observable: `c2AABBtoCapsuleManifold` sets
// `p.count` *between* `c2BBVerts` and `c2Norms`, and `p.verts[-1]` reads `p.count`
// back. Collapsing the assignments into initialisers would be a behavioural change.
#![allow(clippy::field_reassign_with_default)]
// `C2_FLT_MAX` / `C2_FLT_EPSILON` are spelled with every digit the C source spells
// them with, so the constants can be diffed against `c_src/src/lib.c` by eye.
#![allow(clippy::excessive_precision)]
// `c2Poly` / `c2Proxy` keep hand-written `Default` impls next to their `#[repr(C)]`
// definitions so the zero-initialisation that `c2GJK` relies on is explicit rather
// than derived. See the module docs on uninitialised proxies.
#![allow(clippy::derivable_impls)]

pub mod api;
pub mod fp;
pub mod gjk;
pub mod manifold;
pub mod math;
pub mod shapes;
pub mod types;
