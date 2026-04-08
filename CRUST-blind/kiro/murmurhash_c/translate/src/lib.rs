pub mod murmurhash;

pub fn usage() {
    eprint!("usage: murmur [-hV] [options]\n");
}

pub fn help() {
    eprint!("\noptions:\n");
    eprint!("\n  --seed=[seed]  hash seed (optional)");
    eprint!("\n");
}

pub fn read_stdin() -> Vec<u8> {
    let mut buf = String::new();
    match std::io::stdin().read_line(&mut buf) {
        Ok(0) => Vec::new(),
        Ok(_) => buf.into_bytes(),
        Err(_) => Vec::new(),
    }
}

pub fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut seed = String::from("0");

    for arg in &args {
        if arg == "-h" {
            usage();
            help();
            return;
        } else if arg == "-V" {
            eprintln!("{}", murmurhash::MURMURHASH_VERSION);
            return;
        } else if arg.starts_with("--seed=") {
            seed = arg["--seed=".len()..].to_string();
        } else if arg.starts_with("-") {
            let flag = &arg[1..];
            eprintln!("unknown option: `{}'", flag);
            usage();
            return;
        }
    }

    let seed_val: u32 = seed.parse().unwrap_or(0);
    let buf = read_stdin();
    if buf.is_empty() {
        return;
    }
    let h = murmurhash::murmurhash(&buf, seed_val);
    println!("{}", h);

    loop {
        let key = read_stdin();
        if key.is_empty() {
            break;
        }
        let h = murmurhash::murmurhash(&buf, seed_val);
        println!("{}", h);
    }
}
