//! Phase C — differential tests for `pcre2_substring.c`.
//!
//! Every public entry point of `pcre2_substring.c` is driven in BOTH the C and
//! the Rust library through the `.so` exports and every observable is compared:
//! the return code, the value written through every out-parameter, and the bytes
//! of every buffer / heap block that is produced.
//!
//! Rules obeyed throughout:
//!   * buffers that are compared wholesale are pre-filled with the 0xAA sentinel
//!     in BOTH libraries, so no uninitialised memory is ever inspected;
//!   * scalar out-parameters are pre-set to the same sentinel, so "not written"
//!     is itself an observable that is compared;
//!   * ovector pairs are only read for the range PCRE2 defines;
//!   * pointers returned by `pcre2_substring_nametable_scan` are converted to
//!     OFFSETS relative to the pattern's name table before being compared.
mod common;

use common::diff::*;
use common::*;
use std::ffi::c_void;

const SEED: u64 = 0x5B57_2170_BEEFu64;

// ------------------------------------------------------------------ constants
/// `PCRE2_ERROR_DFA_UFUNC` — pcre2.h line 405.
const ERR_DFA_UFUNC: i32 = -41;
/// `PCRE2_INFO_CAPTURECOUNT` — pcre2.h line 450.
const INFO_CAPTURECOUNT: u32 = 4;
const INFO_NAMECOUNT: u32 = 17;
const INFO_NAMEENTRYSIZE: u32 = 18;
const INFO_NAMETABLE: u32 = 19;

/// Sentinel written into every `PCRE2_SIZE` out-parameter before the call.
const SENT: usize = 0xAAAA_AAAA_AAAA_AAAAu64 as usize;

// -------------------------------------------------------------- info helpers
unsafe fn info_u32(api: &Api, code: *mut c_void, what: u32) -> u32 {
    let mut v: u32 = 0;
    let rc = (api.pattern_info)(code, what, &mut v as *mut _ as *mut c_void);
    assert_eq!(rc, 0, "{}: pattern_info({}) failed", api.name, what);
    v
}

/// `(name_count, name_entry_size, name_table_ptr)`.
unsafe fn name_table(api: &Api, code: *mut c_void) -> (u32, u32, *const u8) {
    let cnt = info_u32(api, code, INFO_NAMECOUNT);
    let esz = info_u32(api, code, INFO_NAMEENTRYSIZE);
    let mut p: *const u8 = std::ptr::null();
    let rc = (api.pattern_info)(code, INFO_NAMETABLE, &mut p as *mut _ as *mut c_void);
    assert_eq!(rc, 0, "{}: pattern_info(NAMETABLE) failed", api.name);
    (cnt, esz, p)
}

/// The names actually present in the compiled pattern, as NUL-terminated byte
/// vectors (ready to hand to the `_byname` functions).
unsafe fn names_in_pattern(api: &Api, code: *mut c_void) -> Vec<Vec<u8>> {
    let (cnt, esz, tbl) = name_table(api, code);
    let mut out = Vec::new();
    for i in 0..cnt as usize {
        // entry = 2 bytes group number (IMM2_SIZE) then the zero-terminated name
        let entry = tbl.add(i * esz as usize + 2);
        let mut v = Vec::new();
        let mut q = entry;
        while *q != 0 {
            v.push(*q);
            q = q.add(1);
        }
        v.push(0);
        if !out.contains(&v) {
            out.push(v);
        }
    }
    out
}

/// Names that certainly do NOT exist, plus the empty name. All NUL-terminated.
fn bogus_names() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();
    for s in [
        "",
        "zzz_absent",
        "n",
        "a",
        "A",
        "one",
        "nosuchgroup",
        "0",
        "\u{00e9}",
        "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
    ] {
        let mut b = s.as_bytes().to_vec();
        b.push(0);
        v.push(b);
    }
    v
}

fn show(b: &[u8]) -> String {
    // strip the trailing NUL for readability
    let s = if b.last() == Some(&0) { &b[..b.len() - 1] } else { b };
    format!("{:?}", String::from_utf8_lossy(s))
}

// ---------------------------------------------------------- match-data driver
unsafe fn mk_md(api: &Api, code: *mut c_void, ovecsize: Option<u32>) -> *mut c_void {
    let md = match ovecsize {
        Some(n) => (api.match_data_create)(n, std::ptr::null_mut()),
        None => (api.match_data_create_from_pattern)(code, std::ptr::null_mut()),
    };
    assert!(!md.is_null(), "{}: match_data_create failed", api.name);
    md
}

unsafe fn run_engine(
    api: &Api,
    code: *mut c_void,
    md: *mut c_void,
    subject: &[u8],
    startoffset: usize,
    mopts: u32,
    engine: Engine,
    ws: &mut [i32],
) -> i32 {
    match engine {
        Engine::Interpreter => (api.do_match)(
            code,
            subject.as_ptr(),
            subject.len(),
            startoffset,
            mopts,
            md,
            std::ptr::null_mut(),
        ),
        Engine::JitMatch => (api.jit_match)(
            code,
            subject.as_ptr(),
            subject.len(),
            startoffset,
            mopts,
            md,
            std::ptr::null_mut(),
        ),
        Engine::Dfa => (api.dfa_match)(
            code,
            subject.as_ptr(),
            subject.len(),
            startoffset,
            mopts,
            md,
            std::ptr::null_mut(),
            ws.as_mut_ptr(),
            ws.len(),
        ),
    }
}

/// Number of ovector pairs PCRE2 guarantees to have written.
fn defined_pairs(rc: i32, count: u32) -> usize {
    if rc > 0 {
        (rc as usize).min(count as usize)
    } else if rc == 0 {
        count as usize
    } else if rc == ERR_PARTIAL {
        1.min(count as usize)
    } else {
        0
    }
}

unsafe fn read_mark(api: &Api, md: *mut c_void) -> Option<Vec<u8>> {
    let p = (api.get_mark)(md);
    if p.is_null() {
        return None;
    }
    let mut v = Vec::new();
    let mut q = p;
    while *q != 0 {
        v.push(*q);
        q = q.add(1);
    }
    Some(v)
}

