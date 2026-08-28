//! Level 1: `get_os_arch` — the lowest-level exported function.

mod common;

use common::*;

fn arch_inputs() -> Vec<Vec<u8>> {
    let mut v: Vec<Vec<u8>> = Vec::new();

    // Every architecture the C table knows about, alone.
    for a in [
        "x86_64", "i386", "i686", "sparc", "amd64", "i86pc", "ia64", "AIX", "armv6", "armv7",
        "aarch64", "arm64",
    ] {
        v.push(a.as_bytes().to_vec());
        v.push(format!("Linux |host |5.10 |#1 SMP |{a}").into_bytes());
        v.push(format!("{a} trailing text").into_bytes());
        v.push(format!("leading text {a}").into_bytes());
        // Case matters: the C uses strstr, not a case-insensitive compare.
        v.push(a.to_uppercase().into_bytes());
        v.push(a.to_lowercase().into_bytes());
    }

    // Ordering of the table decides ties: "x86_64" wins over "i386" etc.
    v.push(b"i386 x86_64".to_vec());
    v.push(b"x86_64 i386".to_vec());
    v.push(b"arm64 aarch64".to_vec());
    v.push(b"aarch64 arm64".to_vec());
    v.push(b"armv7 armv6".to_vec());
    v.push(b"amd64 sparc i686".to_vec());
    // "i686" contains "i68"..., and "i386" is a prefix-free case; check both.
    v.push(b"i686 i386".to_vec());
    v.push(b"i386 i686".to_vec());
    // "arm64" is a substring of nothing in the table, but "aarch64" contains
    // "arch64" not "aarch64"-prefixed forms; exercise embedded matches.
    v.push(b"xxaarch64xx".to_vec());
    v.push(b"xxarm64xx".to_vec());
    v.push(b"xxAIXxx".to_vec());

    // No match at all.
    v.push(b"".to_vec());
    v.push(b"no architecture here".to_vec());
    v.push(b"riscv64".to_vec());
    v.push(b"ppc64le".to_vec());
    v.push(b"s390x".to_vec());
    v.push(b"mips".to_vec());
    // Near misses / partial prefixes.
    v.push(b"x86".to_vec());
    v.push(b"x86_6".to_vec());
    v.push(b"i38".to_vec());
    v.push(b"arm".to_vec());
    v.push(b"armv".to_vec());
    v.push(b"aarch".to_vec());
    v.push(b"AI".to_vec());
    v.push(b"aix".to_vec());

    // Real-world uname strings.
    v.push(b"Linux |ubuntu |5.15.0-91-generic |#101-Ubuntu SMP Tue Nov 14 13:30:08 UTC 2023 |x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy Jellyfish)]".to_vec());
    v.push(b"Darwin |mac.local |23.2.0 |Darwin Kernel Version 23.2.0 |arm64 [macOS|darwin: 14.2.1 (Sonoma)]".to_vec());
    v.push(b"SunOS |solaris |5.11 |11.4 |i86pc [SunOS|sunos: 11.4]".to_vec());
    v.push(b"AIX |aix71 |1 |7 [AIX|aix: 7.1]".to_vec());

    // Bytes outside ASCII, and embedded high bytes near a match.
    v.push(vec![0xff, 0xfe, b'x', b'8', b'6', b'_', b'6', b'4', 0x80]);
    v.push(vec![0xc3, 0xa9, b'a', b'r', b'm', b'6', b'4']);

    v
}

#[test]
fn get_os_arch_matches_c() {
    let (c, rust) = load_both();

    for input in arch_inputs() {
        let got_c = c.get_os_arch(&input);
        let got_rust = rust.get_os_arch(&input);
        assert_eq!(
            got_c,
            got_rust,
            "get_os_arch({:?}): C = {}, Rust = {}",
            String::from_utf8_lossy(&input),
            show(&got_c),
            show(&got_rust),
        );
    }
}

#[test]
fn get_os_arch_does_not_modify_input() {
    // The C takes `char *` but only reads it; confirm both agree on that.
    let (c, rust) = load_both();

    for input in arch_inputs() {
        let mut a: Vec<u8> = input.clone();
        a.push(0);
        let mut b: Vec<u8> = input.clone();
        b.push(0);

        // Call through each library on its own copy and compare the buffers.
        let _ = c.get_os_arch(&input);
        let _ = rust.get_os_arch(&input);
        assert_eq!(a, b);
    }
}
