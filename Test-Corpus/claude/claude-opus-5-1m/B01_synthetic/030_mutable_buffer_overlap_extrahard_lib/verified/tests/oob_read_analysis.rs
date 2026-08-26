// Analysis + gate for the second (and last) construct whose result is not a
// property of the library: `driver(data, len)` with a `len` larger than the
// caller's `data` buffer.
//
// `driver` does `memcpy(out, data, len * sizeof(int))` with no length validation
// at all, so an oversized `len` copies -- and then prints -- whatever process
// memory happens to follow `data`. That memory is not part of the input:
// `d_oob_01` shows the *same* C `.so`, with the *same* `data` contents and the
// *same* `len`, printing four different results depending only on an unrelated
// earlier `malloc` in the caller.
//
// `d_oob_02`/`d_oob_03` pin down everything about this case that IS well defined
// and require the two libraries to agree on it exactly.

mod common;

use common::*;
use std::collections::BTreeSet;
use std::ffi::c_int;

/// Number of `int`s actually owned by the caller.
const N: usize = 4096;

fn lines(v: &[u8]) -> Vec<&[u8]> {
    if v.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<&[u8]> = v.split(|&b| b == b'\n').collect();
    // Trailing element after the final '\n' is empty; drop it.
    if out.last().map(|l| l.is_empty()).unwrap_or(false) {
        out.pop();
    }
    out
}

/// PROOF: for an oversized `len`, the C library's own output depends on the
/// caller's heap history, so it is not reproducible by any implementation.
#[test]
fn d_oob_01_c_output_depends_on_the_callers_heap_history() {
    let len = (N + 4096) as c_int;
    let mut seen: BTreeSet<Vec<u8>> = BTreeSet::new();
    let mut leaked: Vec<&'static mut [u8]> = Vec::new();
    for pre in [0usize, 8, 24, 64, 200, 512, 1500, 4096] {
        if pre > 0 {
            // Perturb the heap exactly the way an unrelated caller allocation
            // would, then leak it so it stays perturbed.
            leaked.push(Box::leak(vec![0xAAu8; pre].into_boxed_slice()));
        }
        let data: Vec<c_int> = (0..N as c_int).collect();
        let p = data.as_ptr();
        let (disp, out) = run_in_child_capturing(|| unsafe { (c_lib().driver)(p, len) });
        // The in-bounds prefix must never change, whatever the heap looks like.
        if disp == Disposition::Exited(0) {
            let ls = lines(&out);
            assert!(ls.len() >= N, "C truncated the in-bounds part (pre={pre})");
            seen.insert(out[..].to_vec());
        }
    }
    println!(
        "distinct outputs produced by the SAME C .so for driver(data[{N}], {len}), \
         varying only an unrelated caller malloc: {}",
        seen.len()
    );
    assert!(
        seen.len() > 1,
        "expected the C library's out-of-range read to expose caller heap state \
         (it printed the same bytes every time); if this ever becomes stable the \
         ERRORS.md rationale for the out-of-range-`len` rows must be revisited"
    );
}

/// The comparable half. Neither *what* an out-of-range read yields (`d_oob_01`)
/// nor *whether it faults* (`d_oob_04`) is decided by the input, so the gate is
/// the part the C source does specify: whatever a library manages to print, its
/// first `N` lines -- the ones produced from in-bounds data -- must be exactly
/// `x*x + x` formatted with `%d\n`, identical between the two libraries and
/// identical to the reference model.
#[test]
fn d_oob_02_in_bounds_prefix_and_line_count_match() {
    let data: Vec<c_int> = {
        let mut rng = Rng::new(4242);
        let mut v = vec![0 as c_int; N];
        rng.fill_small(&mut v);
        v
    };
    let p = data.as_ptr();
    let model = model_driver_stdout(&data);
    let ml = lines(&model);
    assert_eq!(ml.len(), N);

    let mut compared = 0usize;
    for &extra in &[1usize, 2, 3, 7, 16, 64, 1024, 4096] {
        let len = (N + extra) as c_int;
        let (cd, co) = run_in_child_capturing(|| unsafe { (c_lib().driver)(p, len) });
        let (rd, ro) = run_in_child_capturing(|| unsafe { (rust_lib().driver)(p, len) });
        let cl = lines(&co);
        let rl = lines(&ro);
        println!("len={len}: C {cd} ({} lines), Rust {rd} ({} lines)", cl.len(), rl.len());

        // Whatever each library printed, its in-bounds part must be correct.
        if cl.len() >= N {
            assert_eq!(&cl[..N], &ml[..], "C's in-bounds prefix is wrong (len={len})");
        }
        if rl.len() >= N {
            assert_eq!(&rl[..N], &ml[..], "Rust's in-bounds prefix is wrong (len={len})");
        }
        if cl.len() >= N && rl.len() >= N {
            assert_eq!(
                &cl[..N],
                &rl[..N],
                "the in-bounds prefixes disagree (len={len})"
            );
            compared += 1;
        }
        // When a library survives, it must have printed exactly `len` lines.
        if cd == Disposition::Exited(0) {
            assert_eq!(cl.len(), len as usize, "C printed {} lines for len={len}", cl.len());
        }
        if rd == Disposition::Exited(0) {
            assert_eq!(rl.len(), len as usize, "Rust printed {} lines for len={len}", rl.len());
        }
    }
    assert!(
        compared > 0,
        "no out-of-range length let both libraries print their in-bounds prefix, \
         so nothing was actually compared"
    );
}