// ----------------------------------------------------------------- probes
/// `pcre2_substring_length_bynumber` — rc plus the written size.
unsafe fn probe_len_bynumber(api: &Api, md: *mut c_void, n: u32) -> (i32, usize) {
    let mut sz = SENT;
    let rc = (api.substring_length_bynumber)(md, n, &mut sz);
    (rc, sz)
}

unsafe fn probe_len_byname(api: &Api, md: *mut c_void, name: &[u8]) -> (i32, usize) {
    let mut sz = SENT;
    let rc = (api.substring_length_byname)(md, name.as_ptr(), &mut sz);
    (rc, sz)
}

/// `pcre2_substring_copy_bynumber`. The buffer is `bufsize + 8` bytes of 0xAA so
/// that the guard area detects any over-write; the whole thing is compared.
unsafe fn probe_copy_bynumber(
    api: &Api,
    md: *mut c_void,
    n: u32,
    bufsize: usize,
) -> (i32, usize, Vec<u8>) {
    let mut buf = vec![0xAAu8; bufsize + 8];
    let mut sz = bufsize;
    let rc = (api.substring_copy_bynumber)(md, n, buf.as_mut_ptr(), &mut sz);
    (rc, sz, buf)
}

unsafe fn probe_copy_byname(
    api: &Api,
    md: *mut c_void,
    name: &[u8],
    bufsize: usize,
) -> (i32, usize, Vec<u8>) {
    let mut buf = vec![0xAAu8; bufsize + 8];
    let mut sz = bufsize;
    let rc = (api.substring_copy_byname)(md, name.as_ptr(), buf.as_mut_ptr(), &mut sz);
    (rc, sz, buf)
}

/// `pcre2_substring_get_bynumber` — rc, written size and the heap bytes
/// (including the terminating zero). The block is freed with
/// `pcre2_substring_free`.
unsafe fn probe_get_bynumber(
    api: &Api,
    md: *mut c_void,
    n: u32,
) -> (i32, usize, Option<Vec<u8>>) {
    let mut p: *mut u8 = std::ptr::null_mut();
    let mut sz = SENT;
    let rc = (api.substring_get_bynumber)(md, n, &mut p, &mut sz);
    if rc == 0 {
        assert!(!p.is_null(), "{}: get_bynumber({}) rc=0 but NULL ptr", api.name, n);
        let v = std::slice::from_raw_parts(p, sz + 1).to_vec();
        (api.substring_free)(p);
        (rc, sz, Some(v))
    } else {
        if !p.is_null() {
            (api.substring_free)(p);
        }
        (rc, sz, None)
    }
}

unsafe fn probe_get_byname(
    api: &Api,
    md: *mut c_void,
    name: &[u8],
) -> (i32, usize, Option<Vec<u8>>) {
    let mut p: *mut u8 = std::ptr::null_mut();
    let mut sz = SENT;
    let rc = (api.substring_get_byname)(md, name.as_ptr(), &mut p, &mut sz);
    if rc == 0 {
        assert!(!p.is_null(), "{}: get_byname rc=0 but NULL ptr", api.name);
        let v = std::slice::from_raw_parts(p, sz + 1).to_vec();
        (api.substring_free)(p);
        (rc, sz, Some(v))
    } else {
        if !p.is_null() {
            (api.substring_free)(p);
        }
        (rc, sz, None)
    }
}

/// `pcre2_substring_nametable_scan` with both pointers supplied. The returned
/// pointers are converted to byte OFFSETS from the start of the name table so
/// that they are comparable between the two libraries.
unsafe fn probe_nametable_scan(
    api: &Api,
    code: *mut c_void,
    name: &[u8],
) -> (i32, Option<(isize, isize)>) {
    let (_cnt, _esz, tbl) = name_table(api, code);
    let mut first: SPTR = std::ptr::null();
    let mut last: SPTR = std::ptr::null();
    let rc = (api.substring_nametable_scan)(code, name.as_ptr(), &mut first, &mut last);
    if rc >= 0 {
        let off = (
            first as isize - tbl as isize,
            last as isize - tbl as isize,
        );
        (rc, Some(off))
    } else {
        (rc, None)
    }
}

/// `pcre2_substring_list_get` with a lengths vector — every string's bytes and
/// every length are captured; the block is freed with
/// `pcre2_substring_list_free`.
unsafe fn probe_list_get(
    api: &Api,
    md: *mut c_void,
) -> (i32, usize, Vec<Vec<u8>>, Vec<usize>) {
    let mut list: *mut *mut u8 = std::ptr::null_mut();
    let mut lens: *mut usize = std::ptr::null_mut();
    let rc = (api.substring_list_get)(md, &mut list, &mut lens);
    if rc != 0 {
        return (rc, 0, Vec::new(), Vec::new());
    }
    assert!(!list.is_null(), "{}: list_get rc=0 but NULL list", api.name);
    assert!(!lens.is_null(), "{}: list_get rc=0 but NULL lengths", api.name);
    let mut n = 0usize;
    while !(*list.add(n)).is_null() {
        n += 1;
        assert!(n < 100_000, "{}: runaway substring list", api.name);
    }
    let mut strings = Vec::with_capacity(n);
    let mut lengths = Vec::with_capacity(n);
    for i in 0..n {
        let l = *lens.add(i);
        lengths.push(l);
        // +1 for the zero terminator the C code always writes
        strings.push(std::slice::from_raw_parts(*list.add(i), l + 1).to_vec());
    }
    (api.substring_list_free)(list);
    (rc, n, strings, lengths)
}

/// `pcre2_substring_list_get` with a NULL lengths pointer (the other branch of
/// the `lengthsptr == NULL` test at pcre2_substring.c:409).
unsafe fn probe_list_get_nolens(api: &Api, md: *mut c_void) -> (i32, usize) {
    let mut list: *mut *mut u8 = std::ptr::null_mut();
    let rc = (api.substring_list_get)(md, &mut list, std::ptr::null_mut());
    if rc != 0 {
        return (rc, 0);
    }
    let mut n = 0usize;
    while !(*list.add(n)).is_null() {
        n += 1;
        assert!(n < 100_000, "{}: runaway substring list", api.name);
    }
    (api.substring_list_free)(list);
    (rc, n)
}

