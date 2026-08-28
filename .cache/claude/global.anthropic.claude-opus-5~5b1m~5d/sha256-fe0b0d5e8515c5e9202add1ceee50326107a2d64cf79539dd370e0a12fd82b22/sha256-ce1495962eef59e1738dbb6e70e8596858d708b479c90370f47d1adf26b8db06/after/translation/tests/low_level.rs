//! Phase B, rows 1-5 of `CONFIGS.md`: the lowest-level exported entry points
//! (`os_calloc`, `os_realloc`, `os_strdup`, `merror`, `FreeAlertData`).

mod common;

use common::*;
use std::ffi::{c_char, c_int, c_void};

/* ---------------- CONFIGS row 1 ---------------- */

#[test]
fn cfg01_os_calloc_random() {
    let mut rng = Rng::new(0x0101);
    unsafe {
        for _ in 0..400 {
            let num = rng.below(70) as usize;
            let size = rng.below(70) as usize;
            let cp = (cc().os_calloc)(num, size);
            let rp = (rs().os_calloc)(num, size);
            assert!(!cp.is_null(), "C os_calloc({num},{size}) returned NULL");
            assert!(!rp.is_null(), "RUST os_calloc({num},{size}) returned NULL");
            let n = num.saturating_mul(size);
            let cb = std::slice::from_raw_parts(cp as *const u8, n);
            let rb = std::slice::from_raw_parts(rp as *const u8, n);
            assert!(cb.iter().all(|&b| b == 0), "C os_calloc not zeroed");
            assert_eq!(cb, rb, "os_calloc({num},{size}) contents differ");
            free(cp);
            free(rp);
        }
        // explicit boundary cases
        for (num, size) in [(0, 0), (0, 1), (1, 0), (1, 1), (4096, 1), (1, 4096)] {
            let cp = (cc().os_calloc)(num, size);
            let rp = (rs().os_calloc)(num, size);
            assert_eq!(cp.is_null(), rp.is_null(), "os_calloc({num},{size}) nullness");
            free(cp);
            free(rp);
        }
    }
}

/* ---------------- CONFIGS row 2 ---------------- */

#[test]
fn cfg02_os_realloc_random() {
    let mut rng = Rng::new(0x0202);
    unsafe {
        // NULL ptr behaves like malloc, including size 0.
        for _ in 0..200 {
            let n = rng.below(129) as usize;
            let cp = (cc().os_realloc)(std::ptr::null_mut(), n);
            let rp = (rs().os_realloc)(std::ptr::null_mut(), n);
            assert!(!cp.is_null() && !rp.is_null(), "os_realloc(NULL,{n})");
            free(cp);
            free(rp);
        }
        // Grow / shrink an existing block through a random size sequence and
        // check the retained prefix survives identically in both.
        for _ in 0..80 {
            let mut cur = 1 + rng.below(32) as usize;
            let mut cp = (cc().os_realloc)(std::ptr::null_mut(), cur);
            let mut rp = (rs().os_realloc)(std::ptr::null_mut(), cur);
            let mut model: Vec<u8> = (0..cur).map(|_| rng.below(256) as u8).collect();
            std::ptr::copy_nonoverlapping(model.as_ptr(), cp as *mut u8, cur);
            std::ptr::copy_nonoverlapping(model.as_ptr(), rp as *mut u8, cur);
            for _ in 0..6 {
                let next = 1 + rng.below(256) as usize;
                cp = (cc().os_realloc)(cp, next);
                rp = (rs().os_realloc)(rp, next);
                assert!(!cp.is_null() && !rp.is_null(), "os_realloc grow to {next}");
                let keep = cur.min(next);
                let cb = std::slice::from_raw_parts(cp as *const u8, keep);
                let rb = std::slice::from_raw_parts(rp as *const u8, keep);
                assert_eq!(cb, &model[..keep], "C os_realloc lost data");
                assert_eq!(rb, &model[..keep], "RUST os_realloc lost data");
                model.truncate(keep);
                model.resize(next, 0);
                for i in keep..next {
                    let b = rng.below(256) as u8;
                    model[i] = b;
                    *(cp as *mut u8).add(i) = b;
                    *(rp as *mut u8).add(i) = b;
                }
                cur = next;
            }
            free(cp);
            free(rp);
        }
    }
}

/* ---------------- CONFIGS row 3 ---------------- */

