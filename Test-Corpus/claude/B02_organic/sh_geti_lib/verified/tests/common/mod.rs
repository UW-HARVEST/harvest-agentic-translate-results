//! Shared helpers for libloading-based integration tests.

use std::path::PathBuf;
use std::sync::Once;

pub fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

pub fn c_lib_path() -> PathBuf {
    project_root().join("c_src/build/libtranslated_rust.so")
}

pub fn rust_lib_path() -> PathBuf {
    let target_dir = std::env::var("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|_| project_root().join("target"));
    target_dir.join("release/libsh_geti_lib.so")
}

static BUILD_ONCE: Once = Once::new();

pub fn ensure_libs_built() {
    BUILD_ONCE.call_once(|| {
        let c_path = c_lib_path();
        if !c_path.exists() {
            let build_dir = project_root().join("c_src/build");
            std::fs::create_dir_all(&build_dir).expect("create build dir");
            let status = std::process::Command::new("cmake")
                .arg("..")
                .arg("-DCMAKE_POSITION_INDEPENDENT_CODE=ON")
                .current_dir(&build_dir)
                .status()
                .expect("run cmake");
            assert!(status.success(), "cmake configure failed");
            let status = std::process::Command::new("cmake")
                .arg("--build")
                .arg(".")
                .current_dir(&build_dir)
                .status()
                .expect("run cmake --build");
            assert!(status.success(), "cmake build failed");
        }

        let rust_path = rust_lib_path();
        if !rust_path.exists() {
            let status = std::process::Command::new(env!("CARGO"))
                .arg("build")
                .arg("--release")
                .current_dir(project_root())
                .status()
                .expect("run cargo build");
            assert!(status.success(), "cargo build --release failed");
        }
    });
}
