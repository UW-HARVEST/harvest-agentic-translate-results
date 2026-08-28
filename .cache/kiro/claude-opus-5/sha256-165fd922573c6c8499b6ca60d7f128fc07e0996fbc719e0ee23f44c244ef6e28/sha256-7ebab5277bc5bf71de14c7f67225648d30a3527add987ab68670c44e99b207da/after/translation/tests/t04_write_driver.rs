//! Top layer: `write_to_file` and `driver`, including their file side effects.

mod common;

use common::*;
use std::ffi::c_int;
use std::path::{Path, PathBuf};

fn tmpdir() -> PathBuf {
    let d = std::env::temp_dir().join(format!("driver_wtf_{}", std::process::id()));
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Calls `write_to_file` in both libraries against distinct paths and compares
/// the return code and the resulting file bytes.
fn write_case(label: &str, rel: &str, content: Option<&str>) {
    let p = pair();
    let d = tmpdir();
    let cpath = d.join(format!("c_{label}_{rel}"));
    let rpath = d.join(format!("r_{label}_{rel}"));
    let _ = std::fs::remove_file(&cpath);
    let _ = std::fs::remove_file(&rpath);

    let cname = cstr(cpath.to_str().unwrap());
    let rname = cstr(rpath.to_str().unwrap());
    let payload = content.map(cstr);

    unsafe {
        let cptr = payload
            .as_ref()
            .map(|v| v.as_ptr())
            .unwrap_or(std::ptr::null());
        let crc = (p.c.write_to_file)(cname.as_ptr(), cptr);
        let rrc = (p.rs.write_to_file)(rname.as_ptr(), cptr);
        assert_eq!(crc, rrc, "write_to_file({label}) return code differs");

        let cdata = std::fs::read(&cpath).ok();
        let rdata = std::fs::read(&rpath).ok();
        assert_eq!(cdata, rdata, "write_to_file({label}) file bytes differ");
    }
}

#[test]
fn write_to_file_success() {
    let _g = fs_lock();
    write_case("plain", "a.txt", Some("hello\n"));
    write_case("empty", "b.txt", Some(""));
    write_case("multiline", "c.txt", Some("1 2\n3 4\n"));
    write_case("nonl", "d.txt", Some("no trailing newline"));
    write_case("long", "e.txt", Some(&"x".repeat(100_000)));
    write_case("tabs", "f.txt", Some("a\tb\r\nc"));
    write_case("percent", "g.txt", Some("100%d %s %n literal"));
    write_case("highbytes", "h.txt", Some("caf\u{e9} \u{4e2d}\u{6587}"));
}

#[test]
fn write_to_file_null_content() {
    let _g = fs_lock();
    write_case("nullcontent", "i.txt", None);
}

#[test]
fn write_to_file_open_failures() {
    let p = pair();
    let _g = fs_lock();
    let payload = cstr("data");

    // A directory that does not exist -> ENOENT.
    for bad in [
        "/nonexistent_dir_xyz/out.txt",
        "/proc/self/definitely_not_writable",
        "",
        "/",
        "/dev/full",
    ] {
        let name = cstr(bad);
        unsafe {
            let crc = (p.c.write_to_file)(name.as_ptr(), payload.as_ptr());
            let rrc = (p.rs.write_to_file)(name.as_ptr(), payload.as_ptr());
            assert_eq!(crc, rrc, "write_to_file({bad:?}) return code differs");
        }
    }
}

/// Runs `driver` with the process cwd set to `dir`, returning the exit code and
/// the contents of `matrix.txt`.
unsafe fn run_driver(
    f: FnDriver,
    dir: &Path,
    wa: c_int,
    ha: c_int,
    a: &str,
    wb: c_int,
    hb: c_int,
    b: &str,
) -> (c_int, Option<Vec<u8>>) {
    let out = dir.join("matrix.txt");
    let _ = std::fs::remove_file(&out);
    let prev = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir).unwrap();
    let sa = cstr(a);
    let sb = cstr(b);
    let rc = unsafe { f(wa, ha, sa.as_ptr(), wb, hb, sb.as_ptr()) };
    std::env::set_current_dir(prev).unwrap();
    (rc, std::fs::read(&out).ok())
}