// ------------------------------------------------------------ the comparator
/// Run one match in both libraries then compare EVERY substring observable.
#[allow(clippy::too_many_arguments)]
unsafe fn diff_battery(
    cc: &Compiled,
    rr: &Compiled,
    subject: &[u8],
    startoffset: usize,
    mopts: u32,
    ovecsize: Option<u32>,
    engine: Engine,
    names: &[Vec<u8>],
    label: &str,
) {
    let c = cc.api;
    let r = rr.api;
    let subj = String::from_utf8_lossy(subject).into_owned();
    let ctx = format!(
        "{} subject={:?} start={} mopts={:#x} ovec={:?} {:?}",
        label, subj, startoffset, mopts, ovecsize, engine
    );

    let cmd = mk_md(c, cc.code, ovecsize);
    let rmd = mk_md(r, rr.code, ovecsize);

    // Pre-fill BOTH ovectors with the same sentinel before matching.
    //
    // This matters for correctness of the comparison, not just tidiness:
    // `pcre2_substring_length_byname` (pcre2_substring.c:405) tests
    // `match_data->ovector[n*2] != PCRE2_UNSET` for every entry carrying the
    // name. After a NOMATCH or a PARTIAL match, the slots for non-participating
    // groups were never written, so that read is INDETERMINATE and the C
    // library's own answer varies run to run (observed flipping between
    // PCRE2_ERROR_UNSET and PCRE2_ERROR_NOMATCH). Seeding both ovectors
    // identically with PCRE2_UNSET makes the read well-defined and keeps the
    // comparison meaningful.
    {
        let n = (c.get_ovector_count)(cmd) as usize * 2;
        let m = (r.get_ovector_count)(rmd) as usize * 2;
        let cp = (c.get_ovector_pointer)(cmd);
        let rp = (r.get_ovector_pointer)(rmd);
        for i in 0..n {
            *cp.add(i) = PCRE2_UNSET;
        }
        for i in 0..m {
            *rp.add(i) = PCRE2_UNSET;
        }
    }

    let mut cws = vec![0i32; 800];
    let mut rws = vec![0i32; 800];
    let crc = run_engine(c, cc.code, cmd, subject, startoffset, mopts, engine, &mut cws);
    let rrc = run_engine(r, rr.code, rmd, subject, startoffset, mopts, engine, &mut rws);
    assert_eq!(crc, rrc, "{}: match rc differs (C={} Rust={})", ctx, crc, rrc);

    // --- the match_data accessors
    let ccount = (c.get_ovector_count)(cmd);
    let rcount = (r.get_ovector_count)(rmd);
    assert_eq!(ccount, rcount, "{}: get_ovector_count differs", ctx);

    let np = defined_pairs(crc, ccount);
    let cov = std::slice::from_raw_parts((c.get_ovector_pointer)(cmd), np * 2).to_vec();
    let rov = std::slice::from_raw_parts((r.get_ovector_pointer)(rmd), np * 2).to_vec();
    assert_eq!(cov, rov, "{}: ovector differs (rc={})", ctx, crc);

    if crc >= 0 || crc == ERR_PARTIAL {
        assert_eq!(
            (c.get_startchar)(cmd),
            (r.get_startchar)(rmd),
            "{}: get_startchar differs",
            ctx
        );
    }
    assert_eq!(read_mark(c, cmd), read_mark(r, rmd), "{}: get_mark differs", ctx);
    assert_eq!(
        (c.get_match_data_size)(cmd),
        (r.get_match_data_size)(rmd),
        "{}: get_match_data_size differs",
        ctx
    );
    assert_eq!(
        (c.get_match_data_heapframes_size)(cmd),
        (r.get_match_data_heapframes_size)(rmd),
        "{}: get_match_data_heapframes_size differs",
        ctx
    );

    let capcount = info_u32(c, cc.code, INFO_CAPTURECOUNT);

    // ------------------------------------------------- by number
    for n in 0..capcount + 4 {
        let cl = probe_len_bynumber(c, cmd, n);
        let rl = probe_len_bynumber(r, rmd, n);
        assert_eq!(cl, rl, "{}: substring_length_bynumber({}) differs", ctx, n);

        // NULL sizeptr is explicitly allowed (pcre2_substring.c:350)
        let cn = (c.substring_length_bynumber)(cmd, n, std::ptr::null_mut());
        let rn = (r.substring_length_bynumber)(rmd, n, std::ptr::null_mut());
        assert_eq!(
            cn, rn,
            "{}: substring_length_bynumber({}, NULL) rc differs",
            ctx, n
        );

        let sizes: Vec<usize> = if cl.0 == 0 && cl.1 != SENT {
            let l = cl.1;
            let mut v = vec![0usize, 1];
            if l > 1 {
                v.push(l - 1);
            }
            v.push(l);
            v.push(l + 1);
            v.push(l + 2);
            v.push(1024);
            v
        } else {
            vec![0, 1, 2, 8, 1024]
        };
        for &bs in &sizes {
            let a = probe_copy_bynumber(c, cmd, n, bs);
            let b = probe_copy_bynumber(r, rmd, n, bs);
            assert_eq!(
                a.0, b.0,
                "{}: substring_copy_bynumber({}) bufsize={} rc differs (C={} Rust={})",
                ctx, n, bs, a.0, b.0
            );
            assert_eq!(
                a.1, b.1,
                "{}: substring_copy_bynumber({}) bufsize={} written length differs",
                ctx, n, bs
            );
            assert_eq!(
                a.2, b.2,
                "{}: substring_copy_bynumber({}) bufsize={} buffer bytes differ",
                ctx, n, bs
            );
        }

        let a = probe_get_bynumber(c, cmd, n);
        let b = probe_get_bynumber(r, rmd, n);
        assert_eq!(a.0, b.0, "{}: substring_get_bynumber({}) rc differs", ctx, n);
        assert_eq!(a.1, b.1, "{}: substring_get_bynumber({}) length differs", ctx, n);
        assert_eq!(a.2, b.2, "{}: substring_get_bynumber({}) bytes differ", ctx, n);
    }

    // ------------------------------------------------- by name
    for name in names {
        let cl = probe_len_byname(c, cmd, name);
        let rl = probe_len_byname(r, rmd, name);
        assert_eq!(
            cl, rl,
            "{}: substring_length_byname({}) differs (C={:?} Rust={:?})",
            ctx,
            show(name),
            cl,
            rl
        );

        let sizes: Vec<usize> = if cl.0 == 0 && cl.1 != SENT {
            let l = cl.1;
            vec![0, 1, l, l + 1, 1024]
        } else {
            vec![0, 1, 4, 1024]
        };
        for &bs in &sizes {
            let a = probe_copy_byname(c, cmd, name, bs);
            let b = probe_copy_byname(r, rmd, name, bs);
            assert_eq!(
                a.0,
                b.0,
                "{}: substring_copy_byname({}) bufsize={} rc differs (C={} Rust={})",
                ctx,
                show(name),
                bs,
                a.0,
                b.0
            );
            assert_eq!(
                a.1,
                b.1,
                "{}: substring_copy_byname({}) bufsize={} written length differs",
                ctx,
                show(name),
                bs
            );
            assert_eq!(
                a.2,
                b.2,
                "{}: substring_copy_byname({}) bufsize={} buffer bytes differ",
                ctx,
                show(name),
                bs
            );
        }

        let a = probe_get_byname(c, cmd, name);
        let b = probe_get_byname(r, rmd, name);
        assert_eq!(
            a.0,
            b.0,
            "{}: substring_get_byname({}) rc differs",
            ctx,
            show(name)
        );
        assert_eq!(
            a.1,
            b.1,
            "{}: substring_get_byname({}) length differs",
            ctx,
            show(name)
        );
        assert_eq!(
            a.2,
            b.2,
            "{}: substring_get_byname({}) bytes differ",
            ctx,
            show(name)
        );
    }

    // ------------------------------------------------- list
    let cl = probe_list_get(c, cmd);
    let rl = probe_list_get(r, rmd);
    assert_eq!(cl.0, rl.0, "{}: substring_list_get rc differs", ctx);
    assert_eq!(cl.1, rl.1, "{}: substring_list_get string count differs", ctx);
    assert_eq!(cl.2, rl.2, "{}: substring_list_get string bytes differ", ctx);
    assert_eq!(cl.3, rl.3, "{}: substring_list_get lengths differ", ctx);

    let cl2 = probe_list_get_nolens(c, cmd);
    let rl2 = probe_list_get_nolens(r, rmd);
    assert_eq!(cl2, rl2, "{}: substring_list_get(lengths=NULL) differs", ctx);

    (c.match_data_free)(cmd);
    (r.match_data_free)(rmd);
}

