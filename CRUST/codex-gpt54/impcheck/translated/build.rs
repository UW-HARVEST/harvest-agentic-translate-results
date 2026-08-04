use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn main() {
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());
    let build_dir = manifest_dir.join("build");
    let temp_dir = manifest_dir.join("target").join("impcheck-build");

    fs::create_dir_all(&build_dir).unwrap();
    fs::create_dir_all(&temp_dir).unwrap();

    for path in [
        "src/checker_interface.rs",
        "src/confirm.rs",
        "src/lrat_check.rs",
        "src/main_check.rs",
        "src/main_confirm.rs",
        "src/main_parse.rs",
        "src/secret.rs",
        "src/siphash.rs",
        "src/top_check.rs",
        "src/trusted_checker.rs",
        "src/trusted_parser.rs",
        "src/trusted_utils.rs",
    ] {
        println!("cargo:rerun-if-changed={path}");
    }

    compile_runner(
        &manifest_dir,
        &temp_dir,
        &build_dir,
        "impcheck_parse",
        &[
            "trusted_utils",
            "secret",
            "siphash",
            "trusted_parser",
            "main_parse",
        ],
        "main_parse::main(std::env::args().len() as i32, std::env::args().collect())",
    );
    compile_runner(
        &manifest_dir,
        &temp_dir,
        &build_dir,
        "impcheck_confirm",
        &[
            "trusted_utils",
            "secret",
            "siphash",
            "confirm",
            "trusted_parser",
            "main_confirm",
        ],
        "main_confirm::main(std::env::args().len() as i32, std::env::args().collect())",
    );
    compile_runner(
        &manifest_dir,
        &temp_dir,
        &build_dir,
        "impcheck_check",
        &[
            "trusted_utils",
            "checker_interface",
            "secret",
            "siphash",
            "confirm",
            "lrat_check",
            "top_check",
            "trusted_checker",
            "main_check",
        ],
        "main_check::main(std::env::args().len() as i32, std::env::args().collect())",
    );
}

fn compile_runner(
    manifest_dir: &Path,
    temp_dir: &Path,
    build_dir: &Path,
    name: &str,
    modules: &[&str],
    entry_expr: &str,
) {
    let runner_path = temp_dir.join(format!("{name}.rs"));
    let mut source = String::new();
    for module in modules {
        let path = manifest_dir.join("src").join(format!("{module}.rs"));
        source.push_str(&format!(
            "#[path = \"{}\"] mod {};\n",
            path.display(),
            module
        ));
    }
    source.push_str("fn main() {\n");
    source.push_str(&format!("    std::process::exit({entry_expr});\n"));
    source.push_str("}\n");
    fs::write(&runner_path, source).unwrap();

    let output_path = build_dir.join(name);
    let status = Command::new("rustc")
        .arg("--edition=2021")
        .arg(&runner_path)
        .arg("-o")
        .arg(&output_path)
        .status()
        .unwrap();
    assert!(status.success(), "failed to build {name}");
}
