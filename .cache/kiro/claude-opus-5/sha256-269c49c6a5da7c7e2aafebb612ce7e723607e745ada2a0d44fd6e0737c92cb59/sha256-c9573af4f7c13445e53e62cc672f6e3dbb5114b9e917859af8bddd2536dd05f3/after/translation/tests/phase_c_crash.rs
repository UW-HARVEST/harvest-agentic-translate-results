//! Phase C — ERRORS.md rows 34, 35, 37: the paths where the C's behaviour is
//! undefined rather than a defined error code.
//!
//! * Row 34 — out-of-range `C2_TYPE` across the FFI boundary. A C enum parameter
//!   is a plain `int`, so this is a real input. The C `switch` has no `default`
//!   and the function has no final `return`, so `%eax` is left as the caller
//!   left it. Both libraries are called **from the identical machine call site**
//!   (one indirect call in one loop), so the incoming `%eax` is the same for
//!   both and the comparison is meaningful.
//! * Rows 35 & 37 — null-pointer dereferences that must SIGSEGV. Tested by
//!   re-executing this test binary as a child process and comparing the signal.

#![allow(non_snake_case)]

mod common;
use common::*;
use std::ffi::c_void;

const CRASH_ENV: &str = "GEN_RAY_PHASE_C_CRASH";

fn ray() -> c2Ray {
    c2Ray {
        p: c2v { x: -10.0, y: 0.0 },
        d: c2v { x: 1.0, y: 0.0 },
        t: 100.0,
    }
}

fn circle() -> c2Circle {
    c2Circle {
        p: c2v { x: 0.0, y: 0.0 },
        r: 3.0,
    }
}

fn capsule() -> c2Capsule {
    c2Capsule {
        a: c2v { x: 0.0, y: -5.0 },
        b: c2v { x: 0.0, y: 5.0 },
        r: 2.0,
    }
}

fn aabb() -> c2AABB {
    c2AABB {
        min: c2v { x: -2.0, y: -2.0 },
        max: c2v { x: 2.0, y: 2.0 },
    }
}

/// A naked trampoline that seeds `%eax` with a known sentinel and then TAIL-JUMPS
/// to `f`, so the callee sees an exactly known incoming `%eax`.
///
/// This makes row 34 deterministic instead of "whatever the compiler happened to
/// leave in `%eax` at this particular call site". The C's fall-through path is
/// `leave; ret` with `%eax` never written, so it must return the sentinel
/// unchanged — and so must the Rust.
///
/// System V AMD64: `A` is MEMORY class and sits at `[rsp+8]` on entry (right
/// above the return address), which is exactly where the callee expects it after
/// a tail `jmp`. `B` is in `rdi`, `typeB` in `esi`, `out` in `rdx`, and the
/// trailing `f` argument lands in `rcx`.
#[cfg(target_arch = "x86_64")]
#[unsafe(naked)]
unsafe extern "C" fn eax_seeded_call(
    _A: c2Ray,
    _B: *const c_void,
    _typeB: i32,
    _out: *mut c2Raycast,
    _f: usize,
) -> i32 {
    core::arch::naked_asm!(
        "mov eax, 0x5A5A5A5A",
        "jmp rcx",
    );
}

const EAX_SENTINEL: i32 = 0x5A5A_5A5Au32 as i32;