/// PROOF: for an oversized `len`, whether the C library *faults at all* is decided
/// by what happens to be mapped after the caller's buffer, not by the input. The
/// same C `.so` is called with the same `len` and the same 4096 element values,
/// once with a `PROT_NONE` guard page right after the buffer and once with
/// ordinary readable memory there.
///
/// Any implementation must put `driver`'s output buffer *somewhere*, and the C's
/// choice (the stack) is not available to a Rust translation that has to survive
/// the `printf` calls in `inner`; so this outcome is not reproducible either.
#[test]
fn d_oob_04_c_fault_depends_on_what_follows_the_buffer() {
    let len = (N + 1024) as c_int; // read 4 KiB past the end
    let values: Vec<c_int> = {
        let mut rng = Rng::new(4244);
        let mut v = vec![0 as c_int; N];
        rng.fill_small(&mut v);
        v
    };

    // (a) the buffer's last element ends exactly at a PROT_NONE guard page
    let mut guarded = GuardedInts::new(N);
    guarded.as_mut_slice().copy_from_slice(&values);
    let gp = guarded.ptr() as *const c_int;
    let d_guarded = run_in_child(|| unsafe { (c_lib().driver)(gp, len) });

    // (b) byte-for-byte the same input, but followed by readable memory
    let bytes = (N + 4096) * std::mem::size_of::<c_int>();
    let m = unsafe {
        libc::mmap(
            std::ptr::null_mut(),
            bytes,
            libc::PROT_READ | libc::PROT_WRITE,
            libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
            -1,
            0,
        )
    };
    assert!(m != libc::MAP_FAILED, "mmap failed");
    let mp = m as *mut c_int;
    unsafe { std::ptr::copy_nonoverlapping(values.as_ptr(), mp, N) };
    let d_mapped = run_in_child(|| unsafe { (c_lib().driver)(mp as *const c_int, len) });

    println!("the SAME C .so, same {N} input values, same len={len}:");
    println!("  buffer followed by a PROT_NONE guard page -> {d_guarded}");
    println!("  buffer followed by readable memory        -> {d_mapped}");
    assert_ne!(
        d_guarded, d_mapped,
        "expected whether the C library faults on an out-of-range read to be \
         decided by the surrounding mappings rather than by the input; if this \
         ever stops being true the ERRORS.md rationale for rows 26-27 must be \
         revisited"
    );
    assert_eq!(
        d_guarded,
        Disposition::Signaled(libc::SIGSEGV),
        "a guarded overrun must fault"
    );
    assert_eq!(
        d_mapped,
        Disposition::Exited(0),
        "an overrun into readable memory must succeed"
    );
    unsafe { libc::munmap(m, bytes) };
}

/// The same for the low-level entry point. `fma_array` has no `memcpy`, it just
/// walks off the end of the buffers, so beyond the in-bounds prefix it reads and
/// *writes* unspecified memory. The in-bounds prefix, and whether the call
/// survives, must still agree.
#[test]
fn d_oob_03_fma_array_in_bounds_prefix_matches() {
    let mut rng = Rng::new(4243);
    for &extra in &[1usize, 2, 8, 64] {
        let n = 256usize;
        let len = (n + extra) as c_int;
        // 4 KiB of slack after each array so the overrun stays inside the mapping
        // (a guarded buffer is covered separately by err_09).
        let run = |lib: &Lib| -> (Disposition, Vec<c_int>) {
            let mut buf = vec![0 as c_int; 4 * (n + 2048)];
            let mut r = Rng::new(99);
            r.fill_full(&mut buf);
            let base = buf.as_mut_ptr();
            let d = run_in_child(|| unsafe {
                (lib.fma_array)(
                    base,
                    base.add(n + 1024),
                    base.add(2 * (n + 1024)),
                    base.add(3 * (n + 1024)),
                    len,
                )
            });
            // Re-run in-process to inspect the destination prefix.
            let mut buf2 = vec![0 as c_int; 4 * (n + 2048)];
            let mut r2 = Rng::new(99);
            r2.fill_full(&mut buf2);
            let b2 = buf2.as_mut_ptr();
            unsafe {
                (lib.fma_array)(
                    b2,
                    b2.add(n + 1024),
                    b2.add(2 * (n + 1024)),
                    b2.add(3 * (n + 1024)),
                    len,
                )
            };
            (d, buf2[..n].to_vec())
        };
        let (cd, cpre) = run(c_lib());
        let (rd, rpre) = run(rust_lib());
        let _ = rng.next_u64();
        assert_eq!(cd, rd, "fma_array(len={len}) over {n} elements: C {cd}, Rust {rd}");
        assert_eq!(
            cpre, rpre,
            "fma_array in-bounds prefix mismatch for len={len}"
        );
    }
}