/// Compare the code-level (match-data-free) functions:
/// `substring_nametable_scan` and `substring_number_from_name`.
unsafe fn diff_nametable(cc: &Compiled, rr: &Compiled, names: &[Vec<u8>], label: &str) {
    let c = cc.api;
    let r = rr.api;

    let (ccnt, cesz, _) = name_table(c, cc.code);
    let (rcnt, resz, _) = name_table(r, rr.code);
    assert_eq!(ccnt, rcnt, "{}: name_count differs", label);
    assert_eq!(cesz, resz, "{}: name_entry_size differs", label);

    for name in names {
        let a = probe_nametable_scan(c, cc.code, name);
        let b = probe_nametable_scan(r, rr.code, name);
        assert_eq!(
            a.0,
            b.0,
            "{}: nametable_scan({}) rc differs (C={} Rust={})",
            label,
            show(name),
            a.0,
            b.0
        );
        assert_eq!(
            a.1,
            b.1,
            "{}: nametable_scan({}) first/last OFFSETS differ (C={:?} Rust={:?})",
            label,
            show(name),
            a.1,
            b.1
        );

        let cn = (c.substring_number_from_name)(cc.code, name.as_ptr());
        let rn = (r.substring_number_from_name)(rr.code, name.as_ptr());
        assert_eq!(
            cn,
            rn,
            "{}: substring_number_from_name({}) differs (C={} Rust={})",
            label,
            show(name),
            cn,
            rn
        );
    }

    // A NULL name is only safe to pass when the pattern has no names at all:
    // pcre2_substring.c:498 dereferences `stringname` inside PRIV(strcmp) as
    // soon as the binary chop executes (which needs name_count > 0).
    if ccnt == 0 {
        let mut f: SPTR = std::ptr::null();
        let mut l: SPTR = std::ptr::null();
        let a = (c.substring_nametable_scan)(cc.code, std::ptr::null(), &mut f, &mut l);
        let mut f2: SPTR = std::ptr::null();
        let mut l2: SPTR = std::ptr::null();
        let b = (r.substring_nametable_scan)(rr.code, std::ptr::null(), &mut f2, &mut l2);
        assert_eq!(a, b, "{}: nametable_scan(NULL name) rc differs", label);
        let a = (c.substring_number_from_name)(cc.code, std::ptr::null());
        let b = (r.substring_number_from_name)(rr.code, std::ptr::null());
        assert_eq!(a, b, "{}: number_from_name(NULL name) rc differs", label);
    }
}

/// Drive `pcre2_next_match` to exhaustion and compare the whole sequence.
unsafe fn diff_next_match(
    cc: &Compiled,
    rr: &Compiled,
    subject: &[u8],
    base_opts: u32,
    label: &str,
) {
    let c = cc.api;
    let r = rr.api;
    let ctx = format!(
        "{} next_match subject={:?} opts={:#x}",
        label,
        String::from_utf8_lossy(subject),
        base_opts
    );
    let cmd = mk_md(c, cc.code, None);
    let rmd = mk_md(r, rr.code, None);

    let mut cstart = 0usize;
    let mut rstart = 0usize;
    let mut copts = base_opts;
    let mut ropts = base_opts;
    let mut ws = vec![0i32; 8];

    for step in 0..256 {
        let crc = run_engine(
            c,
            cc.code,
            cmd,
            subject,
            cstart,
            copts,
            Engine::Interpreter,
            &mut ws,
        );
        let rrc = run_engine(
            r,
            rr.code,
            rmd,
            subject,
            rstart,
            ropts,
            Engine::Interpreter,
            &mut ws,
        );
        assert_eq!(crc, rrc, "{}: step {} match rc differs", ctx, step);

        let ccount = (c.get_ovector_count)(cmd);
        let np = defined_pairs(crc, ccount);
        let cov = std::slice::from_raw_parts((c.get_ovector_pointer)(cmd), np * 2).to_vec();
        let rov = std::slice::from_raw_parts((r.get_ovector_pointer)(rmd), np * 2).to_vec();
        assert_eq!(cov, rov, "{}: step {} ovector differs", ctx, step);
        assert_eq!(
            read_mark(c, cmd),
            read_mark(r, rmd),
            "{}: step {} mark differs",
            ctx,
            step
        );

        let mut cns = SENT;
        let mut cno = 0xAAAA_AAAAu32;
        let cmore = (c.next_match)(cmd, &mut cns, &mut cno);
        let mut rns = SENT;
        let mut rno = 0xAAAA_AAAAu32;
        let rmore = (r.next_match)(rmd, &mut rns, &mut rno);
        assert_eq!(cmore, rmore, "{}: step {} next_match rc differs", ctx, step);
        assert_eq!(
            cns, rns,
            "{}: step {} next_match start_offset differs (C={:#x} Rust={:#x})",
            ctx, step, cns, rns
        );
        assert_eq!(
            cno, rno,
            "{}: step {} next_match options differ (C={:#x} Rust={:#x})",
            ctx, step, cno, rno
        );
        if cmore == 0 {
            break;
        }
        // guard against a non-progressing loop in a buggy implementation
        assert!(cns != SENT, "{}: step {} next_match left offset unwritten", ctx, step);
        cstart = cns;
        rstart = rns;
        copts = base_opts | cno;
        ropts = base_opts | rno;
    }

    (c.match_data_free)(cmd);
    (r.match_data_free)(rmd);
}

