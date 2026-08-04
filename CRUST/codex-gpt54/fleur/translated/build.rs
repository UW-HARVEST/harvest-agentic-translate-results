use std::{env, fs, path::Path};

fn main() {
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let manifest_path = Path::new(&manifest_dir);

    for fixture in [
        "hang0.bin",
        "hang1.bin",
        "hang2.bin",
        "join1.bloom",
        "join2.bloom",
        "join3.bloom",
        "datatest.bloom",
        "header.bin",
    ] {
        let src = manifest_path.join("src/bin").join(fixture);
        let dst = manifest_path.join(fixture);
        println!("cargo:rerun-if-changed={}", src.display());
        let _ = fs::copy(src, dst);
    }
}