#[test]
fn cfg03_os_strdup_random() {
    let mut rng = Rng::new(0x0303);
    unsafe {
        let mut cases: Vec<Vec<u8>> = vec![vec![], b"a".to_vec(), b"hello world".to_vec()];
        cases.push(vec![0xffu8; 512]);
        cases.push(vec![0x80u8; 1]);
        for _ in 0..300 {
            cases.push(rng.raw_line(512));
        }
        for s in &cases {
            let cs_ = cbytes(s);
            let cp = (cc().os_strdup)(cs_.as_ptr());
            let rp = (rs().os_strdup)(cs_.as_ptr());
            assert!(!cp.is_null() && !rp.is_null());
            let cn = strlen(cp);
            let rn = strlen(rp);
            assert_eq!(cn, s.len(), "C os_strdup length");
            assert_eq!(rn, s.len(), "RUST os_strdup length");
            let cb = std::slice::from_raw_parts(cp as *const u8, cn + 1);
            let rb = std::slice::from_raw_parts(rp as *const u8, rn + 1);
            assert_eq!(cb, rb, "os_strdup contents differ");
            assert_ne!(cp, cs_.as_ptr() as *mut c_char, "must be a fresh copy");
            free(cp as *mut c_void);
            free(rp as *mut c_void);
        }
    }
}

/* ---------------- CONFIGS row 4 ---------------- */

const FSTAT_ERROR: &[u8] =
    b"(1118): Could not retrieve information of file '%s' due to [(%d)-(%s)].";
const FSEEK_ERROR: &[u8] = b"(1116): Could not set position in file '%s' due to [(%d)-(%s)].";

#[test]
fn cfg04_merror_random() {
    let _g = guard();
    let mut rng = Rng::new(0x0404);
    let templates: Vec<&[u8]> = vec![
        FSTAT_ERROR,
        FSEEK_ERROR,
        b"%s|%d|%s",
        b"no substitutions at all",
        b"%s",
        b"[%d]",
        b"%.3s %5d %-10s|",
        // long template so that the 256-byte snprintf buffer truncates
        b"AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA %s %d %s",
    ];
    unsafe {
        for i in 0..400 {
            let tpl = *rng.pick(&templates);
            let name: Vec<u8> = if i % 17 == 0 {
                vec![b'N'; 300]
            } else {
                rng.token(80)
            };
            let err = rng.range_i32(-3, 200);
            let msg: Vec<u8> = if i % 13 == 0 {
                let p = strerror(err);
                let n = strlen(p);
                std::slice::from_raw_parts(p as *const u8, n).to_vec()
            } else {
                rng.token(60)
            };
            let tplc = cbytes(tpl);
            let namec = cbytes(&name);
            let msgc = cbytes(&msg);

            let (_, cerr) = capture_stderr(|| {
                (cc().merror)(tplc.as_ptr(), namec.as_ptr(), err, msgc.as_ptr())
            });
            let (_, rerr) = capture_stderr(|| {
                (rs().merror)(tplc.as_ptr(), namec.as_ptr(), err, msgc.as_ptr())
            });
            assert_eq!(
                cerr,
                rerr,
                "merror differs for template {:?} name_len={} err={}\nC  ={:?}\nRUST={:?}",
                String::from_utf8_lossy(tpl),
                name.len(),
                err,
                String::from_utf8_lossy(&cerr),
                String::from_utf8_lossy(&rerr)
            );
        }
    }
}

/* ---------------- CONFIGS row 5 ---------------- */

unsafe fn make_alert(api: &Api, mask: u32, rng: &mut Rng) -> *mut alert_data {
    let p = malloc(std::mem::size_of::<alert_data>()) as *mut alert_data;
    std::ptr::write_bytes(p as *mut u8, 0, std::mem::size_of::<alert_data>());
    (*p).rule = rng.u32();
    (*p).level = rng.u32();
    (*p).srcport = rng.i32();
    (*p).dstport = rng.i32();
    let fields: [*mut *mut c_char; 9] = [
        &mut (*p).alertid,
        &mut (*p).date,
        &mut (*p).location,
        &mut (*p).comment,
        &mut (*p).group,
        &mut (*p).srcip,
        &mut (*p).dstip,
        &mut (*p).user,
        &mut (*p).filename,
    ];
    for (i, f) in fields.iter().enumerate() {
        if mask & (1 << i) != 0 {
            let s = cbytes(&rng.token(40));
            **f = (api.os_strdup)(s.as_ptr());
        }
    }
    p
}

