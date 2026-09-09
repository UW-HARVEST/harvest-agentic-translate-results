mod common;

use common::{Libraries, manifest_path};
use std::collections::BTreeSet;
use std::fs;
use std::process::Command;

fn dynamic_symbols(path: &std::path::Path) -> BTreeSet<String> {
    let output = Command::new("nm")
        .args(["-D", "--defined-only"])
        .arg(path)
        .output()
        .expect("run nm");
    assert!(output.status.success());
    String::from_utf8(output.stdout)
        .unwrap()
        .lines()
        .filter_map(|line| line.split_whitespace().last())
        .map(str::to_owned)
        .collect()
}

#[test]
fn phase_artifacts_are_complete_and_mechanically_consistent() {
    unsafe {
        let crate_dir = manifest_path(".");
        let c_so = crate_dir.join("../c_src/build/liblz4.so");
        let rust_so = crate_dir.join("target/release/liblz4.so");
        let c_symbols = dynamic_symbols(&c_so);
        let rust_symbols = dynamic_symbols(&rust_so);
        assert_eq!(c_symbols, rust_symbols);
        assert_eq!(c_symbols.len(), 143);

        let backend_symbols: BTreeSet<_> =
            fs::read_to_string(crate_dir.join("backend-symbols.txt"))
                .unwrap()
                .lines()
                .filter(|line| !line.is_empty())
                .map(str::to_owned)
                .collect();
        assert_eq!(c_symbols, backend_symbols);

        let build_rs = fs::read_to_string(crate_dir.join("build.rs")).unwrap();
        for source in ["lz4.c", "lz4hc.c", "lz4frame.c", "lz4file.c", "xxhash.c"] {
            assert!(build_rs.contains(source), "build.rs omits {source}");
        }

        let libraries = Libraries::load();
        let configs = fs::read_to_string(crate_dir.join("CONFIGS.md")).unwrap();
        let mut config_rows = 0;
        for line in configs
            .lines()
            .filter(|line| line.starts_with("| ") && line.contains('`'))
        {
            let start = line.find('`').unwrap();
            let tail = &line[start + 1..];
            let end = tail.find('`').unwrap();
            let symbol = &tail[..end];
            let mut name = symbol.as_bytes().to_vec();
            name.push(0);
            let _: libloading::Symbol<'_, unsafe extern "C" fn()> = libraries.c.get(&name).unwrap();
            let _: libloading::Symbol<'_, unsafe extern "C" fn()> =
                libraries.rust.get(&name).unwrap();
            config_rows += 1;
        }
        assert_eq!(config_rows, 143);

        let symbols = fs::read_to_string(crate_dir.join("SYMBOLS.md")).unwrap();
        assert_eq!(
            symbols
                .lines()
                .filter(|line| line.starts_with("| ") && line.contains('`'))
                .count(),
            143
        );
        assert!(symbols.contains("- Missing from Rust: 0"));
        assert!(symbols.contains("- Rust-only exports: 0"));

        let errors = fs::read_to_string(crate_dir.join("ERRORS.md")).unwrap();
        let error_rows = errors
            .lines()
            .filter(|line| line.starts_with("| ") && line.contains(".c:"))
            .count();
        assert!(
            error_rows >= 290,
            "error inventory unexpectedly shrank: {error_rows}"
        );
        assert!(
            !configs.lines().any(|line| line
                .as_bytes()
                .get(2)
                .is_some_and(|byte| byte.is_ascii_digit())
                && line.ends_with("| [ ] |")),
            "CONFIGS.md contains unchecked rows"
        );
        assert!(
            !errors.lines().any(|line| line
                .as_bytes()
                .get(2)
                .is_some_and(|byte| byte.is_ascii_digit())
                && line.ends_with("| [ ] |")),
            "ERRORS.md contains unchecked rows"
        );
    }
}