/// Row 34 — out-of-range enum values. Every value with no valid variant, from
/// both directions and at the `int` extremes.
#[test]
fn err_34_castray_out_of_range_enum() {
    let l = libs();
    let mut d = Diff::new("row34 c2CastRay out-of-range C2_TYPE");

    let bad: Vec<i32> = vec![
        -1,
        3,
        4,
        5,
        99,
        127,
        128,
        255,
        256,
        -2,
        -100,
        0x0001_0000,
        0x7fff_ffff,
        i32::MIN,
        i32::MAX,
        i32::MIN + 1,
        // Values whose low byte looks valid but whose full int does not --
        // the C compares the full `int`, so these must NOT dispatch.
        0x0000_0100,
        0x0001_0001,
        0x0100_0002,
        -2_147_483_646,
    ];

    let payloads: Vec<Vec<u8>> = vec![
        {
            let c = circle();
            let mut v = vec![0u8; 12];
            unsafe { std::ptr::copy_nonoverlapping(&c as *const _ as *const u8, v.as_mut_ptr(), 12) };
            v
        },
        {
            let c = capsule();
            let mut v = vec![0u8; 20];
            unsafe { std::ptr::copy_nonoverlapping(&c as *const _ as *const u8, v.as_mut_ptr(), 20) };
            v
        },
    ];

    for ty in bad {
        for payload in &payloads {
            let mut rets = [0i32; 2];
            let mut outs = [POISON; 2];
            let targets = [l.c.c2CastRay as usize, l.r.c2CastRay as usize];
            for i in 0..2 {
                rets[i] = unsafe {
                    eax_seeded_call(
                        ray(),
                        payload.as_ptr() as *const c_void,
                        ty,
                        &mut outs[i],
                        targets[i],
                    )
                };
            }
            // Both must return the seeded sentinel: the C because its
            // fall-through path never writes %eax, the Rust because its naked
            // dispatch shim reproduces that.
            d.check(rets[0] == EAX_SENTINEL, || {
                format!(
                    "C: c2CastRay(typeB={ty}) returned {} (0x{:08x}), expected the seeded \
                     %eax sentinel 0x5A5A5A5A -- the C's fall-through path must not write %eax",
                    rets[0], rets[0]
                )
            });
            d.check(rets[0] == rets[1], || {
                format!(
                    "c2CastRay(typeB={ty}) mismatch: C -> {} (0x{:08x}) but Rust -> {} (0x{:08x})",
                    rets[0], rets[0], rets[1], rets[1]
                )
            });
            // Neither library may write the out-parameter on this path.
            d.check(rc_eq(outs[0], POISON), || {
                format!("C wrote *out for typeB={ty}: {}", fmt_rc(outs[0]))
            });
            d.check(rc_eq(outs[1], POISON), || {
                format!("Rust wrote *out for typeB={ty}: {}", fmt_rc(outs[1]))
            });
        }
    }

    // Sanity: through the SAME trampoline, the three valid modes must overwrite
    // the sentinel with a real 0/1 result. Without this the test above could
    // pass for a function that ignored `typeB` entirely.
    let circ = circle();
    let mut cp = vec![0u8; 12];
    unsafe { std::ptr::copy_nonoverlapping(&circ as *const _ as *const u8, cp.as_mut_ptr(), 12) };
    for ty in [C2_TYPE_CIRCLE, C2_TYPE_AABB, C2_TYPE_CAPSULE] {
        let targets = [l.c.c2CastRay as usize, l.r.c2CastRay as usize];
        let mut rets = [0i32; 2];
        let mut outs = [POISON; 2];
        for i in 0..2 {
            rets[i] = unsafe {
                eax_seeded_call(
                    ray(),
                    cp.as_ptr() as *const c_void,
                    ty,
                    &mut outs[i],
                    targets[i],
                )
            };
        }
        d.check(rets[0] != EAX_SENTINEL, || {
            format!("C: valid typeB={ty} did not overwrite the sentinel")
        });
        d.check_ray(rets[0], outs[0], rets[1], outs[1], || {
            format!("c2CastRay(valid typeB={ty}) via the eax-seeding trampoline")
        });
    }
    eprintln!("    row34: both libraries returned the seeded %eax sentinel 0x5A5A5A5A unchanged");
    d.finish();
}

/// Row 34 (continued) — the three VALID enum values must still dispatch, and the
/// boundary values `2` (last valid) and `3` (first invalid) must behave
/// differently. This guards against a shim that rejects too much.
#[test]
fn err_34b_castray_enum_boundary() {
    let l = libs();
    let mut d = Diff::new("row34b c2CastRay enum boundary 2 vs 3");

    let cap = capsule();
    let mut payload = vec![0u8; 20];
    unsafe { std::ptr::copy_nonoverlapping(&cap as *const _ as *const u8, payload.as_mut_ptr(), 20) };

    // typeB == 2 (CAPSULE, last valid) must dispatch and write *out.
    let mut co = POISON;
    let mut ro = POISON;
    let cr = unsafe { (l.c.c2CastRay)(ray(), payload.as_ptr() as *const c_void, 2, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray(), payload.as_ptr() as *const c_void, 2, &mut ro) };
    d.check_ray(cr, co, rr, ro, || "c2CastRay(typeB=2) must dispatch".into());
    assert!(
        !rc_eq(co, POISON),
        "typeB=2 must reach c2RaytoCapsule, which always writes *out"
    );

    // typeB == 0 and 1 must dispatch too.
    let circ = circle();
    let mut cp = vec![0u8; 12];
    unsafe { std::ptr::copy_nonoverlapping(&circ as *const _ as *const u8, cp.as_mut_ptr(), 12) };
    let mut co = POISON;
    let mut ro = POISON;
    let cr = unsafe { (l.c.c2CastRay)(ray(), cp.as_ptr() as *const c_void, 0, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray(), cp.as_ptr() as *const c_void, 0, &mut ro) };
    d.check_ray(cr, co, rr, ro, || "c2CastRay(typeB=0)".into());
    assert_eq!(cr, 1, "typeB=0 with a hitting circle must return 1");

    let bx = aabb();
    let mut bp = vec![0u8; 16];
    unsafe { std::ptr::copy_nonoverlapping(&bx as *const _ as *const u8, bp.as_mut_ptr(), 16) };
    let mut co = POISON;
    let mut ro = POISON;
    let cr = unsafe { (l.c.c2CastRay)(ray(), bp.as_ptr() as *const c_void, 1, &mut co) };
    let rr = unsafe { (l.r.c2CastRay)(ray(), bp.as_ptr() as *const c_void, 1, &mut ro) };
    d.check_ray(cr, co, rr, ro, || "c2CastRay(typeB=1)".into());
    assert_eq!(cr, 1, "typeB=1 with a hitting box must return 1");

    // typeB == 3 must NOT dispatch: *out untouched in both libraries.
    let mut co = POISON;
    let mut ro = POISON;
    let _ = unsafe { (l.c.c2CastRay)(ray(), payload.as_ptr() as *const c_void, 3, &mut co) };
    let _ = unsafe { (l.r.c2CastRay)(ray(), payload.as_ptr() as *const c_void, 3, &mut ro) };
    d.check(rc_eq(co, POISON), || "C dispatched on typeB=3".into());
    d.check(rc_eq(ro, POISON), || "Rust dispatched on typeB=3".into());
    d.finish();
}

