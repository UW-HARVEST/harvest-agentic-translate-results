//! Phase B — valid-path differential tests for `parse_uname_string`
//! (and the composed pipeline). Covers CONFIGS.md rows 13-42.

mod common;
use common::*;

// ---------------------------------------------------------------------------
// generators for the input shapes the C code branches on
// ---------------------------------------------------------------------------

/// Version-string shapes D1..D6 from CONFIGS.md.
#[derive(Copy, Clone, Debug)]
enum Shape {
    NonNumeric,   // D1
    Major,        // D2
    MajorMinor,   // D3
    MMBuild,      // D4
    MMBuildRev,   // D5
    Weird,        // D6: multi-digit / leading zeros / very long
}

fn digits(rng: &mut Rng, n: usize) -> Vec<u8> {
    (0..n).map(|_| b'0' + rng.below(10) as u8).collect()
}

fn version(rng: &mut Rng, shape: Shape) -> Vec<u8> {
    match shape {
        Shape::NonNumeric => {
            let words: &[&[u8]] = &[
                b"rolling",
                b"unstable",
                b"sid",
                b"v1.2",
                b".1.2",
                b"-1",
                b"",
                b" 1.2",
                b"a1.2.3",
                b"x",
            ];
            rng.pick(words).to_vec()
        }
        Shape::Major => {
            let n = rng.range(1, 3);
            digits(rng, n)
        }
        Shape::MajorMinor => {
            let a = rng.range(1, 3);
            let mut v = digits(rng, a);
            v.push(b'.');
            let b = rng.range(1, 3);
            let d = digits(rng, b);
            v.extend_from_slice(&d);
            v
        }
        Shape::MMBuild => {
            let mut v = version(rng, Shape::MajorMinor);
            v.push(b'.');
            let n = rng.range(1, 6);
            let d = digits(rng, n);
            v.extend_from_slice(&d);
            v
        }
        Shape::MMBuildRev => {
            let mut v = version(rng, Shape::MMBuild);
            let extra = rng.range(1, 4);
            for _ in 0..extra {
                v.push(b'.');
                let n = rng.range(1, 5);
                let d = digits(rng, n);
                v.extend_from_slice(&d);
            }
            v
        }
        Shape::Weird => {
            let choices = 6;
            match rng.below(choices) {
                0 => b"0".to_vec(),
                1 => b"00.00.00".to_vec(),
                2 => b"007.008.009".to_vec(),
                3 => {
                    // 40-digit components
                    let mut v = digits(rng, 40);
                    v.push(b'.');
                    let d = digits(rng, 40);
                    v.extend_from_slice(&d);
                    v.push(b'.');
                    let d2 = digits(rng, 40);
                    v.extend_from_slice(&d2);
                    v
                }
                4 => b"1........".to_vec(),
                _ => b"9.9.9.9.9.9.9.9.9.9".to_vec(),
            }
        }
    }
}

const ALL_SHAPES: [Shape; 6] = [
    Shape::NonNumeric,
    Shape::Major,
    Shape::MajorMinor,
    Shape::MMBuild,
    Shape::MMBuildRev,
    Shape::Weird,
];

fn name_word(rng: &mut Rng) -> Vec<u8> {
    let words: &[&[u8]] = &[
        b"Microsoft Windows Server 2019",
        b"Linux",
        b"Darwin",
        b"myhost",
        b"a",
        b"",
        b"Windows 10 Pro",
        b"SunOS 5.11",
        b"host.example.com",
    ];
    rng.pick(words).to_vec()
}

fn os_word(rng: &mut Rng) -> Vec<u8> {
    let words: &[&[u8]] = &[
        b"Ubuntu",
        b"CentOS Linux",
        b"Debian GNU/Linux",
        b"Mac OS X",
        b"AIX",
        b"o",
        b"",
        b"Amazon Linux",
    ];
    rng.pick(words).to_vec()
}

fn codename_word(rng: &mut Rng) -> Vec<u8> {
    let words: &[&[u8]] = &[
        b"Jammy Jellyfish",
        b"focal",
        b"Big Sur",
        b"",
        b"x",
        b"Core (Core)",
    ];
    rng.pick(words).to_vec()
}

