//! Phase B — differential tests for the encoder (CONFIGS.md rows 76..95).

mod common;
use common::tree::*;
use common::*;
use std::os::raw::{c_char, c_int, c_void};
use std::ptr;
use std::sync::Mutex;

/* --------------------------------------------------- dump callback state */

static CHUNKS: Mutex<Vec<Vec<u8>>> = Mutex::new(Vec::new());
static FAIL_AT: Mutex<i64> = Mutex::new(-1);

unsafe extern "C" fn cb_record(buf: *const c_char, size: usize, _data: *mut c_void) -> c_int {
    let mut g = CHUNKS.lock().unwrap();
    let n = g.len() as i64;
    g.push(std::slice::from_raw_parts(buf as *const u8, size).to_vec());
    let f = *FAIL_AT.lock().unwrap();
    if f >= 0 && n == f {
        -1
    } else {
        0
    }
}

fn cb_reset(fail_at: i64) {
    CHUNKS.lock().unwrap().clear();
    *FAIL_AT.lock().unwrap() = fail_at;
}

fn cb_take() -> Vec<Vec<u8>> {
    std::mem::take(&mut *CHUNKS.lock().unwrap())
}

/* ------------------------------------------------------------ flag sets */

fn all_flag_sets() -> Vec<(String, usize)> {
    let mut v = Vec::new();
    v.push(("plain".into(), 0usize));
    for n in 0..32usize {
        v.push((format!("indent{n}"), json_indent(n)));
    }
    v.push(("compact".into(), JSON_COMPACT));
    v.push(("compact+ind4".into(), JSON_COMPACT | json_indent(4)));
    v.push(("ascii".into(), JSON_ENSURE_ASCII));
    v.push(("sort".into(), JSON_SORT_KEYS));
    v.push((
        "sort+compact+ind3".into(),
        JSON_SORT_KEYS | JSON_COMPACT | json_indent(3),
    ));
    v.push(("preserve".into(), JSON_PRESERVE_ORDER));
    v.push(("preserve+sort".into(), JSON_PRESERVE_ORDER | JSON_SORT_KEYS));
    v.push(("slash".into(), JSON_ESCAPE_SLASH));
    v.push(("any".into(), JSON_ENCODE_ANY));
    for n in 0..32usize {
        v.push((format!("prec{n}"), JSON_ENCODE_ANY | json_real_precision(n)));
    }
    v.push(("embed".into(), JSON_EMBED));
    v.push(("embed+ind2".into(), JSON_EMBED | json_indent(2)));
    v.push((
        "everything".into(),
        JSON_COMPACT
            | JSON_ENSURE_ASCII
            | JSON_SORT_KEYS
            | JSON_PRESERVE_ORDER
            | JSON_ENCODE_ANY
            | JSON_ESCAPE_SLASH
            | JSON_EMBED
            | json_indent(2)
            | json_real_precision(5),
    ));
    // out-of-range / unknown flag bits (ERRORS.md row 295)
    v.push(("allbits".into(), usize::MAX));
    v.push(("highbits".into(), 0xFFFF_FFFF_0000_0000));
    v
}

fn sample_trees(rng: &mut Rng) -> Vec<Spec> {
    let mut v: Vec<Spec> = vec![
        Spec::Arr(vec![]),
        Spec::Obj(vec![]),
        Spec::Arr(vec![Spec::Int(1)]),
        Spec::Obj(vec![(b"a".to_vec(), Spec::Int(1))]),
        Spec::Arr(vec![Spec::Arr(vec![Spec::Arr(vec![])])]),
        Spec::Obj(vec![
            (b"aa".to_vec(), Spec::Int(1)),
            (b"a".to_vec(), Spec::Int(2)),
            (b"ab".to_vec(), Spec::Int(3)),
            (b"".to_vec(), Spec::Int(4)),
            (b"b".to_vec(), Spec::Int(5)),
            (b"aaa".to_vec(), Spec::Int(6)),
        ]),
        Spec::Arr(vec![
            Spec::Str("plain".into()),
            Spec::Str("\u{80}".into()),
            Spec::Str("\u{7FF}".into()),
            Spec::Str("\u{800}".into()),
            Spec::Str("\u{FFFF}".into()),
            Spec::Str("\u{10000}".into()),
            Spec::Str("\u{10FFFF}".into()),
            Spec::Str("with / slash".into()),
            Spec::Str("q\"b\\s".into()),
            Spec::StrRaw(vec![1, 2, 3, 0x1f, 0x7f, b'a']),
        ]),
        Spec::Arr(vec![
            Spec::Real(0.0),
            Spec::Real(-0.0),
            Spec::Real(1.0),
            Spec::Real(0.1),
            Spec::Real(1e16),
            Spec::Real(1e17),
            Spec::Real(1e-4),
            Spec::Real(1e-5),
            Spec::Real(f64::MAX),
            Spec::Real(5e-324),
            Spec::Real(1.0 / 3.0),
            Spec::Int(i64::MIN),
            Spec::Int(i64::MAX),
        ]),
    ];
    // deeply nested
    let mut deep = Spec::Int(0);
    for _ in 0..64 {
        deep = Spec::Arr(vec![deep]);
    }
    v.push(deep);
    let mut deepo = Spec::Int(0);
    for i in 0..40 {
        deepo = Spec::Obj(vec![(format!("d{i}").into_bytes(), deepo)]);
    }
    v.push(deepo);
    for _ in 0..40 {
        v.push(rand_container(rng, 3));
    }
    v
}