#[test]
fn cfg05_freealertdata_field_subsets() {
    let mut rng = Rng::new(0x0505);
    unsafe {
        // all 2^9 subsets of the nine owned char* fields, for both libraries
        for mask in 0u32..512 {
            let cp = make_alert(cc(), mask, &mut rng);
            let rp = make_alert(rs(), mask, &mut rng);
            (cc().FreeAlertData)(cp);
            (rs().FreeAlertData)(rp);
        }
        // cross-check: freeing a record built by one library with the other
        // library's FreeAlertData must also work (same glibc allocator).
        for mask in [0u32, 0x1ff, 0x155, 0x0aa] {
            let cp = make_alert(rs(), mask, &mut rng);
            (cc().FreeAlertData)(cp);
            let rp = make_alert(cc(), mask, &mut rng);
            (rs().FreeAlertData)(rp);
        }
    }
}

/* -------- struct-layout parity (ABI) -------- */

#[test]
fn cfg05b_struct_layout_matches_c() {
    use std::mem::offset_of;
    // Values taken from a C probe compiled against c_src/include on this target:
    //   sizeof(alert_data)=96  off 0,4,8,16,24,32,40,48,56,64,72,80,88
    //   sizeof(file_queue)=440 off 0,8,12,16,20,24,288,296
    //   sizeof(struct stat)=144 st_size@48 st_mtim@88
    //   sizeof(struct tm)=56 tm_mon@16 tm_year@20
    assert_eq!(std::mem::size_of::<alert_data>(), 96);
    assert_eq!(std::mem::align_of::<alert_data>(), 8);
    assert_eq!(offset_of!(alert_data, rule), 0);
    assert_eq!(offset_of!(alert_data, level), 4);
    assert_eq!(offset_of!(alert_data, alertid), 8);
    assert_eq!(offset_of!(alert_data, date), 16);
    assert_eq!(offset_of!(alert_data, location), 24);
    assert_eq!(offset_of!(alert_data, comment), 32);
    assert_eq!(offset_of!(alert_data, group), 40);
    assert_eq!(offset_of!(alert_data, srcip), 48);
    assert_eq!(offset_of!(alert_data, srcport), 56);
    assert_eq!(offset_of!(alert_data, dstip), 64);
    assert_eq!(offset_of!(alert_data, dstport), 72);
    assert_eq!(offset_of!(alert_data, user), 80);
    assert_eq!(offset_of!(alert_data, filename), 88);

    assert_eq!(std::mem::size_of::<stat_t>(), 144);
    assert_eq!(offset_of!(stat_t, st_size), 48);
    assert_eq!(offset_of!(stat_t, st_mtim), 88);

    assert_eq!(std::mem::size_of::<file_queue>(), 440);
    assert_eq!(std::mem::align_of::<file_queue>(), 8);
    assert_eq!(offset_of!(file_queue, last_change), 0);
    assert_eq!(offset_of!(file_queue, year), 8);
    assert_eq!(offset_of!(file_queue, day), 12);
    assert_eq!(offset_of!(file_queue, flags), 16);
    assert_eq!(offset_of!(file_queue, mon), 20);
    assert_eq!(offset_of!(file_queue, file_name), 24);
    assert_eq!(offset_of!(file_queue, fp), 288);
    assert_eq!(offset_of!(file_queue, f_status), 296);

    assert_eq!(std::mem::size_of::<tm>(), 56);
    assert_eq!(offset_of!(tm, tm_mon), 16);
    assert_eq!(offset_of!(tm, tm_year), 20);
    // `Init_FileQueue` must not write outside the struct: run it on a padded
    // buffer and check the guard bytes.
    let _g = guard();
    write_file(ALERTS_DAILY, b"");
    unsafe {
        for api in [cc(), rs()] {
            let mut buf = vec![0xABu8; std::mem::size_of::<file_queue>() + 64];
            let q = buf.as_mut_ptr() as *mut file_queue;
            std::ptr::write_bytes(q as *mut u8, 0, std::mem::size_of::<file_queue>());
            let t = tm::new(1, 0, 100);
            let rc = (api.Init_FileQueue)(q, &t, 0);
            assert_eq!(rc, 0, "{}", api.name);
            if !(*q).fp.is_null() {
                fclose((*q).fp);
            }
            assert!(
                buf[std::mem::size_of::<file_queue>()..].iter().all(|&b| b == 0xAB),
                "{} wrote past the end of file_queue",
                api.name
            );
        }
    }
    remove_file(ALERTS_DAILY);
}

/* -------- sub-process worker (unused here, keeps the harness uniform) -------- */

