//! Phase B/C extra — the `strlen(mystr) > INT_MAX` code path.
//!
//! `slice()` keeps `len` in a `size_t` but `start`/`stop` in an `int`:
//!
//! ```c
//! size_t len = strlen(mystr);
//! ...
//! } else stop = len;                       /* size_t -> int truncation   */
//! printf("%.*s\n", stop - start, mystr + start);  /* int subtraction      */
//! ```
//!
//! For a string of exactly 2 GiB (`len == 2^31`) the default `stop` truncates to
//! `INT_MIN`, `INT_MAX` becomes a *valid* `start` (it is `len - 1`), and the
//! precision `stop - start` wraps around.  Nothing below `INT_MAX` exercises
//! that, so this row is verified separately: the string is allocated once and
//! handed to both `.so`s by pointer (no copies), and only index pairs whose
//! wrapped precision is small are used, so stdout stays tiny.
//!
//! Requires ~2 GiB of RAM.  If the allocation fails the row reports `skip`
//! instead of a bogus pass.  Set `SKIP_HUGE_STRING=1` to skip deliberately.

#[path = "harness/mod.rs"]
mod harness;

use harness::{Lib, Runner, Session};
use std::ffi::{c_char, c_int};

const LEN: usize = 1usize << 31; // 2_147_483_648 == INT_MAX + 1

/// Cheap fingerprint over a strided sample, used to prove neither library
/// modified the 2 GiB buffer (a full memcmp would dominate the runtime).
fn fingerprint(buf: &[u8]) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    let mut i = 0usize;
    while i < buf.len() {
        h ^= buf[i] as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
        i += 4093;
    }
    // Always include the two ends exactly.
    for &b in buf[..4096].iter().chain(buf[buf.len() - 4096..].iter()) {
        h ^= b as u64;
        h = h.wrapping_mul(0x100_0000_01b3);
    }
    h
}