/// `<name> [Ver: <version>]` — the Windows branch (B1).
fn build_windows(name: &[u8], version: &[u8], close: bool) -> Vec<u8> {
    let mut v = name.to_vec();
    v.extend_from_slice(b" [Ver: ");
    v.extend_from_slice(version);
    if close {
        v.push(b']');
    }
    v
}

/// `<prefix> [<os>[|<plat>][: <version>[ (<codename>)]]]` — the Unix branch (B2).
fn build_unix(
    prefix: &[u8],
    os: &[u8],
    plat: Option<&[u8]>,
    version: Option<&[u8]>,
    codename: Option<&[u8]>,
    close: bool,
) -> Vec<u8> {
    let mut v = prefix.to_vec();
    v.extend_from_slice(b" [");
    v.extend_from_slice(os);
    if let Some(p) = plat {
        v.push(b'|');
        v.extend_from_slice(p);
    }
    if let Some(ver) = version {
        v.extend_from_slice(b": ");
        v.extend_from_slice(ver);
        if let Some(c) = codename {
            v.extend_from_slice(b" (");
            v.extend_from_slice(c);
            v.push(b')');
        }
    }
    if close {
        v.push(b']');
    }
    v
}

/// Run one input against both poison values (all-NULL and 0xAA-filled `os_data`).
fn both_poisons(ctx: &str, uname: &[u8]) {
    diff_parse_uname(ctx, uname, 0x00);
    diff_parse_uname(ctx, uname, 0xAA);
}

// ---------------------------------------------------------------------------
// Rows 13-23: the Windows (" [Ver: ") branch
// ---------------------------------------------------------------------------

fn windows_shape_row(row: &str, seed: u64, shape: Shape) {
    let mut rng = Rng::new(seed);
    for iter in 0..1500 {
        let n = name_word(&mut rng);
        let v = version(&mut rng, shape);
        let u = build_windows(&n, &v, true);
        both_poisons(&format!("{row} {shape:?} iter={iter}"), &u);
    }
}

#[test]
fn row13_windows_non_numeric_version() {
    windows_shape_row("row13", 0x3013, Shape::NonNumeric);
}

#[test]
fn row14_windows_major_only() {
    windows_shape_row("row14", 0x3014, Shape::Major);
}

#[test]
fn row15_windows_major_minor() {
    windows_shape_row("row15", 0x3015, Shape::MajorMinor);
}

#[test]
fn row16_windows_major_minor_build() {
    windows_shape_row("row16", 0x3016, Shape::MMBuild);
}

#[test]
fn row17_windows_multi_component_build() {
    windows_shape_row("row17", 0x3017, Shape::MMBuildRev);
}

#[test]
fn row18_windows_weird_numeric_components() {
    windows_shape_row("row18", 0x3018, Shape::Weird);
}

/// Row 19: arch text present in the Windows branch — `os_arch` must stay unset.
#[test]
fn row19_windows_ignores_arch() {
    let mut rng = Rng::new(0x3019);
    for arch in ARCHS.iter() {
        for shape in ALL_SHAPES {
            for _ in 0..40 {
                let v = version(&mut rng, shape);
                let mut name = Vec::new();
                name.extend_from_slice(b"Windows ");
                name.extend_from_slice(arch.as_bytes());
                name.extend_from_slice(b" build");
                let u = build_windows(&name, &v, true);
                both_poisons(&format!("row19 {arch} {shape:?}"), &u);

                // arch after the marker too
                let mut v2 = v.clone();
                v2.extend_from_slice(b" ");
                v2.extend_from_slice(arch.as_bytes());
                let u2 = build_windows(b"Windows", &v2, true);
                both_poisons(&format!("row19b {arch} {shape:?}"), &u2);
            }
        }
    }
}

