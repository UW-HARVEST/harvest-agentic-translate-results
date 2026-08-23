//! Phase C — the `ERRORS.md` rows marked **BIGMEM**: rejections guarded by
//! `JS_STRLIMIT` (1<<28 = 256 MB) and `JS_ARRAYLIMIT` (1<<26 entries = 1 GB of
//! flat array data). They are genuinely reachable from JavaScript, but only by
//! actually materialising hundreds of megabytes, so they live in their own file
//! and are skipped (loudly) when the machine does not have the head-room.
//!
//! Rows covered:
//!   jsstring.c:163  Sp_concat           "invalid string length" (this-string >= 2^28)
//!   jsstring.c:171  Sp_concat           "invalid string length" (accumulated  > 2^28)
//!   jsarray.c:149   Ap_join             "invalid string length"
//!   jsrun.c:149     js_pushstring       "invalid string length" (via js_concat)
//!   jsrun.c:676     jsR_setarrayindex   "array too large"
#![allow(non_snake_case)]

mod common;
use common::*;

/// MemAvailable from /proc/meminfo, in MiB.
fn mem_available_mib() -> u64 {
    let s = std::fs::read_to_string("/proc/meminfo").unwrap_or_default();
    for line in s.lines() {
        if let Some(rest) = line.strip_prefix("MemAvailable:") {
            if let Some(kb) = rest.split_whitespace().next() {
                if let Ok(kb) = kb.parse::<u64>() {
                    return kb / 1024;
                }
            }
        }
    }
    0
}

/// Skip (with a loud message) unless at least `need_mib` MiB are available.
fn require_mem(what: &str, need_mib: u64) -> bool {
    let have = mem_available_mib();
    if have < need_mib {
        eprintln!(
            "SKIPPING {}: needs ~{} MiB of head-room, only {} MiB available",
            what, need_mib, have
        );
        return false;
    }
    true
}

#[track_caller]
fn diff_big(label: &str, need_mib: u64, src: &str) {
    if !require_mem(label, need_mib) {
        return;
    }
    let t0 = std::time::Instant::now();
    let (c, r) = both(|api, _| run_script(api, 0, src));
    eprintln!("  {} took {:?}", label, t0.elapsed());
    assert_eq!(
        c,
        r,
        "DIVERGENCE for {}:\n  script: {}\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
        label,
        src,
        c.0,
        String::from_utf8_lossy(&c.1),
        r.0,
        String::from_utf8_lossy(&r.1)
    );
}

/// Build a string of exactly 2^`pow` bytes by doubling.
fn make_pow2_string(pow: u32) -> String {
    format!(
        "var s='a'; while (s.length < {}) s += s; print('built', s.length);",
        1u64 << pow
    )
}

// ------------------------------------------------------------------ jsstring.c:163
#[test]
fn bigmem_sp_concat_this_string_over_limit() {
    // n = 1 + strlen(this) must exceed JS_STRLIMIT, so `this` needs 2^28 bytes.
    // The check fires before any allocation for the result.
    let src = format!(
        "{} try {{ print(s.concat('x').length) }} catch (e) {{ print('caught', e.name, e.message) }}",
        make_pow2_string(28)
    );
    diff_big("jsstring.c:163 Sp_concat (this >= 2^28)", 1200, &src);
}

// ------------------------------------------------------------------ jsstring.c:171
#[test]
fn bigmem_sp_concat_accumulated_over_limit() {
    // this = 2^27 passes the first check (n = 1 + 2^27); appending the same
    // string pushes n to 1 + 2^28 inside the loop.
    let src = format!(
        "{} try {{ print(s.concat(s).length) }} catch (e) {{ print('caught', e.name, e.message) }}",
        make_pow2_string(27)
    );
    diff_big("jsstring.c:171 Sp_concat (accumulated > 2^28)", 900, &src);
}

// ------------------------------------------------------------------ jsarray.c:149
#[test]
fn bigmem_ap_join_over_limit() {
    // n accumulates: after k=0 n = 2^27; k=1 gives exactly 2^28 (not >);
    // k=2 exceeds it. The three elements share one string object.
    let src = format!(
        "{} var a=[s,s,s]; try {{ print(a.join('').length) }} catch (e) {{ print('caught', e.name, e.message) }}",
        make_pow2_string(27)
    );
    diff_big("jsarray.c:149 Ap_join", 1400, &src);
}

// ------------------------------------------------------------------ jsrun.c:149
#[test]
fn bigmem_js_pushstring_over_limit() {
    // js_concat mallocs strlen(a)+strlen(b)+1 = 2^29+1 and then hands it to
    // js_pushstring, whose strlen check rejects it.
    let src = format!(
        "{} try {{ var t = s + s; print(t.length) }} catch (e) {{ print('caught', e.name, e.message) }}",
        make_pow2_string(28)
    );
    diff_big("jsrun.c:149 js_pushstring (via js_concat)", 2600, &src);
}

// ------------------------------------------------------------------ jsrun.c:676
#[test]
fn bigmem_array_too_large_flat_append() {
    // jsR_setarrayindex rejects newlen > JS_ARRAYLIMIT, but it is only reached
    // for an APPEND (k <= flat_length), so the flat part really has to grow to
    // 2^26 entries * sizeof(js_Value) = 1 GiB before the check can fire.
    let limit = JS_ARRAYLIMIT as i64; // 1 << 26
    let src = format!(
        "var a=[]; try {{ for (var i=0;i<={};++i) a[i]=0; print('no error', a.length) }} \
         catch (e) {{ print('caught', e.name, e.message, a.length) }}",
        limit
    );
    diff_big("jsrun.c:676 jsR_setarrayindex (array too large)", 3500, &src);
}

// ------------------------------------------------------------------ near misses
#[test]
fn bigmem_near_misses_just_inside_the_limits() {
    // Just-inside variants must SUCCEED identically in both libraries, proving
    // the tests above really straddle the boundary.
    if !require_mem("bigmem near misses", 1400) {
        return;
    }
    let scripts = [
        // strlen(this) = 2^27 -> n = 1 + 2^27, well under the limit
        format!("{} print(s.concat('x').length)", make_pow2_string(27)),
        // n reaches exactly 2^28, which is NOT > JS_STRLIMIT
        format!("{} print(s.concat(s).length)", make_pow2_string(27)),
        // join of two halves reaches exactly 2^28
        format!(
            "{} var a=[s,s]; print(a.join('').length)",
            make_pow2_string(27)
        ),
        // flat array append just under JS_ARRAYLIMIT is fine (kept small: the
        // limit itself is exercised by the test above)
        "var a=[]; for (var i=0;i<100000;++i) a[i]=i; print(a.length, a[99999])".to_string(),
        // length may be set to exactly JS_ARRAYLIMIT
        format!("var a=[]; a.length={}; print(a.length)", JS_ARRAYLIMIT),
        format!(
            "var a=[]; try {{ a.length={} }} catch(e) {{ print('caught', e.name, e.message) }} print(a.length)",
            JS_ARRAYLIMIT as i64 + 1
        ),
    ];
    for s in &scripts {
        let (c, r) = both(|api, _| run_script(api, 0, s));
        assert_eq!(
            c,
            r,
            "DIVERGENCE near-miss:\n  script: {}\n  C   : rc={} out={:?}\n  Rust: rc={} out={:?}",
            s,
            c.0,
            String::from_utf8_lossy(&c.1),
            r.0,
            String::from_utf8_lossy(&r.1)
        );
    }
}