/* --------------------------------- rows 76..88: json_dumps × all flags -- */

#[test]
fn cfg76to88_json_dumps_all_flags() {
    diff("cfg76-88 json_dumps flags", |api, rec| unsafe {
        let mut rng = Rng::new(0x7600);
        let trees = sample_trees(&mut rng);
        let flags = all_flag_sets();
        for (ti, spec) in trees.iter().enumerate() {
            let j = build(api, spec);
            for (name, f) in &flags {
                match dumps(api, j, *f) {
                    None => rec.line(&format!("t{ti}.{name}=NULL")),
                    Some(d) => rec.tag_bytes(&format!("t{ti}.{name}"), &d),
                }
            }
            decref(api, j);
        }
        // row 84: JSON_ENCODE_ANY on every scalar type
        for s in [
            Spec::Null,
            Spec::True,
            Spec::False,
            Spec::Int(0),
            Spec::Int(-1),
            Spec::Real(1.5),
            Spec::Str("s".into()),
        ] {
            let j = build(api, &s);
            for (name, f) in &flags {
                match dumps(api, j, *f) {
                    None => rec.line(&format!("scalar.{name}=NULL")),
                    Some(d) => rec.tag_bytes(&format!("scalar.{name}"), &d),
                }
            }
            decref(api, j);
        }
    });
}

#[test]
fn cfg87_random_flag_crossproduct() {
    diff("cfg87 randomised flag cross-product", |api, rec| unsafe {
        let mut rng = Rng::new(0x8700);
        for _ in 0..400 {
            let spec = rand_container(&mut rng, 3);
            let j = build(api, &spec);
            let f = json_indent(rng.below(32))
                | if rng.below(2) == 0 { JSON_COMPACT } else { 0 }
                | if rng.below(2) == 0 { JSON_ENSURE_ASCII } else { 0 }
                | if rng.below(2) == 0 { JSON_SORT_KEYS } else { 0 }
                | if rng.below(2) == 0 { JSON_PRESERVE_ORDER } else { 0 }
                | if rng.below(2) == 0 { JSON_ENCODE_ANY } else { 0 }
                | if rng.below(2) == 0 { JSON_ESCAPE_SLASH } else { 0 }
                | if rng.below(2) == 0 { JSON_EMBED } else { 0 }
                | json_real_precision(rng.below(32));
            rec.tag_u("flags", f);
            match dumps(api, j, f) {
                None => rec.line("dump=NULL"),
                Some(d) => rec.tag_bytes("dump", &d),
            }
            decref(api, j);
        }
    });
}

/* ---------------------------------------- rows 89..90: json_dumpb ------ */

#[test]
fn cfg89and90_json_dumpb() {
    diff("cfg89-90 json_dumpb", |api, rec| unsafe {
        let mut rng = Rng::new(0x8900);
        let trees = sample_trees(&mut rng);
        let flags = all_flag_sets();
        for (ti, spec) in trees.iter().enumerate() {
            let j = build(api, spec);
            for (name, f) in &flags {
                let needed = (api.json_dumpb)(j, ptr::null_mut(), 0, *f);
                rec.tag_u(&format!("t{ti}.{name}.needed"), needed);
                for size in [
                    0usize,
                    1,
                    needed.saturating_sub(1),
                    needed,
                    needed + 1,
                    needed + 64,
                ] {
                    let mut buf = vec![0x5Au8; size + 8];
                    let n = (api.json_dumpb)(j, buf.as_mut_ptr() as *mut c_char, size, *f);
                    rec.tag_u(&format!("t{ti}.{name}.n{size}"), n);
                    rec.tag_bytes(&format!("t{ti}.{name}.b{size}"), &buf);
                }
            }
            decref(api, j);
        }
    });
}

/* ------------------------------ rows 91..93: FILE*, fd, path sinks ----- */