/// Row 20: a `|` inside the Windows name part — ignored, `os_platform` is
/// always `"windows"` in this branch.
#[test]
fn row20_windows_pipe_in_name() {
    let mut rng = Rng::new(0x3020);
    for shape in ALL_SHAPES {
        for _ in 0..300 {
            let v = version(&mut rng, shape);
            let names: &[&[u8]] = &[
                b"Windows|win32",
                b"|",
                b"a|b|c",
                b"Windows 10|",
                b"|Windows",
            ];
            let n = rng.pick(names).to_vec();
            let u = build_windows(&n, &v, true);
            both_poisons("row20", &u);
        }
    }
}

/// Row 21: `" [Ver: "` twice — the first `strstr` hit wins.
#[test]
fn row21_windows_marker_twice() {
    let mut rng = Rng::new(0x3021);
    for shape in ALL_SHAPES {
        for _ in 0..300 {
            let v1 = version(&mut rng, shape);
            let v2 = version(&mut rng, shape);
            let mut u = build_windows(b"Win", &v1, false);
            u.extend_from_slice(b" [Ver: ");
            u.extend_from_slice(&v2);
            u.push(b']');
            both_poisons("row21", &u);
        }
    }
}

/// Row 22: both markers present, in both orders — `" [Ver: "` always wins
/// because it is tested first.
#[test]
fn row22_both_markers_ver_wins() {
    let mut rng = Rng::new(0x3022);
    for shape in ALL_SHAPES {
        for _ in 0..300 {
            let v = version(&mut rng, shape);

            // " [" first, then " [Ver: "
            let mut a = Vec::new();
            a.extend_from_slice(b"Linux [Ubuntu: 22.04] x86_64 [Ver: ");
            a.extend_from_slice(&v);
            a.push(b']');
            both_poisons("row22 bracket-then-ver", &a);

            // " [Ver: " first, then " ["
            let mut b = Vec::new();
            b.extend_from_slice(b"Win [Ver: ");
            b.extend_from_slice(&v);
            b.extend_from_slice(b"] amd64 [Ubuntu: 1.2 (x)]");
            both_poisons("row22 ver-then-bracket", &b);

            // the two markers overlapping: " [Ver: " *is* a " [" match, so the
            // Ver test must fire even when " [" appears earlier in the same run
            let mut c = Vec::new();
            c.extend_from_slice(b"x [Ver: ");
            c.extend_from_slice(&v);
            c.extend_from_slice(b" [Ver: 1.2]");
            both_poisons("row22 nested", &c);
        }
    }
}

/// Row 23: no trailing `]` — the last real byte is eaten instead.
#[test]
fn row23_windows_unterminated() {
    let mut rng = Rng::new(0x3023);
    for shape in ALL_SHAPES {
        for _ in 0..400 {
            let v = version(&mut rng, shape);
            let n = name_word(&mut rng);
            let u = build_windows(&n, &v, false);
            both_poisons("row23", &u);
        }
    }
    // exact boundary: nothing at all after the marker
    both_poisons("row23 empty-after-marker", b"Windows [Ver: ");
    both_poisons("row23 marker-only", b" [Ver: ");
    both_poisons("row23 marker-plus-one", b" [Ver: ]");
    both_poisons("row23 marker-plus-two", b" [Ver: 1");
}

// ---------------------------------------------------------------------------
// Rows 24-36: the Unix (" [") branch
// ---------------------------------------------------------------------------

fn unix_row(row: &str, seed: u64, with_plat: bool, with_codename: bool, with_version: bool) {
    let mut rng = Rng::new(seed);
    for shape in ALL_SHAPES {
        for iter in 0..350 {
            let prefix = name_word(&mut rng);
            let os = os_word(&mut rng);
            let plat: Option<Vec<u8>> = if with_plat {
                Some(rng.pick(&[b"ubuntu".as_slice(), b"centos", b"darwin", b"", b"p"]).to_vec())
            } else {
                None
            };
            let ver = if with_version {
                Some(version(&mut rng, shape))
            } else {
                None
            };
            let code = if with_codename && with_version {
                Some(codename_word(&mut rng))
            } else {
                None
            };
            let u = build_unix(
                &prefix,
                &os,
                plat.as_deref(),
                ver.as_deref(),
                code.as_deref(),
                true,
            );
            both_poisons(&format!("{row} {shape:?} iter={iter}"), &u);
        }
    }
}