/// `(label, start, stop, why)` — `None` means a NULL pointer argument.
#[allow(clippy::type_complexity)]
fn cases() -> Vec<(String, Option<i32>, Option<i32>, &'static str)> {
    let max = i32::MAX; // == LEN - 1, a *valid* start at this length
    let mut v: Vec<(String, Option<i32>, Option<i32>, &'static str)> = Vec::new();

    // stop_ptr == NULL: stop = (int)2^31 = INT_MIN, precision = INT_MIN - start
    // wraps to (1 + k) for start = INT_MAX - k.
    for k in 0..9i32 {
        v.push((
            format!("start=INT_MAX-{k} stop=NULL (truncated stop, wrapped precision)"),
            Some(max - k),
            None,
            "prints k+1 bytes",
        ));
    }
    // Ordinary explicit windows at a length that no int can hold.
    v.push((
        "start=100 stop=300".into(),
        Some(100),
        Some(300),
        "normal window",
    ));
    v.push((
        "start=INT_MAX-64 stop=INT_MAX".into(),
        Some(max - 64),
        Some(max),
        "window at the very end",
    ));
    v.push((
        "start=NULL stop=5".into(),
        None,
        Some(5),
        "default start, tiny stop",
    ));
    v.push((
        "start=INT_MAX stop=INT_MAX (equal)".into(),
        Some(max),
        Some(max),
        "E3: stop must come after start",
    ));
    v.push((
        "start=1000 stop=1000 (equal)".into(),
        Some(1000),
        Some(1000),
        "E3",
    ));
    v.push((
        "start=INT_MAX stop=7 (stop < start)".into(),
        Some(max),
        Some(7),
        "E3",
    ));
    // Error paths at len > INT_MAX.
    v.push(("start=-1".into(), Some(-1), None, "E1 (negative -> huge)"));
    v.push((
        "start=INT_MIN".into(),
        Some(i32::MIN),
        None,
        "E1 (INT_MIN -> huge)",
    ));
    v.push((
        "start=0 stop=-1".into(),
        Some(0),
        Some(-1),
        "E2 (negative stop)",
    ));
    v.push((
        "start=0 stop=INT_MIN".into(),
        Some(0),
        Some(i32::MIN),
        "E2",
    ));
    v.push((
        "start=NULL stop=INT_MIN".into(),
        None,
        Some(i32::MIN),
        "E2",
    ));
    v
}

fn main() {
    let mut run = Runner::new("Phase B/C extra — strlen(mystr) > INT_MAX (2 GiB string)");

    run.raw_row("huge-01 len == 2^31 (int truncation + wrapping precision)", |c: &Lib, r: &Lib| {
        if std::env::var_os("SKIP_HUGE_STRING").is_some() {
            return Ok("skipped: SKIP_HUGE_STRING is set".into());
        }
        // Allocate 2^31 + 1 bytes: 2^31 payload bytes plus the NUL terminator.
        let mut buf: Vec<u8> = Vec::new();
        if buf.try_reserve_exact(LEN + 1).is_err() {
            return Ok("skipped: cannot reserve 2 GiB".into());
        }
        buf.resize(LEN + 1, b'A');
        // Distinguishable content at both ends so the printed window is unique.
        for (i, b) in buf[..4096].iter_mut().enumerate() {
            *b = b'a' + (i % 26) as u8;
        }
        let end = LEN - 4096;
        for (i, b) in buf[end..LEN].iter_mut().enumerate() {
            *b = b'0' + (i % 10) as u8;
        }
        buf[LEN] = 0; // strlen(buf) == LEN == 2^31
        let before = fingerprint(&buf[..LEN]);

        let p = buf.as_mut_ptr() as *mut c_char;
        let list = cases();
        let mut results: Vec<Vec<(c_int, Vec<u8>)>> = Vec::new();
        {
            let mut sess = Session::new();
            for lib in [c, r] {
                let mut per_lib = Vec::new();
                for (_, st, sp, _) in &list {
                    let mut s_cell: c_int = st.unwrap_or(0);
                    let mut e_cell: c_int = sp.unwrap_or(0);
                    let spp: *mut c_int = if st.is_some() {
                        &mut s_cell
                    } else {
                        std::ptr::null_mut()
                    };
                    let epp: *mut c_int = if sp.is_some() {
                        &mut e_cell
                    } else {
                        std::ptr::null_mut()
                    };
                    let f = lib.slice;
                    per_lib.push(sess.call(|| unsafe { f(p, spp, epp) }));
                }
                results.push(per_lib);
            }
        }
        let after = fingerprint(&buf[..LEN]);
        drop(buf);

        if before != after {
            return Err("the 2 GiB input buffer was modified by a callee".into());
        }
        let (a, b) = (&results[0], &results[1]);
        let mut diffs = Vec::new();
        for (i, ((ra, oa), (rb, ob))) in a.iter().zip(b.iter()).enumerate() {
            if ra != rb || oa != ob {
                diffs.push(format!(
                    "{}: C ret={ra} out={:?} | Rust ret={rb} out={:?}  ({})",
                    list[i].0,
                    String::from_utf8_lossy(oa),
                    String::from_utf8_lossy(ob),
                    list[i].3
                ));
            }
        }
        if !diffs.is_empty() {
            return Err(diffs.join("\n  "));
        }
        // Sanity: the row must really have taken the truncation path, i.e. the
        // C library must have printed exactly k+1 bytes plus '\n' for the
        // `stop = NULL` cases.  If that ever stops holding, the row is no
        // longer testing what it claims to.
        for k in 0..9usize {
            let (ret, out) = &a[k];
            if *ret != 0 || out.len() != k + 2 || *out.last().unwrap() != b'\n' {
                return Err(format!(
                    "C did not take the expected wrapped-precision path for k={k}: ret={ret} out={:?}",
                    String::from_utf8_lossy(out)
                ));
            }
        }
        Ok(format!("{} cases, buffer unmodified", list.len()))
    });

    run.finish();
}