/// Row 36 — `out == NULL` on a path where it is never dereferenced. `c2RaytoCircle`
/// and `c2RaytoAABB` only touch `*out` on a hit, so a guaranteed miss with a null
/// out-pointer must return 0 from BOTH libraries without crashing.
#[test]
fn err_36_null_out_no_hit() {
    let l = libs();
    let mut d = Diff::new("row36 null out-param, guaranteed miss");

    // Circle far off the ray line -> `disc < 0` -> returns before touching *out.
    let far_circle = c2Circle {
        p: c2v { x: 0.0, y: 1000.0 },
        r: 1.0,
    };
    let cr = unsafe { (l.c.c2RaytoCircle)(ray(), far_circle, std::ptr::null_mut()) };
    let rr = unsafe { (l.r.c2RaytoCircle)(ray(), far_circle, std::ptr::null_mut()) };
    d.check_i(cr, rr, || "c2RaytoCircle(out=NULL, miss)".into());
    assert_eq!(cr, 0);

    // Box far off the ray -> bbox reject -> returns before touching *out.
    let far_box = c2AABB {
        min: c2v { x: 0.0, y: 1000.0 },
        max: c2v { x: 1.0, y: 1001.0 },
    };
    let cr = unsafe { (l.c.c2RaytoAABB)(ray(), far_box, std::ptr::null_mut()) };
    let rr = unsafe { (l.r.c2RaytoAABB)(ray(), far_box, std::ptr::null_mut()) };
    d.check_i(cr, rr, || "c2RaytoAABB(out=NULL, miss)".into());
    assert_eq!(cr, 0);

    // Same via the dispatcher.
    let mut cp = vec![0u8; 12];
    unsafe {
        std::ptr::copy_nonoverlapping(&far_circle as *const _ as *const u8, cp.as_mut_ptr(), 12)
    };
    let cr = unsafe {
        (l.c.c2CastRay)(ray(), cp.as_ptr() as *const c_void, C2_TYPE_CIRCLE, std::ptr::null_mut())
    };
    let rr = unsafe {
        (l.r.c2CastRay)(ray(), cp.as_ptr() as *const c_void, C2_TYPE_CIRCLE, std::ptr::null_mut())
    };
    d.check_i(cr, rr, || "c2CastRay(CIRCLE, out=NULL, miss)".into());

    // And an out-of-range enum with a null out-param: neither may dereference it.
    let mut rets = [0i32; 2];
    for (i, f) in [l.c.c2CastRay, l.r.c2CastRay].into_iter().enumerate() {
        rets[i] = unsafe {
            f(ray(), cp.as_ptr() as *const c_void, 7, std::ptr::null_mut())
        };
    }
    d.check_i(rets[0], rets[1], || {
        "c2CastRay(typeB=7, out=NULL) must not dereference out".into()
    });
    d.finish();
}

// ---------------------------------------------------------------------------
// Rows 35 & 37 — SIGSEGV parity, checked in a child process.
// ---------------------------------------------------------------------------