#[test]
fn zz_subprocess_worker() {
    let Some(action) = worker_action() else {
        return;
    };
    let (api, rest) = worker_api(&action);
    unsafe {
        match rest {
            "calloc_fail" => {
                let p = (api.os_calloc)(usize::MAX, usize::MAX);
                emit(&format!("returned {:?}", p));
            }
            "realloc_fail" => {
                let p = (api.os_realloc)(std::ptr::null_mut(), usize::MAX);
                emit(&format!("returned {:?}", p));
            }
            "realloc_zero" => {
                let base = (api.os_realloc)(std::ptr::null_mut(), 16);
                let p = (api.os_realloc)(base, 0);
                emit(&format!("returned null={}", p.is_null()));
            }
            "strdup_null" => {
                let p = (api.os_strdup)(std::ptr::null());
                emit(&format!("returned {:?}", p));
            }
            other => panic!("unknown worker action {other:?}"),
        }
    }
    std::process::exit(0);
}

/* ---------------- ERRORS rows 1-3 (in this file, they use os_*) ---------------- */

#[test]
fn err01_os_calloc_alloc_failure_exits() {
    diff_worker("calloc_fail");
    let c = run_worker("c:calloc_fail");
    assert_eq!(c.status, Some(1), "C os_calloc OOM must exit(EXIT_FAILURE)");
    assert!(
        c.stderr.contains("Memory allocation failed in os_calloc"),
        "unexpected stderr: {:?}",
        c.stderr
    );
    let r = run_worker("rust:calloc_fail");
    assert!(
        r.stderr.contains("Memory allocation failed in os_calloc"),
        "unexpected stderr: {:?}",
        r.stderr
    );
}

#[test]
fn err02_os_realloc_alloc_failure_exits() {
    diff_worker("realloc_fail");
    let c = run_worker("c:realloc_fail");
    assert_eq!(c.status, Some(1));
    assert!(c.stderr.contains("Memory allocation failed in os_realloc"));
    // realloc(ptr, 0): glibc frees and returns NULL -> os_realloc must exit(1)
    diff_worker("realloc_zero");
}

#[test]
fn err03_os_strdup_null_exits() {
    diff_worker("strdup_null");
    let c = run_worker("c:strdup_null");
    assert_eq!(c.status, Some(1));
    assert!(c.stderr.contains("NULL string passed to os_strdup"));
}

#[test]
fn err04_os_strdup_oom_documented() {
    // `shared.h:37` only triggers when glibc `strdup` itself returns NULL. Both
    // implementations call the very same glibc `strdup`, so the branch is
    // byte-identical by construction; it cannot be provoked deterministically
    // without an allocator-failure injector. Documented in ERRORS.md row 4.
    let mut rng = Rng::new(4);
    unsafe {
        for _ in 0..32 {
            let s = cbytes(&rng.token(64));
            let a = (cc().os_strdup)(s.as_ptr());
            let b = (rs().os_strdup)(s.as_ptr());
            assert!(!a.is_null() && !b.is_null());
            free(a as *mut c_void);
            free(b as *mut c_void);
        }
    }
}

#[test]
fn err28_merror_truncation() {
    let _g = guard();
    unsafe {
        for len in [200usize, 240, 255, 256, 300, 1000] {
            let name = vec![b'X'; len];
            let tpl = cbytes(FSTAT_ERROR);
            let namec = cbytes(&name);
            let msgc = cbytes(b"Bad file descriptor");
            let (_, cerr) =
                capture_stderr(|| (cc().merror)(tpl.as_ptr(), namec.as_ptr(), 9, msgc.as_ptr()));
            let (_, rerr) =
                capture_stderr(|| (rs().merror)(tpl.as_ptr(), namec.as_ptr(), 9, msgc.as_ptr()));
            assert_eq!(cerr, rerr, "merror truncation differs at len {len}");
            // 255 formatted bytes + '\n'
            assert!(cerr.len() <= 256, "unexpected length {}", cerr.len());
        }
    }
}

#[test]
fn err29_merror_null_args() {
    let _g = guard();
    unsafe {
        let tpl = cbytes(FSTAT_ERROR);
        let good = cbytes(b"alerts.log");
        for (n, m) in [
            (std::ptr::null::<c_char>(), good.as_ptr()),
            (good.as_ptr(), std::ptr::null()),
            (std::ptr::null(), std::ptr::null()),
        ] {
            let (_, cerr) = capture_stderr(|| (cc().merror)(tpl.as_ptr(), n, 2, m));
            let (_, rerr) = capture_stderr(|| (rs().merror)(tpl.as_ptr(), n, 2, m));
            assert_eq!(cerr, rerr, "merror(NULL args) differs");
            assert!(String::from_utf8_lossy(&cerr).contains("(null)"));
        }
        // NULL template: glibc snprintf with a NULL format is UB, so it is not
        // exercised here (documented, not an input the library ever produces).
        let _: c_int = 0;
    }
}