#[test]
fn row24_unix_version_and_codename() {
    unix_row("row24", 0x3024, false, true, true);
}

#[test]
fn row25_unix_version_no_codename() {
    unix_row("row25", 0x3025, false, false, true);
}

#[test]
fn row26_unix_platform_version_codename() {
    unix_row("row26", 0x3026, true, true, true);
}

#[test]
fn row27_unix_platform_version_no_codename() {
    unix_row("row27", 0x3027, true, false, true);
}

#[test]
fn row28_unix_no_colon_no_pipe() {
    unix_row("row28", 0x3028, false, false, false);
}

#[test]
fn row29_unix_no_colon_with_pipe() {
    unix_row("row29", 0x3029, true, false, false);
}

/// Rows 30-32 are the version-shape axis re-crossed with the Unix branch, with
/// explicit emphasis on the shapes where a regex does *not* fire.
#[test]
fn row30to32_unix_version_shapes() {
    let mut rng = Rng::new(0x3030);
    for shape in ALL_SHAPES {
        for plat in [false, true] {
            for code in [false, true] {
                for _ in 0..200 {
                    let v = version(&mut rng, shape);
                    let p: Option<&[u8]> = if plat { Some(b"plat") } else { None };
                    let c: Option<&[u8]> = if code { Some(b"Code Name") } else { None };
                    let u = build_unix(b"host", b"OS", p, Some(&v), c, true);
                    both_poisons(&format!("row30-32 {shape:?} plat={plat} code={code}"), &u);
                }
            }
        }
    }
}

/// Row 33: arch in the prefix, crossed with all Unix sub-branches.
#[test]
fn row33_unix_arch_in_prefix() {
    let mut rng = Rng::new(0x3033);
    for arch in ARCHS.iter() {
        for plat in [false, true] {
            for code in [false, true] {
                for ver in [false, true] {
                    for _ in 0..12 {
                        let mut prefix = Vec::new();
                        prefix.extend_from_slice(b"Linux host 5.15 ");
                        prefix.extend_from_slice(arch.as_bytes());
                        let sh = *rng_shape(&mut rng);
                        let v = version(&mut rng, sh);
                        let p: Option<&[u8]> = if plat { Some(b"plat") } else { None };
                        let c: Option<&[u8]> = if code { Some(b"Code") } else { None };
                        let u = build_unix(
                            &prefix,
                            b"Distro",
                            p,
                            if ver { Some(&v) } else { None },
                            if ver { c } else { None },
                            true,
                        );
                        both_poisons(&format!("row33 {arch}"), &u);
                    }
                }
            }
        }
    }
}

fn rng_shape(rng: &mut Rng) -> &'static Shape {
    let i = rng.below(ALL_SHAPES.len());
    &ALL_SHAPES[i]
}

/// Row 34: the arch appears only *after* the `" ["`, so the truncated prefix no
/// longer contains it and `get_os_arch` must miss it.
#[test]
fn row34_arch_only_after_bracket() {
    let mut rng = Rng::new(0x3034);
    for arch in ARCHS.iter() {
        for _ in 0..60 {
            let v = version(&mut rng, Shape::MajorMinor);
            let mut os = Vec::new();
            os.extend_from_slice(b"Distro ");
            os.extend_from_slice(arch.as_bytes());
            let u = build_unix(b"host", &os, None, Some(&v), None, true);
            both_poisons(&format!("row34 {arch}"), &u);

            // arch inside the version / codename as well
            let mut v2 = v.clone();
            v2.extend_from_slice(b" ");
            v2.extend_from_slice(arch.as_bytes());
            let u2 = build_unix(b"host", b"Distro", None, Some(&v2), Some(arch.as_bytes()), true);
            both_poisons(&format!("row34b {arch}"), &u2);
        }
    }
}

