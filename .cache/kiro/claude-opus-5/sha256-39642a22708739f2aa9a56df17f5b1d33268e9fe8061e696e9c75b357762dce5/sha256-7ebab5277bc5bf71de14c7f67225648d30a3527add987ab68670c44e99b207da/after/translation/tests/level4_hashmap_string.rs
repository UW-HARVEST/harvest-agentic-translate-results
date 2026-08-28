//! Level 4: string-keyed hash maps (`STBDS_HM_STRING`) in all three arena
//! modes, `stbds_shmode_func`, and the top-level `intput` entry point.

mod harness;

use harness::map::*;
use harness::*;
use std::ffi::c_void;

/// `char *key; int value;` => 16 bytes with 4 bytes of tail padding
const STR_ELEM: usize = 16;

/// Keys must outlive the map: in `STBDS_SH_DEFAULT` mode the library stores the
/// caller's pointer verbatim.
struct Keys(Vec<Vec<u8>>);

impl Keys {
    fn new(names: impl IntoIterator<Item = String>) -> Keys {
        Keys(names.into_iter().map(|s| cstring(&s)).collect())
    }
    fn get(&mut self, i: usize) -> &mut [u8] {
        &mut self.0[i]
    }
    fn len(&self) -> usize {
        self.0.len()
    }
}

fn names(n: usize) -> Vec<String> {
    (0..n).map(|i| format!("test_{i}")).collect()
}

fn run_string_map(shmode: Option<i32>, n: usize, tag: &str) {
    let p = pair();
    let mut m = match shmode {
        None => MapPair::string(&p, STR_ELEM),
        Some(mode) => MapPair::string_mode(&p, STR_ELEM, mode),
    };
    let mut keys = Keys::new(names(n));

    for i in 0..keys.len() {
        let payload = [(i as u8).wrapping_mul(13), 0x5a, 0x00, 0xff];
        m.put(keys.get(i), &payload, &format!("{tag}: shput {i}"));
    }
    for i in 0..keys.len() {
        let t = m.get(keys.get(i), &format!("{tag}: shget {i}"));
        assert!(t >= 0, "{tag}: key {i} should be present");
    }
    // absent keys
    let mut absent = Keys::new((n..n + 15).map(|i| format!("test_{i}")));
    for i in 0..absent.len() {
        let t = m.get(absent.get(i), &format!("{tag}: shget-absent {i}"));
        assert_eq!(t, -1, "{tag}: absent key {i}");
    }
    // overwrite everything (exercises the "key already present" path, which is
    // where stbds_temp_key is refreshed from the stored pointer)
    for i in 0..keys.len() {
        m.put(keys.get(i), &[0x77, 0x66, 0x55, 0x44], &format!("{tag}: reput {i}"));
    }
    // delete every other key, then the rest
    let mut i = 0;
    while i < keys.len() {
        let r = m.del(keys.get(i), 0, &format!("{tag}: shdel {i}"));
        assert_eq!(r, 1, "{tag}: key {i} should have been deleted");
        i += 2;
    }
    let mut i = 1;
    while i < keys.len() {
        let r = m.del(keys.get(i), 0, &format!("{tag}: shdel2 {i}"));
        assert_eq!(r, 1, "{tag}: key {i} should have been deleted");
        i += 2;
    }
    for i in 0..keys.len() {
        let t = m.get(keys.get(i), &format!("{tag}: shget-after-del {i}"));
        assert_eq!(t, -1, "{tag}: key {i} should be gone");
    }
    m.free();
}

#[test]
fn string_map_default_mode() {
    for n in [1, 5, 6, 7, 20, 100] {
        run_string_map(None, n, &format!("SH_DEFAULT n={n}"));
    }
}

#[test]
fn string_map_strdup_mode() {
    for n in [1, 5, 6, 7, 20, 100] {
        run_string_map(Some(STBDS_SH_STRDUP), n, &format!("SH_STRDUP n={n}"));
    }
}

#[test]
fn string_map_arena_mode() {
    for n in [1, 5, 6, 7, 20, 100] {
        run_string_map(Some(STBDS_SH_ARENA), n, &format!("SH_ARENA n={n}"));
    }
}

#[test]
fn string_map_shmode_default() {
    // `sh_new_*` only ever passes STRDUP/ARENA, but STBDS_SH_DEFAULT is a
    // reachable value of the same parameter and behaves like the NULL-seeded map.
    run_string_map(Some(STBDS_SH_DEFAULT), 20, "shmode=SH_DEFAULT");
}

