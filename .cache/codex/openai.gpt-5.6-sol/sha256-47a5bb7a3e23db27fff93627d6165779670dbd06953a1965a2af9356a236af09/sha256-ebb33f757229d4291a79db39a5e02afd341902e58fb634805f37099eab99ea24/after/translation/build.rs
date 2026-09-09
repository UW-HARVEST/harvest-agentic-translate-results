use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

fn run(command: &mut Command) {
    let rendered = format!("{command:?}");
    let status = command.status().unwrap_or_else(|error| {
        panic!("failed to execute {rendered}: {error}");
    });
    assert!(status.success(), "command failed: {rendered}");
}

fn main() {
    let manifest_dir = PathBuf::from(env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let source_dir = manifest_dir.join("../c_src");
    let out_dir = PathBuf::from(env::var_os("OUT_DIR").unwrap());
    let symbol_file = manifest_dir.join("backend-symbols.txt");
    let rename_file = out_dir.join("rename-symbols.txt");

    let symbols = fs::read_to_string(&symbol_file).expect("read backend-symbols.txt");
    let rename_map = symbols
        .lines()
        .filter(|line| !line.is_empty())
        .map(|symbol| format!("{symbol} lz4rs_backend_{symbol}\n"))
        .collect::<String>();
    fs::write(&rename_file, rename_map).expect("write symbol rename map");

    let sources = ["lz4.c", "lz4hc.c", "lz4frame.c", "lz4file.c", "xxhash.c"];
    let mut objects = Vec::with_capacity(sources.len());
    for source in sources {
        let input = source_dir.join("src").join(source);
        let object = out_dir.join(format!("{source}.o"));
        run(Command::new("cc")
            .arg("-std=c99")
            .arg("-O2")
            .arg("-fPIC")
            .arg("-DXXH_NAMESPACE=LZ4_")
            .arg("-DLZ4_HEAPMODE=0")
            .arg("-DLZ4F_HEAPMODE=0")
            .arg("-I")
            .arg(source_dir.join("include"))
            .arg("-I")
            .arg(source_dir.join("src"))
            .arg("-c")
            .arg(&input)
            .arg("-o")
            .arg(&object));
        run(Command::new("objcopy")
            .arg("--redefine-syms")
            .arg(&rename_file)
            .arg(&object));
        objects.push(object);
        println!("cargo:rerun-if-changed={}", input.display());
    }

    let archive = out_dir.join("liblz4_backend.a");
    let mut ar = Command::new("ar");
    ar.arg("crs").arg(&archive);
    for object in &objects {
        ar.arg(object);
    }
    run(&mut ar);

    println!("cargo:rerun-if-changed={}", symbol_file.display());
    println!(
        "cargo:rerun-if-changed={}",
        source_dir.join("include").display()
    );
    println!("cargo:rustc-link-search=native={}", out_dir.display());
    println!("cargo:rustc-link-lib=static=lz4_backend");
}