/// Row 35: repeated markers — first `" ["`, first `": "`, first `" ("`, first
/// `"|"` each win.
#[test]
fn row35_repeated_markers() {
    let cases: &[&[u8]] = &[
        b"a [b: 1.2] [c: 3.4]",
        b"a [b: 1.2 (x)] [c: 3.4 (y)]",
        b"a [b: 1.2: 3.4]",
        b"a [b: 1.2 (x) (y)]",
        b"a [b|c|d: 1.2]",
        b"a [b: 1.2|3]",
        b"a|z [b: 1.2]",
        b"a [b: 1.2 (x|y)]",
        b"a [ [ [b: 1.2]",
        b"a [b: : 1.2]",
        b"a [: 1.2]",
        b"a [|: 1.2]",
        b"a [b:  1.2]",
        b"a [b:  (x)]",
        b" [ [Ver: 1.2]",
    ];
    for c in cases {
        both_poisons("row35", c);
    }
    // randomized repetition
    let mut rng = Rng::new(0x3035);
    for _ in 0..3000 {
        let mut u = name_word(&mut rng);
        let reps = rng.range(1, 3);
        for _ in 0..reps {
            u.extend_from_slice(b" [");
            let os = os_word(&mut rng);
            u.extend_from_slice(&os);
            if rng.chance(2) {
                u.push(b'|');
                u.extend_from_slice(b"plat");
            }
            if rng.chance(2) {
                u.extend_from_slice(b": ");
                let sh = *rng_shape(&mut rng);
                let v = version(&mut rng, sh);
                u.extend_from_slice(&v);
                if rng.chance(2) {
                    u.extend_from_slice(b" (");
                    let c = codename_word(&mut rng);
                    u.extend_from_slice(&c);
                    u.push(b')');
                }
            }
            u.push(b']');
        }
        both_poisons("row35 rand", &u);
    }
}

/// Row 36: `": "` inside the codename part, and other marker interleavings.
#[test]
fn row36_colon_inside_codename() {
    let cases: &[&[u8]] = &[
        b"h [OS 1.2 (a: b)]",
        b"h [OS (a: b)]",
        b"h [OS: 1.2 (a: b)]",
        b"h [OS (a) : 1.2]",
        b"h [OS (1.2): 3.4]",
        b"h [OS| (1.2): 3.4]",
        b"h [OS (: )]",
        b"h [OS ( : )]",
    ];
    for c in cases {
        both_poisons("row36", c);
    }
}

// ---------------------------------------------------------------------------
// Rows 37-42
// ---------------------------------------------------------------------------

/// Row 37: neither marker — arch present (all 12) and absent.
#[test]
fn row37_no_markers() {
    let mut rng = Rng::new(0x3037);
    both_poisons("row37 empty", b"");
    for arch in ARCHS.iter() {
        for _ in 0..80 {
            let mut u = b"Linux host 5.15.0-generic ".to_vec();
            u.extend_from_slice(arch.as_bytes());
            u.extend_from_slice(b" GNU/Linux");
            both_poisons(&format!("row37 {arch}"), &u);
        }
    }
    for _ in 0..2000 {
        // no " [" and no " [Ver: "
        let mut u = name_word(&mut rng);
        u.extend_from_slice(b" no-brackets-here ");
        let os = os_word(&mut rng);
        u.extend_from_slice(&os);
        both_poisons("row37 none", &u);
    }
    // '[' present but not preceded by a space
    for c in [
        b"Linux[Ubuntu: 22.04]".as_slice(),
        b"[Ubuntu: 22.04]",
        b"[",
        b" ",
        b"  ",
        b"[ ",
        b"]",
        b": ",
        b" (",
        b"|",
        b"Ver: 1.2",
        b"[Ver: 1.2]",
    ] {
        both_poisons("row37 nearmiss", c);
    }
}

/// Row 38: fully randomized `uname` over an alphabet that makes *every* marker
/// appear by chance — 20 000 cases, both poison values.
#[test]
fn row38_marker_rich_fuzz() {
    const ALPHA: &[u8] = b" [](:|)Ver.0123456789xai86_64AIXsparcmd";
    let mut rng = Rng::new(0x3038);
    for _ in 0..20000 {
        let len = rng.range(0, 40);
        let s: Vec<u8> = (0..len).map(|_| *rng.pick(ALPHA)).collect();
        both_poisons("row38", &s);
    }
}