// ================================================================== corpora
/// Patterns chosen to cover every branch of pcre2_substring.c: no groups, one
/// group, many groups, named groups, many named groups, duplicate names,
/// participating vs non-participating groups, nesting, alternation, and
/// zero-length captures.
const PATTERNS: &[&str] = &[
    // --- no capturing groups at all (name_count == 0, top_bracket == 0)
    "abc",
    "a*",
    "(?:abc)",
    "(?:a|b)+",
    ".",
    "",
    // --- exactly one group
    "(a)",
    "(a*)",
    "(abc)?d",
    "(.*)",
    // --- several groups
    "(a)(b)",
    "(a)(b)(c)",
    "(a)(b)(c)(d)(e)",
    "(\\w+)\\s+(\\w+)\\s+(\\w+)",
    // --- MANY groups (exercises top_bracket > ovector size too)
    "(a)(b)(c)(d)(e)(f)(g)(h)(i)(j)(k)(l)",
    "(1)?(2)?(3)?(4)?(5)?(6)?(7)?(8)?(9)?x",
    // --- named groups
    "(?<a>x)",
    "(?<a>x)(?<b>y)",
    "(?<first>\\w+)\\s(?<second>\\w+)",
    "(?'q'a)(?P<w>b)(?<e>c)",
    // --- MANY named groups: forces a multi-entry name table and binary chop
    "(?<n01>a)(?<n02>b)(?<n03>c)(?<n04>d)(?<n05>e)(?<n06>f)(?<n07>g)(?<n08>h)\
     (?<n09>i)(?<n10>j)(?<n11>k)(?<n12>l)",
    "(?<zebra>a)(?<apple>b)(?<mango>c)(?<kiwi>d)(?<lemon>e)",
    // --- names of very different lengths (name_entry_size padding)
    "(?<a>1)(?<bb>2)(?<ccc>3)(?<dddd>4)(?<eeeee>5)",
    // --- duplicate names via (?J) and via PCRE2_DUPNAMES
    "(?J)(?<n>a)|(?<n>b)",
    "(?J)(?<n>a)|(?<n>b)|(?<n>c)",
    "(?J)(?<n>a)(?<n>b)?",
    "(?J)(?<dup>x)(?<other>y)|(?<dup>z)",
    "(?J)(?<n>a)|(?<n>b)|(?<m>c)|(?<m>d)",
    // --- groups that do NOT participate (unset -> PCRE2_UNSET)
    "(a)|(b)",
    "(a)|(b)|(c)",
    "(a)(b)?(c)?",
    "x(?:(a)|(b))y",
    "(?<yes>a)|(?<no>b)",
    // --- nested groups
    "((a)(b))",
    "(a(b(c)))",
    "((((a))))",
    "(a(b)?(c)?)?d",
    "((\\w)(\\w))+",
    // --- groups after alternation
    "(?:foo|bar)(x)(y)?",
    "abc|(def)(ghi)",
    "^(?:(a)|(?:(b)(c)))$",
    // --- zero-length / empty captures
    "()",
    "()()()",
    "(a?)(b?)(c?)",
    "(?<e>)x",
    "(\\b)(\\w*)",
    "(?=(a))a",
    // --- \K, which can push ovector[0] around
    "a\\Kb",
    "(?<n>a)\\Kb",
    // --- back references
    "(a)\\1",
    "(?<n>a)\\k<n>",
];

/// Patterns that need `PCRE2_MATCH_UNSET_BACKREF`.
const UNSET_BACKREF_PATTERNS: &[&str] = &[
    "(a)?\\1b",
    "(?:(a)|(b))\\1\\2",
    "(?<n>a)?\\k<n>b",
    "(a)(b)?\\2",
];

/// UTF patterns with multi-byte captures.
const UTF_PATTERNS: &[&str] = &[
    "(\u{00e9})",
    "(?<acc>\u{00e9}+)",
    "(\u{20ac})(\u{00a3})?",
    "(?<emoji>\u{1F600})(?<rest>.*)",
    "(\u{03B1})(\u{03B2})(\u{03B3})",
    "(.)(.)(.)",
    "(?J)(?<n>\u{00e9})|(?<n>\u{20ac})",
    "(\\X)(\\X)",
    "(?<w>\\w+)\\W(?<v>\\w+)",
];

const SUBJECTS: &[&str] = &[
    "",
    "a",
    "b",
    "c",
    "d",
    "x",
    "ab",
    "abc",
    "abcd",
    "aab",
    "aaa",
    "xay",
    "xby",
    "abcabc",
    "def",
    "defghi",
    "foox",
    "fooxy",
    "barx",
    "12345",
    "123456789x",
    "abcdefghijkl",
    "one two three",
    "hello   world  again",
    "a\0b",
    "a\nb",
    "\0\0",
    " ",
    "12",
    "zzzz",
];