fn driver_case(wa: c_int, ha: c_int, a: &str, wb: c_int, hb: c_int, b: &str) {
    let p = pair();
    let _g = fs_lock();
    let root = tmpdir();
    let cdir = root.join("c_driver");
    let rdir = root.join("r_driver");
    std::fs::create_dir_all(&cdir).unwrap();
    std::fs::create_dir_all(&rdir).unwrap();

    unsafe {
        let (crc, cout) = run_driver(p.c.driver, &cdir, wa, ha, a, wb, hb, b);
        let (rrc, rout) = run_driver(p.rs.driver, &rdir, wa, ha, a, wb, hb, b);
        assert_eq!(
            crc, rrc,
            "driver({wa},{ha},{a:?},{wb},{hb},{b:?}) return code differs"
        );
        assert_eq!(
            cout.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            rout.as_ref().map(|v| String::from_utf8_lossy(v).into_owned()),
            "driver({wa},{ha},{a:?},{wb},{hb},{b:?}) matrix.txt differs"
        );
    }
}

#[test]
fn driver_success_paths() {
    driver_case(2, 2, "1 2\n3 4\n", 2, 2, "5 6\n7 8\n");
    driver_case(1, 1, "6\n", 1, 1, "7\n");
    driver_case(3, 2, "1 2 3\n4 5 6\n", 2, 3, "7 8\n9 10\n11 12\n");
    driver_case(1, 3, "1 2 3\n", 3, 1, "4 5 6\n");
    driver_case(3, 1, "1\n2\n3\n", 1, 3, "4 5 6\n");
    driver_case(3, 3, "1 2 3\n4 5 6\n7 8 9\n", 3, 3, "1 0 0\n0 1 0\n0 0 1\n");
    driver_case(2, 2, "-1 -2\n-3 -4\n", 2, 2, "5 -6\n-7 8\n");
    driver_case(2, 2, "0 0\n0 0\n", 2, 2, "0 0\n0 0\n");
    driver_case(4, 4, "1 2 3 4\n5 6 7 8\n9 10 11 12\n13 14 15 16\n",
                4, 4, "1 1 1 1\n2 2 2 2\n3 3 3 3\n4 4 4 4\n");
}

#[test]
fn driver_failure_paths() {
    // mat_a fails to parse.
    driver_case(2, 2, "1 2\n", 2, 2, "5 6\n7 8\n");
    driver_case(3, 2, "1 2\n3 4\n", 2, 3, "1 2\n3 4\n5 6\n");
    // mat_b fails to parse.
    driver_case(2, 2, "1 2\n3 4\n", 2, 2, "5 6\n");
    driver_case(2, 2, "1 2\n3 4\n", 3, 2, "1 2\n3 4\n");
    // Dimension mismatch at multiply time.
    driver_case(2, 2, "1 2\n3 4\n", 3, 3, "1 2 3\n4 5 6\n7 8 9\n");
    driver_case(3, 1, "1 2 3\n", 1, 1, "4\n");
    // Negative / zero dimensions.
    driver_case(-1, 2, "1 2\n3 4\n", 2, 2, "1 2\n3 4\n");
    driver_case(2, -1, "1 2\n3 4\n", 2, 2, "1 2\n3 4\n");
    driver_case(0, 0, "", 0, 0, "");
    driver_case(2, 0, "1 2\n", 2, 2, "1 2\n3 4\n");
    driver_case(0, 2, "1 2\n3 4\n", 2, 0, "1 2\n");
    // Empty inputs.
    driver_case(1, 1, "", 1, 1, "1\n");
    driver_case(1, 1, "1\n", 1, 1, "");
}

#[test]
fn driver_value_quirks() {
    // atoi saturation and int overflow inside the product.
    driver_case(1, 1, "2147483647\n", 1, 1, "2\n");
    driver_case(1, 1, "99999999999999999999\n", 1, 1, "1\n");
    driver_case(1, 1, "-2147483648\n", 1, 1, "-1\n");
    driver_case(2, 2, "abc def\nghi jkl\n", 2, 2, "1 2\n3 4\n");
    driver_case(2, 2, "1x 2y\n3z 4w\n", 2, 2, "1 2\n3 4\n");
    driver_case(1, 1, "  42  \n", 1, 1, "1\n");
    driver_case(1, 1, "+7\n", 1, 1, "+6\n");
}