/// Row 38b: token-level fuzz — random concatenation of the exact markers the C
/// searches for, so all combinations and orders occur.
#[test]
fn row38b_token_fuzz() {
    const TOKENS: [&[u8]; 18] = [
        b" [Ver: ", b" [", b": ", b" (", b")", b"]", b"|", b" ", b"1", b"2.3", b"4.5.6",
        b"7.8.9.10", b"abc", b"x86_64", b"arm64", b"", b"[", b"(",
    ];
    let mut rng = Rng::new(0x3038b);
    for _ in 0..20000 {
        let n = rng.range(0, 8);
        let mut s = Vec::new();
        for _ in 0..n {
            let t = *rng.pick(&TOKENS);
            s.extend_from_slice(t);
        }
        both_poisons("row38b", &s);
    }
}

/// Row 39 is folded into every other row via [`both_poisons`] (poison `0xAA`);
/// this test isolates it and additionally sweeps several poison byte values.
#[test]
fn row39_poisoned_os_data() {
    let mut rng = Rng::new(0x3039);
    let inputs: &[&[u8]] = &[
        b"Win [Ver: 10.0.19041.1234]",
        b"x86_64 [Ubuntu|ubuntu: 22.04.3 LTS (Jammy)]",
        b"host [OS]",
        b"host [OS|plat]",
        b"amd64 only",
        b"",
        b" [Ver: ",
        b" [",
    ];
    for poison in [0x00u8, 0x01, 0x11, 0x7f, 0xAA, 0xFF] {
        for i in inputs {
            diff_parse_uname(&format!("row39 poison=0x{poison:02x}"), i, poison);
        }
        for _ in 0..500 {
            let sh = *rng_shape(&mut rng);
            let v = version(&mut rng, sh);
            let u = build_unix(b"aarch64 host", b"Distro", Some(b"plat"), Some(&v), Some(b"code"), true);
            diff_parse_uname("row39 rand", &u, poison);
        }
    }
}

/// Row 40: 4 KiB `uname` with markers at random offsets.
#[test]
fn row40_large_uname() {
    let mut rng = Rng::new(0x3040);
    for _ in 0..400 {
        let total = rng.range(1024, 4096);
        let mut s: Vec<u8> = (0..total)
            .map(|_| *rng.pick(b"abcdefgh 0123456789".as_slice()))
            .collect();
        // plant markers at random offsets
        let markers: &[&[u8]] = &[b" [Ver: ", b" [", b": ", b" (", b")", b"]", b"|"];
        let plants = rng.range(0, 6);
        for _ in 0..plants {
            let m = *rng.pick(markers);
            if m.len() >= total {
                continue;
            }
            let at = rng.below(total - m.len());
            s[at..at + m.len()].copy_from_slice(m);
        }
        if rng.chance(2) {
            let arch = *rng.pick(&ARCHS);
            let a = arch.as_bytes();
            let at = rng.below(total - a.len());
            s[at..at + a.len()].copy_from_slice(a);
        }
        both_poisons("row40", &s);
    }
}