#[test]
fn shmode_func_fresh_table() {
    let p = pair();
    for mode in [
        STBDS_SH_NONE,
        STBDS_SH_DEFAULT,
        STBDS_SH_STRDUP,
        STBDS_SH_ARENA,
    ] {
        for elemsize in [8usize, 16, 24, 40] {
            unsafe {
                let ct = (p.c.shmode_func)(elemsize, mode);
                let rt = (p.r.shmode_func)(elemsize, mode);
                let cs = snap::snap_map(ct, elemsize, snap::KeyKind::Binary);
                let rs = snap::snap_map(rt, elemsize, snap::KeyKind::Binary);
                assert_eq!(cs, rs, "shmode_func(elemsize={elemsize}, mode={mode})");
                assert_eq!(cs.header.length, 1);
                assert_eq!(cs.index.as_ref().unwrap().arena.mode, mode as u8);
                (p.c.hmfree_func)((ct as *mut u8).sub(elemsize) as *mut c_void, elemsize);
                (p.r.hmfree_func)((rt as *mut u8).sub(elemsize) as *mut c_void, elemsize);
            }
        }
    }
}

/// Long keys in arena mode drive `stbds_stralloc`'s block growth and the
/// oversized-block branch from inside the map.
#[test]
fn string_map_arena_long_keys() {
    let p = pair();
    let mut m = MapPair::string_mode(&p, STR_ELEM, STBDS_SH_ARENA);
    let mut keys = Keys::new((0..80).map(|i| format!("k{i}_{}", "q".repeat(i * 11))));
    for i in 0..keys.len() {
        m.put(keys.get(i), &[i as u8], &format!("arena long put {i}"));
    }
    for i in 0..keys.len() {
        assert!(m.get(keys.get(i), &format!("arena long get {i}")) >= 0);
    }
    m.free();
}

#[test]
fn string_map_strdup_churn() {
    let p = pair();
    let mut m = MapPair::string_mode(&p, STR_ELEM, STBDS_SH_STRDUP);
    let mut keys = Keys::new((0..200).map(|i| format!("churn_{i}")));
    let mut live: Vec<usize> = Vec::new();
    let mut s: u64 = 0xfeed_face_dead_beef;
    let mut next = move || {
        s ^= s << 13;
        s ^= s >> 7;
        s ^= s << 17;
        s
    };
    for step in 0..1500u32 {
        let r = next();
        let op = r % 100;
        let i = ((r >> 8) as usize) % keys.len();
        if op < 55 || live.is_empty() {
            m.put(keys.get(i), &[(step & 0xff) as u8], &format!("churn {step} put {i}"));
            if !live.contains(&i) {
                live.push(i);
            }
        } else if op < 85 {
            let idx = ((r >> 32) as usize) % live.len();
            let i = live.swap_remove(idx);
            let got = m.del(keys.get(i), 0, &format!("churn {step} del {i}"));
            assert_eq!(got, 1, "churn {step}: key {i} should have been live");
        } else {
            let t = m.get(keys.get(i), &format!("churn {step} get {i}"));
            assert_eq!(t >= 0, live.contains(&i), "churn {step}: presence of {i}");
        }
    }
    m.free();
}

/// Empty and single-character keys, plus keys differing only past the first
/// bucket-sized prefix.
#[test]
fn string_map_edge_keys() {
    let p = pair();
    let mut m = MapPair::string(&p, STR_ELEM);
    let mut keys = Keys::new(
        [
            "",
            "a",
            "b",
            "ab",
            "ba",
            "aaaaaaaaaaaaaaaa0",
            "aaaaaaaaaaaaaaaa1",
            "\u{7f}",
            "test_0",
            "test_00",
        ]
        .into_iter()
        .map(|s| s.to_string()),
    );
    for i in 0..keys.len() {
        m.put(keys.get(i), &[i as u8, 1, 2, 3], &format!("edge put {i}"));
    }
    for i in 0..keys.len() {
        assert!(m.get(keys.get(i), &format!("edge get {i}")) >= 0);
    }
    for i in 0..keys.len() {
        assert_eq!(m.del(keys.get(i), 0, &format!("edge del {i}")), 1);
    }
    m.free();
}

/// `hmget_key_ts` against a string map.
#[test]
fn string_map_get_ts() {
    let p = pair();
    let mut m = MapPair::string(&p, STR_ELEM);
    let mut keys = Keys::new(names(40));
    for i in 0..keys.len() {
        m.put(keys.get(i), &[i as u8], &format!("ts put {i}"));
    }
    for i in 0..keys.len() {
        let a = m.get_ts(keys.get(i), &format!("ts get_ts {i}"));
        let b = m.get(keys.get(i), &format!("ts get {i}"));
        assert_eq!(a, b);
    }
    m.free();
}

