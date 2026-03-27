use std::process::Command;

/// Both C and Rust .so are loaded into separate fresh processes to ensure
/// identical allocator state. This is necessary because compute_hash compares
/// raw pointer addresses, making results allocator-state-dependent.
#[test]
fn test_betagamma_fresh_process() {
    let dir = env!("CARGO_MANIFEST_DIR");

    // Build C test binary
    let c_src = format!("{dir}/target/test_c_main.c");
    let c_bin = format!("{dir}/target/test_c_main");
    std::fs::write(&c_src, DRIVER_C).unwrap();
    assert!(
        Command::new("gcc")
            .args([&c_src, "-o", &c_bin,
                   "-L", &format!("{dir}/c_src/build"), "-ltranslated_rust",
                   &format!("-Wl,-rpath,{dir}/c_src/build")])
            .status().unwrap().success(),
        "gcc (C .so) failed"
    );

    // Build Rust cdylib then link a test binary against it
    assert!(
        Command::new("cargo")
            .args(["build", "--lib"])
            .current_dir(dir)
            .status().unwrap().success(),
        "cargo build failed"
    );
    let r_src = format!("{dir}/target/test_r_main.c");
    let r_bin = format!("{dir}/target/test_r_main");
    std::fs::write(&r_src, DRIVER_C).unwrap();
    assert!(
        Command::new("gcc")
            .args([&r_src, "-o", &r_bin,
                   "-L", &format!("{dir}/target/debug"), "-lbetagamma_lib",
                   &format!("-Wl,-rpath,{dir}/target/debug")])
            .status().unwrap().success(),
        "gcc (Rust .so) failed"
    );

    let cases: &[(i32, i32, i32, i32)] = &[
        (0, 0, 0, 0),
        (1, 2, 3, 4),
        (5, 10, 15, 20),
        (7, 3, 1, 9),
        (10, 20, 30, 40),
        (100, 200, 300, 400),
        (9, 8, 7, 6),
        (15, 25, 35, 45),
        (0, 1, 0, 1),
        (3, 3, 3, 3),
        (1, 1, 1, 1),
        (50, 60, 70, 80),
    ];

    for &(a, b, c, d) in cases {
        let args = [a.to_string(), b.to_string(), c.to_string(), d.to_string()];
        let c_val = run_and_parse(&c_bin, &args);
        let r_val = run_and_parse(&r_bin, &args);
        eprintln!("betagamma({a},{b},{c},{d}): C={c_val}, Rust={r_val}");
        assert_eq!(c_val, r_val, "Mismatch for betagamma({a},{b},{c},{d})");
    }
}

fn run_and_parse(bin: &str, args: &[String]) -> i32 {
    let out = Command::new(bin).args(args).output().unwrap();
    assert!(out.status.success(), "{bin} failed: {}", String::from_utf8_lossy(&out.stderr));
    String::from_utf8(out.stdout).unwrap().trim().parse().unwrap()
}

const DRIVER_C: &str = r#"
#include <stdio.h>
extern int betagamma(int,int,int,int);
int main(int argc, char**argv) {
    int a,b,c,d;
    sscanf(argv[1],"%d",&a); sscanf(argv[2],"%d",&b);
    sscanf(argv[3],"%d",&c); sscanf(argv[4],"%d",&d);
    printf("%d\n", betagamma(a,b,c,d));
}
"#;