/// Row 41: real-world-shaped uname strings.
#[test]
fn row41_real_world_corpus() {
    let corpus: &[&[u8]] = &[
        b"Microsoft Windows 10 Pro [Ver: 10.0.19041.1237]",
        b"Microsoft Windows Server 2019 Datacenter [Ver: 10.0.17763.2183]",
        b"Microsoft Windows 7 Professional [Ver: 6.1.7601]",
        b"Microsoft Windows XP [Ver: 5.1.2600]",
        b"Microsoft Windows 11 [Ver: 10.0.22621.1928.2]",
        b"Linux ubuntu 5.15.0-56-generic #62-Ubuntu SMP x86_64 [Ubuntu|ubuntu: 22.04.1 LTS (Jammy Jellyfish)]",
        b"Linux centos 3.10.0-1160.el7.x86_64 #1 SMP x86_64 [CentOS Linux|centos: 7.9.2009 (Core)]",
        b"Linux debian 5.10.0-19-amd64 #1 SMP Debian amd64 [Debian GNU/Linux|debian: 11 (bullseye)]",
        b"Linux arch 6.0.9-arch1-1 #1 SMP PREEMPT_DYNAMIC x86_64 [Arch Linux|arch: rolling]",
        b"Linux alpine 5.15.79-0-lts #1-Alpine SMP x86_64 [Alpine Linux|alpine: 3.17.0]",
        b"Linux amzn 4.14.294-220.533.amzn2.x86_64 #1 SMP x86_64 [Amazon Linux|amzn: 2]",
        b"Linux rpi 5.15.61-v7+ #1579 SMP armv7l [Raspbian GNU/Linux|raspbian: 10 (buster)]",
        b"Linux pine 5.19.0 #1 SMP aarch64 [Manjaro ARM|manjaro-arm: 22.06]",
        b"Darwin macbook 21.6.0 Darwin Kernel Version 21.6.0 x86_64 [Mac OS X|darwin: 12.6 (Monterey)]",
        b"Darwin mini 22.1.0 arm64 [macOS|darwin: 13.0.1 (Ventura)]",
        b"AIX host 1 7 00F9C1D14C00 [AIX|aix: 7.1]",
        b"SunOS solaris 5.11 11.4.42.111.0 i86pc i386 [Oracle Solaris|sunos: 11.4]",
        b"SunOS sol 5.10 Generic_150401-49 sun4v sparc [Solaris|sunos: 10]",
        b"FreeBSD bsd 13.1-RELEASE FreeBSD amd64 [FreeBSD|freebsd: 13.1]",
        b"OpenBSD obsd 7.2 GENERIC.MP#3 amd64 [OpenBSD|openbsd: 7.2]",
        b"HP-UX hp B.11.31 U ia64 [HP-UX|hpux: 11.31]",
        // partially formed / legacy shapes
        b"Linux host 2.6.32 [CentOS release 6.10]",
        b"Linux host 2.6.32 x86_64 [CentOS]",
        b"Linux host [Ubuntu: 22.04]",
        b"Linux host [Ubuntu|ubuntu]",
        b"Windows [Ver: 6.3.9600]",
        b"Linux x86_64",
        b"x86_64",
    ];
    for c in corpus {
        both_poisons("row41", c);
    }
}

/// Row 42: composed pipeline across all three entry points.
#[test]
fn row42_composed_pipeline() {
    let mut rng = Rng::new(0x3042);
    let seeds: &[&[u8]] = &[
        b"Microsoft Windows 10 [Ver: 10.0.19041.1237]",
        b"Linux x86_64 [Ubuntu|ubuntu: 22.04.1 LTS (Jammy)]",
        b"Darwin arm64 [macOS|darwin: 13.0.1 (Ventura)]",
        b"host [OS]",
        b"amd64",
        b"",
    ];
    for s in seeds {
        diff_pipeline("row42 seed", s, 0x00);
        diff_pipeline("row42 seed", s, 0xAA);
    }
    for _ in 0..4000 {
        let sh = *rng_shape(&mut rng);
        let v = version(&mut rng, sh);
        let mut prefix = name_word(&mut rng);
        prefix.push(b' ');
        let arch = *rng.pick(&ARCHS);
        prefix.extend_from_slice(arch.as_bytes());
        let u = if rng.chance(3) {
            build_windows(&prefix, &v, rng.chance(4) == false)
        } else {
            let os = os_word(&mut rng);
            let plat: Option<&[u8]> = if rng.chance(2) { Some(b"plat") } else { None };
            let code: Option<&[u8]> = if rng.chance(2) { Some(b"Code Name") } else { None };
            let ver = if rng.chance(4) { None } else { Some(v.as_slice()) };
            build_unix(&prefix, &os, plat, ver, code, true)
        };
        diff_pipeline("row42 rand", &u, 0x00);
        diff_pipeline("row42 rand", &u, 0xAA);
    }
}