#[test]
fn cfg91to93_file_sinks() {
    diff("cfg91-93 file sinks", |api, rec| unsafe {
        let mut rng = Rng::new(0x9100);
        let trees = sample_trees(&mut rng);
        let flags = all_flag_sets();
        let pf = tmp_file("dumpf");
        let pfd = tmp_file("dumpfd");
        let pfile = tmp_file("dumpfile");
        let cpf = cs(pf.to_str().unwrap());
        let cpfile = cs(pfile.to_str().unwrap());
        let mode = cs("w+");
        for (ti, spec) in trees.iter().enumerate().take(12) {
            let j = build(api, spec);
            for (name, f) in &flags {
                // json_dumpf
                let fh = fopen(cpf.as_ptr(), mode.as_ptr());
                assert!(!fh.is_null());
                let r = (api.json_dumpf)(j, fh, *f);
                fflush(fh);
                fclose(fh);
                rec.tag_i(&format!("t{ti}.{name}.dumpf"), r as i64);
                rec.tag_bytes(
                    &format!("t{ti}.{name}.dumpf_content"),
                    &std::fs::read(&pf).unwrap(),
                );

                // json_dumpfd
                {
                    use std::os::unix::io::AsRawFd;
                    let file = std::fs::File::create(&pfd).unwrap();
                    let r = (api.json_dumpfd)(j, file.as_raw_fd(), *f);
                    drop(file);
                    rec.tag_i(&format!("t{ti}.{name}.dumpfd"), r as i64);
                    rec.tag_bytes(
                        &format!("t{ti}.{name}.dumpfd_content"),
                        &std::fs::read(&pfd).unwrap(),
                    );
                }

                // json_dump_file
                let r = (api.json_dump_file)(j, cpfile.as_ptr(), *f);
                rec.tag_i(&format!("t{ti}.{name}.dumpfile"), r as i64);
                rec.tag_bytes(
                    &format!("t{ti}.{name}.dumpfile_content"),
                    &std::fs::read(&pfile).unwrap_or_default(),
                );
            }
            decref(api, j);
        }
        let _ = std::fs::remove_file(&pf);
        let _ = std::fs::remove_file(&pfd);
        let _ = std::fs::remove_file(&pfile);
    });
}

/* ----------------------------- rows 94..95: json_dump_callback --------- */

#[test]
fn cfg94and95_dump_callback_chunks() {
    diff("cfg94-95 dump_callback chunking", |api, rec| unsafe {
        let mut rng = Rng::new(0x9400);
        let trees = sample_trees(&mut rng);
        let flags = all_flag_sets();
        for (ti, spec) in trees.iter().enumerate() {
            let j = build(api, spec);
            for (name, f) in &flags {
                cb_reset(-1);
                let r = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), *f);
                let chunks = cb_take();
                rec.tag_i(&format!("t{ti}.{name}.ret"), r as i64);
                rec.tag_i(&format!("t{ti}.{name}.nchunks"), chunks.len() as i64);
                for (i, c) in chunks.iter().enumerate() {
                    rec.tag_bytes(&format!("t{ti}.{name}.c{i}"), c);
                }
            }
            decref(api, j);
        }
    });
}

#[test]
fn cfg95_dump_callback_failure_at_every_chunk() {
    // ERRORS.md rows 118, 120, 137: the callback fails at chunk k for every
    // reachable k; both libraries must abort at the same point.
    diff("cfg95 callback failure points", |api, rec| unsafe {
        let mut rng = Rng::new(0x9500);
        let trees = sample_trees(&mut rng);
        for (ti, spec) in trees.iter().enumerate().take(14) {
            let j = build(api, spec);
            for f in [
                JSON_ENCODE_ANY,
                JSON_ENCODE_ANY | json_indent(2),
                JSON_ENCODE_ANY | JSON_SORT_KEYS,
                JSON_ENCODE_ANY | JSON_COMPACT,
                JSON_ENCODE_ANY | JSON_EMBED | json_indent(1),
            ] {
                cb_reset(-1);
                let _ = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                let total = cb_take().len();
                for k in 0..total {
                    cb_reset(k as i64);
                    let r = (api.json_dump_callback)(j, Some(cb_record), ptr::null_mut(), f);
                    let chunks = cb_take();
                    rec.tag_i(&format!("t{ti}.f{f}.k{k}.ret"), r as i64);
                    rec.tag_i(&format!("t{ti}.f{f}.k{k}.n"), chunks.len() as i64);
                    for (i, c) in chunks.iter().enumerate() {
                        rec.tag_bytes(&format!("t{ti}.f{f}.k{k}.c{i}"), c);
                    }
                }
            }
            decref(api, j);
        }
    });
}