/// The crashing bodies, selected by the `GEN_RAY_PHASE_C_CRASH` env var when
/// this binary re-executes itself.
fn run_crash_case(case: &str) -> ! {
    let l = libs();
    let which = &case[..1];
    let lib = if which == "c" { &l.c } else { &l.r };
    let body = &case[2..];
    unsafe {
        match body {
            // Row 37 -- c2RaytoCapsule writes *out unconditionally at the top,
            // so a null out-pointer crashes even though the ray misses.
            "capsule_null_out" => {
                let far = c2Capsule {
                    a: c2v { x: 0.0, y: 1000.0 },
                    b: c2v { x: 0.0, y: 1001.0 },
                    r: 0.5,
                };
                let r = (lib.c2RaytoCapsule)(ray(), far, std::ptr::null_mut());
                println!("NO_CRASH ret={r}");
            }
            // Row 35 -- c2CastRay dereferences the shape pointer with no check.
            "castray_null_shape" => {
                let mut out = POISON;
                let r = (lib.c2CastRay)(ray(), std::ptr::null(), C2_TYPE_CIRCLE, &mut out);
                println!("NO_CRASH ret={r}");
            }
            "castray_null_shape_aabb" => {
                let mut out = POISON;
                let r = (lib.c2CastRay)(ray(), std::ptr::null(), C2_TYPE_AABB, &mut out);
                println!("NO_CRASH ret={r}");
            }
            "castray_null_shape_capsule" => {
                let mut out = POISON;
                let r = (lib.c2CastRay)(ray(), std::ptr::null(), C2_TYPE_CAPSULE, &mut out);
                println!("NO_CRASH ret={r}");
            }
            // Row 36 companion -- a HIT with a null out-pointer does crash.
            "circle_null_out_hit" => {
                let r = (lib.c2RaytoCircle)(ray(), circle(), std::ptr::null_mut());
                println!("NO_CRASH ret={r}");
            }
            "aabb_null_out_hit" => {
                let r = (lib.c2RaytoAABB)(ray(), aabb(), std::ptr::null_mut());
                println!("NO_CRASH ret={r}");
            }
            // gen_ray with a null cast2: the capsule leg always writes.
            "gen_ray_null_cast2" => {
                let mut o1 = POISON;
                let mut o3 = POISON;
                let r = (lib.gen_ray)(
                    &mut o1,
                    std::ptr::null_mut(),
                    &mut o3,
                    10.0, 0.0, -10.0, 0.0,
                    0.0, 0.0, 3.0,
                    0.0, -5.0, 0.0, 5.0, 2.0,
                    -2.0, -2.0, 2.0, 2.0,
                );
                println!("NO_CRASH ret={r}");
            }
            other => panic!("unknown crash case {other}"),
        }
    }
    std::process::exit(0);
}

/// Runs one crash case in a child process and returns `(exit_code, signal)`.
fn spawn_case(case: &str) -> (Option<i32>, Option<i32>) {
    use std::os::unix::process::ExitStatusExt;
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(exe)
        .arg("--exact")
        .arg("crash_child_entry")
        .arg("--nocapture")
        .env(CRASH_ENV, case)
        .output()
        .expect("spawn child");
    (out.status.code(), out.status.signal())
}

/// The child-process entry point. Does nothing unless the env var is set, so it
/// is a harmless no-op during a normal test run.
#[test]
fn crash_child_entry() {
    if let Ok(case) = std::env::var(CRASH_ENV) {
        run_crash_case(&case);
    }
}

/// Rows 35 & 37 — every null-pointer dereference must produce the SAME fatal
/// signal from the C and the Rust library.
#[test]
fn err_35_37_null_pointer_crash_parity() {
    let cases = [
        ("capsule_null_out", "row37 c2RaytoCapsule(out=NULL) -- writes *out unconditionally"),
        ("castray_null_shape", "row35 c2CastRay(B=NULL, CIRCLE)"),
        ("castray_null_shape_aabb", "row35 c2CastRay(B=NULL, AABB)"),
        ("castray_null_shape_capsule", "row35 c2CastRay(B=NULL, CAPSULE)"),
        ("circle_null_out_hit", "row36 c2RaytoCircle(out=NULL) on a HIT"),
        ("aabb_null_out_hit", "row36 c2RaytoAABB(out=NULL) on a HIT"),
        ("gen_ray_null_cast2", "row41 gen_ray(cast2=NULL)"),
    ];
    let mut failures = Vec::new();
    for (case, desc) in cases {
        let c = spawn_case(&format!("c:{case}"));
        let r = spawn_case(&format!("r:{case}"));
        eprintln!("    {desc}: C={c:?} Rust={r:?}");
        if c != r {
            failures.push(format!(
                "{desc}: C exited {c:?} but Rust exited {r:?}"
            ));
        }
        // Every one of these must actually be fatal in the C -- otherwise the
        // ERRORS.md row is wrong and the test proves nothing.
        if c.1.is_none() {
            failures.push(format!(
                "{desc}: expected the C to die from a signal, got exit code {:?}",
                c.0
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
