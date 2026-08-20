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
//! * All arithmetic is `f32`, associated exactly as the C source associates it. GCC
//!   does not reassociate floating point without `-ffast-math`, and neither toolchain
//!   enables FMA at the x86-64 baseline (SSE2 only), so the arithmetic agrees exactly.
//! * Every exported function is `#[inline(never)]`. The C library is built without
//!   `-fno-semantic-interposition`, so GCC routes *every* public→public call through
//!   the PLT (e.g. `c2Len` really does `call c2Dot@plt`). Letting LLVM inline these
//!   instead would change which operand ends up as an instruction's destination, and
//!   therefore which NaN survives.
//! * `c2Maxv` / `c2Minv` / `c2Absv` use raw comparisons, not `f32::max` / `f32::min` /
//!   `f32::abs`: C's ternaries return the *second* operand when the first is NaN, and
//!   return `-0.0` unchanged, whereas the Rust library functions do neither.
//! * Commutative `+`/`*` go through [`fp::add`] / [`fp::mul`], which take the SSE
//!   *destination* operand first. GCC's choice of destination is recorded at each
//!   site (several are reversed relative to the source order, e.g. `c2Mulvs` is
//!   vectorised so the broadcast scalar is the destination, and
//!   `c2AABBtoAABBManifold` adds the extents as `eB + eA`).
//! * Array indexing that C leaves unchecked is done with raw pointer offsets, so no
//!   bounds check can panic across the FFI boundary where C would happily read on.
//!
//! # Bugs are reproduced, not fixed
//!
//! * `c2MakeProxy` and `ptr_from_parts` have no `C2_TYPE_POLY` case and no `default`,
//!   and `c2Collide` silently ignores polygons. See [`gjk::c2GJK`] and
//!   [`api::ptr_from_parts`].
//! * `omni_manifold` leaks both shapes it allocates.
//!
//! # Where the C library is not a function of its inputs
//!
//! Because `c2MakeProxy` never writes the proxy for `C2_TYPE_POLY`, `c2GJK` reads an
//! uninitialised `c2Proxy` off its own stack on every polygon path — which includes
//! `c2CapsuletoPolyManifold`, `c2AABBtoCapsuleManifold` and hence
//! `omni_manifold`/`c2Collide` for AABB-vs-capsule. Measured against the compiled C
//! library:
//!
//! * Normally that stack region is virgin, zero-filled, so the polygon proxy behaves
//!   as a single point at the origin with radius 0. This crate reproduces that exact
//!   behaviour by zero-initialising the proxies — deterministic and non-crashing.
//! * When the region has been dirtied, the proxy's `count` becomes garbage and
//!   `c2Support` walks off the end of the array. The C library therefore **segfaults
//!   in roughly a third of runs** purely as a function of stack layout / ASLR.
//!
//! A related case: in `c2CapsuletoPolyManifold` the `if (d > sep)` search leaves
//! `index` at its initial `~0 == -1` when every candidate is NaN (a degenerate AABB
//! makes `c2Norms` produce NaN normals), after which C evaluates `p->verts[-1]`, four
//! bytes *before* the struct. This crate performs the same out-of-bounds read.
//!
//! Differential testing against the C `.so` covered ~1.5 M cases — all 46 symbols
//! over structured corpora, random 32-bit patterns (quiet/signalling NaNs of both
//! signs with random payloads, infinities, signed zeros, denormals), random
//! simplices, GJK with and without transforms and with cache round-trips, and
//! ~440 K `omni_manifold` calls across all 16 type pairs. Output is compared as raw
//! bytes, so NaN payloads and `-0.0` are included. Every case in which the C library
//! is a deterministic function of its inputs matches byte for byte.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

pub mod api;
pub mod fp;
pub mod gjk;
pub mod manifold;
pub mod math;
pub mod shapes;
pub mod types;
