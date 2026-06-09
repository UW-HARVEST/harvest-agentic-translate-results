use std::io::{self, BufRead, Write};

use uname_parser::{parse_uname_string, OsData};

fn print_field(out: &mut impl Write, name: &str, value: &Option<String>) -> io::Result<()> {
    match value {
        Some(s) => writeln!(out, "{}: {}", name, s),
        None => writeln!(out, "{}: (null)", name),
    }
}

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        let mut osd = OsData::new();
        parse_uname_string(&line, &mut osd);

        let _ = print_field(&mut out, "os_name", &osd.os_name);
        let _ = print_field(&mut out, "os_version", &osd.os_version);
        let _ = print_field(&mut out, "os_major", &osd.os_major);
        let _ = print_field(&mut out, "os_minor", &osd.os_minor);
        let _ = print_field(&mut out, "os_codename", &osd.os_codename);
        let _ = print_field(&mut out, "os_platform", &osd.os_platform);
        let _ = print_field(&mut out, "os_build", &osd.os_build);
        let _ = print_field(&mut out, "os_uname", &osd.os_uname);
        let _ = print_field(&mut out, "os_arch", &osd.os_arch);
    }
}