const UTF_SUBJECTS: &[&str] = &[
    "",
    "a",
    "\u{00e9}",
    "\u{00e9}\u{00e9}\u{00e9}",
    "\u{20ac}",
    "\u{20ac}\u{00a3}",
    "\u{1F600}tail",
    "\u{03B1}\u{03B2}\u{03B3}",
    "caf\u{00e9} au lait",
    "\u{4E00}\u{4E8C}\u{4E09}",
    "abc",
    "a\u{00e9}b",
];

/// Compile one pattern in both libraries and collect the name list to probe.
struct Case {
    cc: Compiled,
    rr: Compiled,
    names: Vec<Vec<u8>>,
    label: String,
}

unsafe fn build_cases(patterns: &[&str], cfg: &CompileCfg, tag: &str) -> Vec<Case> {
    let mut out = Vec::new();
    for pat in patterns {
        let label = format!("{} pattern={:?}", tag, pat);
        let (cc, rr) = compile_both(pat.as_bytes(), pat.len(), cfg, &label);
        if cc.code.is_null() {
            continue;
        }
        let cnames = names_in_pattern(cc.api, cc.code);
        let rnames = names_in_pattern(rr.api, rr.code);
        assert_eq!(cnames, rnames, "{}: name table contents differ", label);
        let mut names = cnames;
        names.extend(bogus_names());
        out.push(Case { cc, rr, names, label });
    }
    out
}

// ==================================================================== tests