/// Distinct buffers holding equal text: the `strcmp` branch of
/// `stbds_is_key_equal` has to recognise them as the same key, and in
/// `STBDS_SH_DEFAULT` mode the *first* pointer must stay stored.
#[test]
fn string_map_equal_content_distinct_buffers() {
    for shmode in [None, Some(STBDS_SH_STRDUP), Some(STBDS_SH_ARENA)] {
        let p = pair();
        let mut m = match shmode {
            None => MapPair::string(&p, STR_ELEM),
            Some(mode) => MapPair::string_mode(&p, STR_ELEM, mode),
        };
        let tag = format!("dup-content shmode={shmode:?}");
        let mut first = Keys::new(names(30));
        let mut second = Keys::new(names(30));
        for i in 0..first.len() {
            m.put(first.get(i), &[i as u8, 0, 0, 0], &format!("{tag}: put A {i}"));
        }
        for i in 0..second.len() {
            m.put(second.get(i), &[0xee, i as u8, 0, 0], &format!("{tag}: put B {i}"));
        }
        assert_eq!(
            m.snap_c().header.length,
            31,
            "{tag}: equal-content keys must not create new entries"
        );
        for i in 0..second.len() {
            let t = m.get(second.get(i), &format!("{tag}: get B {i}"));
            assert!(t >= 0, "{tag}: key {i} should be found via the other buffer");
        }
        for i in 0..second.len() {
            assert_eq!(m.del(second.get(i), 0, &format!("{tag}: del B {i}")), 1);
        }
        m.free();
    }
}

// --- top-level entry point --------------------------------------------------

/// Child-process worker: `HARVEST_INTPUT=<c|r>:<num>` makes this test load a
/// single library and call `intput`, so that an `assert` abort (which `intput`
/// legitimately does for `num == 9` and `num == 11`) does not take down the
/// whole test binary.
#[test]
fn intput_child() {
    let spec = match std::env::var("HARVEST_INTPUT") {
        Ok(s) => s,
        Err(_) => return,
    };
    let (which, num) = spec.split_once(':').expect("HARVEST_INTPUT=<c|r>:<num>");
    let num: i32 = num.parse().unwrap();
    let path = match which {
        "c" => c_so_path(),
        "r" => rust_so_path(),
        other => panic!("unknown library {other}"),
    };
    let lib = Lib::open("child", &path);
    unsafe {
        (lib.rand_seed)(0x31415926);
        (lib.intput)(num);
    }
    // reached only when intput did not abort
    std::process::exit(0);
}

/// Split a glibc/`assert_fail` message into (program name, text from `lib.c:`
/// onwards).  The `__FILE__` *directory* differs between the CMake build and the
/// Rust translation and is not part of the behaviour under test.
fn split_assert_msg(s: &str) -> (Option<String>, Option<String>) {
    let prog = s.find(": ").map(|i| s[..i].to_string());
    let tail = s.find("lib.c:").map(|i| s[i..].to_string());
    (prog, tail)
}

fn run_intput_child(which: &str, num: i32) -> (Option<i32>, Option<i32>, String) {
    use std::os::unix::process::ExitStatusExt;
    let out = std::process::Command::new(std::env::current_exe().unwrap())
        .args(["intput_child", "--exact", "--nocapture", "--test-threads=1"])
        .env("HARVEST_INTPUT", format!("{which}:{num}"))
        .output()
        .expect("spawn child");
    (
        out.status.code(),
        out.status.signal(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

#[test]
fn intput_matches() {
    for num in [
        0i32, 1, 2, 3, 7, 8, 9, 10, 11, 12, -1, -9, -11, 100, 65536, i32::MAX, i32::MIN,
    ] {
        let (cc, cs, cerr) = run_intput_child("c", num);
        let (rc, rs, rerr) = run_intput_child("r", num);
        assert_eq!(cc, rc, "intput({num}): exit code differs\nC: {cerr}\nR: {rerr}");
        assert_eq!(cs, rs, "intput({num}): termination signal differs\nC: {cerr}\nR: {rerr}");
        let (cprog, ctail) = split_assert_msg(&cerr);
        let (rprog, rtail) = split_assert_msg(&rerr);
        assert_eq!(ctail, rtail, "intput({num}): assertion message differs");
        if ctail.is_some() {
            assert_eq!(cprog, rprog, "intput({num}): assert program name differs");
        }
        // 9 and 11 must abort: the C code asserts hmget(intmap,num)==7 after
        // overwriting that key.
        if num == 9 || num == 11 {
            assert_eq!(cs, Some(6), "intput({num}) was expected to SIGABRT");
        } else {
            assert_eq!(cc, Some(0), "intput({num}) was expected to succeed");
        }
    }
}

/// Re-implements `intput`'s body through the exported primitives so the
/// resulting table can actually be compared structurally.
#[test]
fn intput_body_replayed() {
    let p = pair();
    for num in [0i32, 1, 7, 9, 11, 12, -1, -11, 100, i32::MAX, i32::MIN] {
        let mut m = MapPair::binary(&p, 8, 4);
        for (k, v) in [(num, 7), (11, 3), (9, num)] {
            let mut kb = k.to_ne_bytes().to_vec();
            m.put(&mut kb, &v.to_ne_bytes(), &format!("intput({num}) put {k}"));
        }
        for k in [9, 11, num] {
            let mut kb = k.to_ne_bytes().to_vec();
            m.get(&mut kb, &format!("intput({num}) get {k}"));
        }
        m.free();
    }
}
