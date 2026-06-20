use crate::{csvline::CsvLine};
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::sync::{Mutex, OnceLock};

pub const BUFSIZE: usize = 4096;
pub const MAXCOL: usize = 1024;
const RV_STATE_MULTILINE: i32 = 0x01;
const RV_STATE_EOL: i32 = 0x02;
const RV_DELIM: i32 = 0x04;

#[derive(Clone)]
struct Config {
    delim_in: char,
    delim_out: char,
    quote_in: char,
    quote_out: char,
    end_line: Option<String>,
    allow_binary: bool,
    verbose: i32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            delim_in: ',',
            delim_out: ',',
            quote_in: '"',
            quote_out: '"',
            end_line: None,
            allow_binary: false,
            verbose: 0,
        }
    }
}

fn config() -> &'static Mutex<Config> {
    static CONFIG: OnceLock<Mutex<Config>> = OnceLock::new();
    CONFIG.get_or_init(|| Mutex::new(Config::default()))
}

pub fn output_line(cline: &CsvLine) {
let cfg = config().lock().expect("config mutex poisoned").clone();
let mut out = io::stdout().lock();
for i in 0..cline.get_field_count() {
    let mut value = cline.get_field(i).unwrap_or("");
    let mut slot = [value];
    format_output(&mut slot);
    value = slot[0];
    let _ = write!(out, "{}", value);
    if i + 1 != cline.get_field_count() {
        let _ = write!(out, "{}", cfg.delim_out);
    }
}
let eol = cfg.end_line.as_deref().unwrap_or(cline.eol_str.as_str());
let _ = write!(out, "{}", eol);
}
pub fn usage(fp: &mut std::fs::File) {
let help = "cissy [options]\n\t-i <inputfile>\t\t (defaults to stdin)\n\t-o <outputfile>\t\t (defaults to stdout)\n\n\t-c <columns>\t\t specify columns to output eg. [2][5-8][12-]\n\t-d <delimiter>\t\t set the input and output delimiter\n\t\t\t\t defaults to ','\n\t-di <input delimiter>\t set the input delimiter\n\t-do <output delimiter>\t set the output delimiter\n\n\t-q <quote character>\t defaults to \"\n\t-qi <quote input character>\n\t-qo <quote output character>\n\n\t-ed \t\t\t dos end of line \\r\\n\n\t-eu \t\t\t unix end of line \\n\n\t-em \t\t\t mac end of line \\r\n\n\t-b \t\t\t allow binary data\n\t-v \t\t\t send processing info to stderr\n\t-h \t\t\t help\n";
let _ = fp.write_all(help.as_bytes());
}
pub fn debug(level: i32, fmt: &str) {
if level <= config().lock().expect("config mutex poisoned").verbose {
    let _ = io::stderr().write_all(fmt.as_bytes());
}
}
pub fn is_bin_char(c: char) -> bool {
matches!(c as u32, 1..=8 | 11 | 12 | 14..=26 | 28..=31 | 127)
}
pub fn format_output(str: &mut [&str]) {
if str.is_empty() {
    return;
}
let cfg = config().lock().expect("config mutex poisoned").clone();
let value = str[0];
if cfg.quote_in == cfg.quote_out || value.len() < 2 {
    return;
}
let mut chars = value.chars();
let first = chars.next();
let last = value.chars().last();
if first == Some(cfg.quote_in) && last == Some(cfg.quote_in) {
    let mut rendered = String::with_capacity(value.len());
    rendered.push(cfg.quote_out);
    rendered.push_str(&value[cfg.quote_in.len_utf8()..value.len() - cfg.quote_in.len_utf8()]);
    rendered.push(cfg.quote_out);
    let leaked: &'static str = Box::leak(rendered.into_boxed_str());
    str[0] = leaked;
}
}
pub fn get_field(buf: &str, buflen: usize, end: &mut usize, in_quoted: &mut bool) -> i32 {
let cfg = config().lock().expect("config mutex poisoned").clone();
*end = 0;
for (idx, ch) in buf[..buflen.min(buf.len())].char_indices() {
    *end = idx;
    if !cfg.allow_binary && is_bin_char(ch) {
        assert!(cfg.allow_binary, "error: binary character found");
    }
    if *in_quoted {
        if ch == cfg.quote_in {
            *in_quoted = false;
        }
    } else {
        if ch == cfg.quote_in {
            *in_quoted = true;
        }
        if ch == cfg.delim_in {
            return RV_DELIM;
        }
        if ch == '\r' || ch == '\n' {
            return RV_STATE_EOL;
        }
        if ch == '\0' && idx + ch.len_utf8() >= buflen {
            return RV_STATE_EOL;
        }
    }
}
*end = buflen.min(buf.len());
if *in_quoted { RV_STATE_MULTILINE } else { RV_STATE_EOL }
}
pub fn main(argc: i32, argv: &[&str]) -> i32 {
let _ = argc;
*config().lock().expect("config mutex poisoned") = Config::default();

let mut input_path: Option<String> = None;
let mut output_path: Option<String> = None;
let mut arginc = 1usize;
while arginc < argv.len() {
    match argv[arginc] {
        "-i" => {
            arginc += 1;
            if arginc >= argv.len() {
                return -1;
            }
            input_path = Some(argv[arginc].to_string());
        }
        "-o" => {
            arginc += 1;
            if arginc >= argv.len() {
                return -1;
            }
            output_path = Some(argv[arginc].to_string());
        }
        "-d" => {
            arginc += 1;
            if arginc >= argv.len() || argv[arginc].chars().count() != 1 {
                return -1;
            }
            let ch = argv[arginc].chars().next().unwrap_or(',');
            let mut cfg = config().lock().expect("config mutex poisoned");
            cfg.delim_in = ch;
            cfg.delim_out = ch;
        }
        "-di" => {
            arginc += 1;
            if arginc >= argv.len() || argv[arginc].chars().count() != 1 {
                return -1;
            }
            config().lock().expect("config mutex poisoned").delim_in = argv[arginc].chars().next().unwrap();
        }
        "-do" => {
            arginc += 1;
            if arginc >= argv.len() || argv[arginc].chars().count() != 1 {
                return -1;
            }
            config().lock().expect("config mutex poisoned").delim_out = argv[arginc].chars().next().unwrap();
        }
        "-q" | "-qi" => {
            arginc += 1;
            if arginc >= argv.len() || argv[arginc].chars().count() != 1 {
                return -1;
            }
            config().lock().expect("config mutex poisoned").quote_in = argv[arginc].chars().next().unwrap();
        }
        "-qo" => {
            arginc += 1;
            if arginc >= argv.len() || argv[arginc].chars().count() != 1 {
                return -1;
            }
            config().lock().expect("config mutex poisoned").quote_out = argv[arginc].chars().next().unwrap();
        }
        "-eu" => config().lock().expect("config mutex poisoned").end_line = Some("\n".to_string()),
        "-ed" => config().lock().expect("config mutex poisoned").end_line = Some("\r\n".to_string()),
        "-em" => config().lock().expect("config mutex poisoned").end_line = Some("\r".to_string()),
        "-v" => config().lock().expect("config mutex poisoned").verbose += 1,
        "-DDD" => config().lock().expect("config mutex poisoned").verbose += 100,
        "-b" => config().lock().expect("config mutex poisoned").allow_binary = true,
        "-h" => {
            let mut stdout_file = output_path
                .as_deref()
                .and_then(|path| File::create(path).ok())
                .unwrap_or_else(|| File::create("/dev/stdout").expect("open stdout"));
            usage(&mut stdout_file);
            return 0;
        }
        "-c" => {
            arginc += 1;
            if arginc >= argv.len() {
                return -1;
            }
        }
        _ => return -1,
    }
    arginc += 1;
}

let reader: Box<dyn BufRead> = match input_path {
    Some(path) => match File::open(path) {
        Ok(file) => Box::new(BufReader::new(file)),
        Err(_) => return -1,
    },
    None => Box::new(BufReader::new(io::stdin())),
};

let mut writer: Box<dyn Write> = match output_path {
    Some(path) => match File::create(path) {
        Ok(file) => Box::new(file),
        Err(_) => return -1,
    },
    None => Box::new(io::stdout()),
};

let cfg = config().lock().expect("config mutex poisoned").clone();
for line in reader.lines() {
    let line = match line {
        Ok(line) => line,
        Err(_) => return -1,
    };
    let mut cline = CsvLine::new();
    let mut start = 0usize;
    let bytes = line.as_bytes();
    for (idx, b) in bytes.iter().enumerate() {
        if char::from(*b) == cfg.delim_in {
            cline.add_field(&line, start, idx - start);
            start = idx + 1;
        }
    }
    cline.add_field(&line, start, bytes.len().saturating_sub(start));
    for i in 0..cline.get_field_count() {
        if let Some(field) = cline.get_field(i) {
            let _ = write!(writer, "{}", field);
        }
        if i + 1 != cline.get_field_count() {
            let _ = write!(writer, "{}", cfg.delim_out);
        }
    }
    let _ = write!(writer, "{}", cfg.end_line.as_deref().unwrap_or("\n"));
}
0
}