/// Every substring function, over the whole corpus, with the ovector sized from
/// the pattern.
#[test]
fn substring_all_functions_default() {
    unsafe {
        let cases = build_cases(PATTERNS, &CompileCfg::new(0), "default");
        assert!(!cases.is_empty());
        for case in &cases {
            diff_nametable(&case.cc, &case.rr, &case.names, &case.label);
            for subj in SUBJECTS {
                let b = subj.as_bytes();
                for start in [0usize, b.len() / 2, b.len()] {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        b,
                        start,
                        0,
                        None,
                        Engine::Interpreter,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
    }
}

/// The same battery under the compile options that change what gets captured:
/// CASELESS, UNGREEDY, NO_AUTO_CAPTURE (which removes all numbered groups) and
/// DOTALL.
#[test]
fn substring_compile_option_axes() {
    unsafe {
        for &(tag, opts) in &[
            ("CASELESS", PCRE2_CASELESS),
            ("UNGREEDY", PCRE2_UNGREEDY),
            ("NO_AUTO_CAPTURE", PCRE2_NO_AUTO_CAPTURE),
            ("DOTALL", PCRE2_DOTALL),
            ("MULTILINE", PCRE2_MULTILINE),
            ("ANCHORED", PCRE2_ANCHORED),
            ("ENDANCHORED", PCRE2_ENDANCHORED),
        ] {
            let cases = build_cases(PATTERNS, &CompileCfg::new(opts), tag);
            for case in &cases {
                diff_nametable(&case.cc, &case.rr, &case.names, &case.label);
                for subj in SUBJECTS {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        subj.as_bytes(),
                        0,
                        0,
                        None,
                        Engine::Interpreter,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
    }
}

/// The battery under the match options that change the ovector contents.
#[test]
fn substring_match_option_axes() {
    unsafe {
        let cases = build_cases(PATTERNS, &CompileCfg::new(0), "mopts");
        for &mopts in &[
            PCRE2_NOTBOL,
            PCRE2_NOTEOL,
            PCRE2_NOTEMPTY,
            PCRE2_NOTEMPTY_ATSTART,
            PCRE2_NOTBOL | PCRE2_NOTEOL,
        ] {
            for case in &cases {
                for subj in SUBJECTS {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        subj.as_bytes(),
                        0,
                        mopts,
                        None,
                        Engine::Interpreter,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
    }
}

/// The same corpus but with deliberately UNDERSIZED ovectors, which drives the
/// `PCRE2_ERROR_UNAVAILABLE` branches (pcre2_substring.c:328, 83, 171, 278) and
/// the `rc == 0` path in `substring_list_get` (line 390).
#[test]
fn substring_small_ovectors() {
    unsafe {
        let cases = build_cases(PATTERNS, &CompileCfg::new(0), "small-ovec");
        for case in &cases {
            for ovec in [1u32, 2, 3, 5] {
                for subj in SUBJECTS {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        subj.as_bytes(),
                        0,
                        0,
                        Some(ovec),
                        Engine::Interpreter,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
    }
}

/// `PCRE2_DUPNAMES` compiled patterns (as opposed to the inline `(?J)` form),
/// exercising the duplicate-name scan loops at pcre2_substring.c:80/168/275 and
/// the `PCRE2_ERROR_NOUNIQUESUBSTRING` return at line 517.
#[test]
fn substring_dupnames() {
    let pats: &[&str] = &[
        "(?<n>a)|(?<n>b)",
        "(?<n>a)|(?<n>b)|(?<n>c)|(?<n>d)",
        "(?<n>a)(?<n>b)?",
        "(?<n>a)?(?<n>b)?(?<n>c)?",
        "(?<a>1)|(?<a>2)|(?<b>3)|(?<b>4)|(?<c>5)",
        "(?<n>x(?<n>y))",
        "(?<same>a)(?<same>b)(?<same>c)",
    ];
    unsafe {
        let cases = build_cases(pats, &CompileCfg::new(PCRE2_DUPNAMES), "dupnames");
        assert!(!cases.is_empty());
        for case in &cases {
            diff_nametable(&case.cc, &case.rr, &case.names, &case.label);
            for subj in SUBJECTS {
                for ovec in [None, Some(1u32), Some(2)] {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        subj.as_bytes(),
                        0,
                        0,
                        ovec,
                        Engine::Interpreter,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
    }
}

/// `PCRE2_MATCH_UNSET_BACKREF` changes which groups end up unset.
#[test]
fn substring_match_unset_backref() {
    unsafe {
        for &opts in &[PCRE2_MATCH_UNSET_BACKREF, PCRE2_MATCH_UNSET_BACKREF | PCRE2_DUPNAMES]
        {
            let cases = build_cases(
                UNSET_BACKREF_PATTERNS,
                &CompileCfg::new(opts),
                &format!("unset-backref opts={:#x}", opts),
            );
            for case in &cases {
                diff_nametable(&case.cc, &case.rr, &case.names, &case.label);
                for subj in SUBJECTS {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        subj.as_bytes(),
                        0,
                        0,
                        None,
                        Engine::Interpreter,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
    }
}

/// UTF patterns with multi-byte captures — the copied bytes must agree exactly.
#[test]
fn substring_utf_multibyte() {
    unsafe {
        for &opts in &[PCRE2_UTF, PCRE2_UTF | PCRE2_UCP, PCRE2_UTF | PCRE2_DUPNAMES] {
            let cases = build_cases(
                UTF_PATTERNS,
                &CompileCfg::new(opts),
                &format!("utf opts={:#x}", opts),
            );
            for case in &cases {
                diff_nametable(&case.cc, &case.rr, &case.names, &case.label);
                for subj in UTF_SUBJECTS {
                    let b = subj.as_bytes();
                    // only code-point boundaries are legal start offsets in UTF
                    let starts: Vec<usize> = subj.char_indices().map(|(i, _)| i).collect();
                    for start in starts.into_iter().chain(std::iter::once(b.len())) {
                        diff_battery(
                            &case.cc,
                            &case.rr,
                            b,
                            start,
                            0,
                            None,
                            Engine::Interpreter,
                            &case.names,
                            &case.label,
                        );
                    }
                }
            }
        }
    }
}

/// `pcre2_dfa_match` sets `matchedby = PCRE2_MATCHEDBY_DFA_INTERPRETER`, which
/// makes the `_byname` functions return `PCRE2_ERROR_DFA_UFUNC`
/// (pcre2_substring.c:74, 162, 269) and takes the alternative branch at line
/// 333 in `substring_length_bynumber`.
#[test]
fn substring_after_dfa_match() {
    unsafe {
        let cases = build_cases(PATTERNS, &CompileCfg::new(0), "dfa");
        for case in &cases {
            for subj in SUBJECTS {
                for ovec in [None, Some(1u32), Some(3)] {
                    diff_battery(
                        &case.cc,
                        &case.rr,
                        subj.as_bytes(),
                        0,
                        0,
                        ovec,
                        Engine::Dfa,
                        &case.names,
                        &case.label,
                    );
                }
            }
        }
        // Confirm the DFA_UFUNC contract really is being reached in both libs.
        let (c, r) = both();
        let pat = b"(?<n>a)(b)";
        let (cc, rr) = compile_both(pat, pat.len(), &CompileCfg::new(0), "dfa-ufunc");
        let cmd = mk_md(c, cc.code, None);
        let rmd = mk_md(r, rr.code, None);
        let mut ws = vec![0i32; 200];
        let subj = b"ab";
        let a = (c.dfa_match)(
            cc.code,
            subj.as_ptr(),
            2,
            0,
            0,
            cmd,
            std::ptr::null_mut(),
            ws.as_mut_ptr(),
            ws.len(),
        );
        let b = (r.dfa_match)(
            rr.code,
            subj.as_ptr(),
            2,
            0,
            0,
            rmd,
            std::ptr::null_mut(),
            ws.as_mut_ptr(),
            ws.len(),
        );
        assert_eq!(a, b, "dfa-ufunc: dfa_match rc");
        assert!(a > 0, "dfa-ufunc: expected a match, got {}", a);
        let name = b"n\0";
        let cl = probe_len_byname(c, cmd, name);
        let rl = probe_len_byname(r, rmd, name);
        assert_eq!(cl, rl, "dfa-ufunc: length_byname");
        assert_eq!(cl.0, ERR_DFA_UFUNC, "dfa-ufunc: expected DFA_UFUNC");
        (c.match_data_free)(cmd);
        (r.match_data_free)(rmd);
    }
}

/// Partial matches take the special path at pcre2_substring.c:317-321.
#[test]
fn substring_after_partial_match() {
    unsafe {
        let pats: &[&str] = &[
            "(abcd)",
            "(?<n>abcd)",
            "(a)(b)(c)(d)",
            "^(?<h>\\d\\d)-(?<t>\\d\\d)$",
            "(?J)(?<n>abc)|(?<n>abd)",
            "()abcd",
        ];
        for &mopts in &[PCRE2_PARTIAL_SOFT, PCRE2_PARTIAL_HARD] {
            let cases = build_cases(
                pats,
                &CompileCfg::new(PCRE2_DUPNAMES),
                &format!("partial mopts={:#x}", mopts),
            );
            for case in &cases {
                for subj in ["", "a", "ab", "abc", "abcd", "12", "12-", "12-3"] {
                    for engine in [Engine::Interpreter, Engine::Dfa] {
                        diff_battery(
                            &case.cc,
                            &case.rr,
                            subj.as_bytes(),
                            0,
                            mopts,
                            None,
                            engine,
                            &case.names,
                            &case.label,
                        );
                    }
                }
            }
        }
    }
}

/// `pcre2_next_match` driven to exhaustion over the corpus.
#[test]
fn substring_next_match_sequences() {
    let pats: &[&str] = &[
        "a",
        "a*",
        "a*?",
        "",
        "()",
        "b?",
        "(a)|(b)",
        "\\b",
        "(?<n>a)",
        "x",
        ".",
        ".?",
        "(*MARK:one)a|(*MARK:two)b",
        "a\\Kb",
        "(?=a)",
        "(?<=a)",
        "\\Ka",
        "(?:(?<=a)\\K)",
        "\\R",
        "(\\w)",
        "\\X",
    ];
    let subjects: &[&str] = &[
        "",
        "a",
        "aa",
        "aaa",
        "ab",
        "abab",
        "bbb",
        "xaxaxa",
        "a\r\nb",
        "\r\n\r\n",
        "one a two b three",
        "aaaaaaaa",
    ];
    unsafe {
        for &opts in &[0u32, PCRE2_MULTILINE] {
            let cases = build_cases(pats, &CompileCfg::new(opts), &format!("next opts={:#x}", opts));
            for case in &cases {
                for subj in subjects {
                    diff_next_match(&case.cc, &case.rr, subj.as_bytes(), 0, &case.label);
                }
            }
        }
        // and with a CRLF newline convention, which changes do_bumpalong()
        for nl in ALL_NEWLINES {
            let cases = build_cases(
                &["a", "", ".", "\\R", "b?"],
                &CompileCfg::new(0).newline(nl),
                &format!("next nl={}", nl),
            );
            for case in &cases {
                for subj in ["a\r\nb", "\r\n", "\r\r\n\n", "ab"] {
                    diff_next_match(&case.cc, &case.rr, subj.as_bytes(), 0, &case.label);
                }
            }
        }
        // UTF mode changes do_bumpalong() as well (FORWARDCHARTEST)
        let cases = build_cases(
            &["", ".", "a", "\\X", "(?<n>.)"],
            &CompileCfg::new(PCRE2_UTF),
            "next utf",
        );
        for case in &cases {
            for subj in UTF_SUBJECTS {
                diff_next_match(&case.cc, &case.rr, subj.as_bytes(), 0, &case.label);
            }
        }
    }
}

/// Name-table scanning on its own, including absent names, the empty name, and
/// duplicate names, for a corpus with widely varying name tables.
#[test]
fn substring_nametable_scan_only() {
    let pats: &[&str] = &[
        "abc",
        "(a)",
        "(?<a>x)",
        "(?<aa>x)(?<ab>y)(?<ac>z)",
        "(?<b>1)(?<a>2)(?<c>3)",
        "(?J)(?<n>a)|(?<n>b)|(?<n>c)",
        "(?J)(?<a>1)|(?<a>2)|(?<b>3)",
        "(?<n01>a)(?<n02>b)(?<n03>c)(?<n04>d)(?<n05>e)(?<n06>f)(?<n07>g)(?<n08>h)\
         (?<n09>i)(?<n10>j)(?<n11>k)(?<n12>l)(?<n13>m)(?<n14>n)(?<n15>o)(?<n16>p)",
        "(?<x>1)(?<xx>2)(?<xxx>3)(?<xxxx>4)(?<xxxxx>5)",
        "(?<\u{00e9}>a)",
    ];
    unsafe {
        for &opts in &[0u32, PCRE2_DUPNAMES, PCRE2_UTF, PCRE2_UTF | PCRE2_DUPNAMES] {
            let cases =
                build_cases(pats, &CompileCfg::new(opts), &format!("nt opts={:#x}", opts));
            for case in &cases {
                // probe every name in the table, every 1-char prefix/suffix
                // variation of it, and the bogus set
                let mut names = case.names.clone();
                for n in case.names.clone() {
                    let base = &n[..n.len() - 1];
                    let mut longer = base.to_vec();
                    longer.extend_from_slice(b"x\0");
                    names.push(longer);
                    if base.len() > 1 {
                        let mut shorter = base[..base.len() - 1].to_vec();
                        shorter.push(0);
                        names.push(shorter);
                    }
                }
                diff_nametable(&case.cc, &case.rr, &names, &case.label);
            }
        }
    }
}

/// Randomized subjects against the whole pattern corpus.
#[test]
fn substring_randomized() {
    unsafe {
        let mut cases = build_cases(PATTERNS, &CompileCfg::new(0), "rand");
        cases.extend(build_cases(
            UNSET_BACKREF_PATTERNS,
            &CompileCfg::new(PCRE2_MATCH_UNSET_BACKREF | PCRE2_DUPNAMES),
            "rand-unset",
        ));
        cases.extend(build_cases(
            &[
                "(?<n>a)|(?<n>b)",
                "(?<n>a)(?<n>b)?(?<m>c)?",
                "(?<a>1)|(?<a>2)|(?<b>3)",
            ],
            &CompileCfg::new(PCRE2_DUPNAMES),
            "rand-dup",
        ));
        assert!(cases.len() > 20);

        let alphabet: &[u8] = b"ab c123\ndefghijkl\0xyz\t";
        let mut rng = Rng::new(SEED);
        let mut iters = 0;
        while iters < 1600 {
            let case = &cases[rng.below(cases.len() as u32) as usize];
            let len = rng.below(14) as usize;
            let subj = rng.bytes_from(len, alphabet);
            let start = if subj.is_empty() {
                0
            } else {
                rng.below(subj.len() as u32 + 1) as usize
            };
            let ovec = match rng.below(4) {
                0 => None,
                1 => Some(1),
                2 => Some(2),
                _ => Some(rng.range(1, 8)),
            };
            let engine = if rng.below(5) == 0 {
                Engine::Dfa
            } else {
                Engine::Interpreter
            };
            let mopts = match rng.below(6) {
                0 => PCRE2_NOTEMPTY,
                1 => PCRE2_PARTIAL_SOFT,
                2 => PCRE2_NOTBOL,
                3 => PCRE2_NOTEMPTY_ATSTART,
                _ => 0,
            };
            diff_battery(
                &case.cc,
                &case.rr,
                &subj,
                start,
                mopts,
                ovec,
                engine,
                &case.names,
                &case.label,
            );
            iters += 1;
        }
        assert_eq!(iters, 1600);
    }
}

/// Randomized `pcre2_next_match` walks (a second randomized axis, with raw
/// bytes so that CR/LF/NUL bumpalong handling is hit).
#[test]
fn substring_randomized_next_match() {
    unsafe {
        let pats: &[&str] = &[
            "", "a", "a*", "b?", ".", ".?", "\\R", "(a)|(b)", "\\b", "a\\Kb",
            "(?<n>a)|(?<m>b)", "\\X", "(*MARK:m)a|b", "(?:)", "[ab]*",
        ];
        let mut cases = build_cases(pats, &CompileCfg::new(0), "rand-next");
        cases.extend(build_cases(pats, &CompileCfg::new(PCRE2_UTF), "rand-next-utf"));
        cases.extend(build_cases(
            &["a", "", ".", "\\R"],
            &CompileCfg::new(0).newline(NL_CRLF),
            "rand-next-crlf",
        ));
        cases.extend(build_cases(
            &["a", "", ".", "\\R"],
            &CompileCfg::new(0).newline(NL_ANYCRLF),
            "rand-next-anycrlf",
        ));

        let alphabet: &[u8] = b"ab\r\n \0";
        let mut rng = Rng::new(SEED ^ 0xDEAD);
        for _ in 0..1600 {
            let case = &cases[rng.below(cases.len() as u32) as usize];
            let len = rng.below(10) as usize;
            let subj = rng.bytes_from(len, alphabet);
            // Non-UTF subjects only for the UTF cases would raise UTF errors in
            // both libraries identically, which is itself worth comparing.
            let mopts = if rng.below(4) == 0 { PCRE2_NO_UTF_CHECK } else { 0 };
            diff_next_match(&case.cc, &case.rr, &subj, mopts, &case.label);
        }
    }
}
