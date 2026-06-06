use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

pub fn isexpression(c: char) -> bool {
    match c {
        '-' | '0'..='9' | 'z' | 'Z' | 'x' | 'X' | 'b' | 'U' => true,
        _ => false,
    }
}
pub const VCD_SIGNAL_COUNT: usize = 32;
pub const VCD_VALUE_CHANGE_COUNT: usize = 4096;
pub const VCD_SIGNAL_SIZE: usize = 64;
pub const VCD_NAME_SIZE: usize = 32;
pub const VCD_TIME_UNIT_SIZE: usize = 8;
pub const VCD_VERSION_SIZE: usize = 64;
pub const VCD_DATE_SIZE: usize = 64;
pub type Timestamp = u32;
#[derive(Debug, Clone, Copy)]
pub struct ValueChange {
    pub timestamp: Timestamp,
    pub value: [u8; VCD_SIGNAL_SIZE], // Fixed-size array instead of C char array
}
#[derive(Debug)]
pub struct Signal {
    pub name: [u8; VCD_NAME_SIZE],
    pub size: usize,
    pub value_changes: Vec<ValueChange>,
}
#[derive(Debug)]
pub struct Timescale {
    pub unit: [u8; VCD_TIME_UNIT_SIZE], // Fixed-size array replacing C char array
    pub scale: usize,
}
#[derive(Debug)]
pub struct VCD {
    pub signals: Vec<Signal>,
    pub date: [u8; VCD_DATE_SIZE],
    pub version: [u8; VCD_VERSION_SIZE],
    pub timescale: Timescale,
}

// ----- internal helpers -----

fn read_byte(mut file: &File) -> std::io::Result<Option<u8>> {
    let mut buf = [0u8; 1];
    match file.read(&mut buf) {
        Ok(0) => Ok(None),
        Ok(_) => Ok(Some(buf[0])),
        Err(e) => Err(e),
    }
}

fn unread_byte(mut file: &File) -> std::io::Result<()> {
    file.seek(SeekFrom::Current(-1))?;
    Ok(())
}

fn skip_whitespace(file: &File) -> std::io::Result<()> {
    while let Some(b) = read_byte(file)? {
        if !b.is_ascii_whitespace() {
            unread_byte(file)?;
            return Ok(());
        }
    }
    Ok(())
}

fn read_word(file: &File) -> std::io::Result<String> {
    skip_whitespace(file)?;
    let mut buf: Vec<u8> = Vec::new();
    while let Some(b) = read_byte(file)? {
        if b.is_ascii_whitespace() {
            unread_byte(file)?;
            break;
        }
        buf.push(b);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn read_until<F: Fn(u8) -> bool>(file: &File, terminator: F) -> std::io::Result<String> {
    let mut buf: Vec<u8> = Vec::new();
    while let Some(b) = read_byte(file)? {
        if terminator(b) {
            unread_byte(file)?;
            break;
        }
        buf.push(b);
    }
    Ok(String::from_utf8_lossy(&buf).into_owned())
}

fn skip_until<F: Fn(u8) -> bool>(file: &File, terminator: F) -> std::io::Result<()> {
    while let Some(b) = read_byte(file)? {
        if terminator(b) {
            unread_byte(file)?;
            return Ok(());
        }
    }
    Ok(())
}

fn read_digits(file: &File) -> std::io::Result<String> {
    let mut s = String::new();
    while let Some(b) = read_byte(file)? {
        if b.is_ascii_digit() {
            s.push(b as char);
        } else {
            unread_byte(file)?;
            break;
        }
    }
    Ok(s)
}

fn copy_str_to_array(s: &str, arr: &mut [u8]) {
    for b in arr.iter_mut() {
        *b = 0;
    }
    let bytes = s.as_bytes();
    let n = bytes.len().min(arr.len());
    arr[..n].copy_from_slice(&bytes[..n]);
}

fn array_to_str(arr: &[u8]) -> &str {
    let len = arr.iter().position(|&b| b == 0).unwrap_or(arr.len());
    std::str::from_utf8(&arr[..len]).unwrap_or("")
}

fn io_err(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg)
}

// ----- impl -----

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut vcd = VCD {
            signals: Vec::new(),
            date: [0; VCD_DATE_SIZE],
            version: [0; VCD_VERSION_SIZE],
            timescale: Timescale {
                unit: [0; VCD_TIME_UNIT_SIZE],
                scale: 0,
            },
        };
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        while let Some(c) = read_byte(&file)? {
            if c == b'$' {
                parse_instruction(&file, &mut vcd, &mut state)?;
                continue;
            } else if c == b'#' {
                current_timestamp = parse_timestamp(&file)?;
                continue;
            } else if isexpression(c as char) {
                unread_byte(&file)?;
                parse_assignment(&file, &mut vcd, &current_timestamp)?;
                continue;
            } else if c.is_ascii_whitespace() {
                continue;
            }

            return Err(io_err("unexpected character in VCD file"));
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for signal in &self.signals {
            if array_to_str(&signal.name) == signal_name {
                return Some(signal);
            }
        }
        None
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&[u8; VCD_SIGNAL_SIZE]> {
        let mut previous: Option<&[u8; VCD_SIGNAL_SIZE]> = None;
        for vc in &self.value_changes {
            if timestamp < vc.timestamp {
                break;
            }
            previous = Some(&vc.value);
        }
        previous
    }
}

pub const BUFFER_LENGTH: usize = 512;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}

pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let instruction = read_word(file)?;
    if instruction.is_empty() {
        return Err(io_err("empty instruction"));
    }

    match instruction.as_str() {
        "end" | "dumpvars" | "dumpall" => Ok(()),
        "scope" => {
            *state = match *state {
                State::BeforeModuleDefinitions => State::InsideTopModule,
                State::InsideTopModule => State::InsideInnerModules,
                other => other,
            };
            // fscanf(file, "\n%*[^$]") - skip whitespace then skip until '$' (don't consume).
            skip_whitespace(file)?;
            skip_until(file, |b| b == b'$')?;
            Ok(())
        }
        "upscope" | "enddefinitions" | "comment" => {
            skip_whitespace(file)?;
            skip_until(file, |b| b == b'$')?;
            Ok(())
        }
        "var" => {
            if *state == State::InsideInnerModules {
                // fscanf(file, " %*[^\n]\n") - skip whitespace, skip up to newline, consume newline.
                skip_whitespace(file)?;
                skip_until(file, |b| b == b'\n')?;
                // consume the newline (if present)
                let _ = read_byte(file)?;
                return Ok(());
            }

            // " %*s %zu %[^ ] %[^ $]%*[^$]"
            skip_whitespace(file)?;
            let _type_str = read_word(file)?; // %*s (e.g., "wire", "reg")
            skip_whitespace(file)?;
            let size_str = read_word(file)?;
            let size: usize = size_str
                .parse()
                .map_err(|_| io_err("invalid signal size"))?;
            skip_whitespace(file)?;
            let signal_id = read_until(file, |b| b.is_ascii_whitespace())?;
            skip_whitespace(file)?;
            let name = read_until(file, |b| b == b' ' || b == b'$')?;
            skip_until(file, |b| b == b'$')?;

            // Mirror C behavior: always append a new Signal at signals.len(); the
            // C alias check is dead code (both branches return true).
            let _ = signal_id; // signal_id is used implicitly via insertion order.
            let mut new_signal = Signal {
                name: [0; VCD_NAME_SIZE],
                size,
                value_changes: Vec::new(),
            };
            copy_str_to_array(&name, &mut new_signal.name);
            vcd.signals.push(new_signal);
            Ok(())
        }
        "date" => {
            // fscanf(file, "\n%[^$\n]", vcd->date)
            skip_whitespace(file)?;
            let date = read_until(file, |b| b == b'$' || b == b'\n')?;
            copy_str_to_array(&date, &mut vcd.date);
            Ok(())
        }
        "version" => {
            skip_whitespace(file)?;
            let version = read_until(file, |b| b == b'$' || b == b'\n')?;
            copy_str_to_array(&version, &mut vcd.version);
            Ok(())
        }
        "timescale" => {
            // fscanf(file, "\n\t%zu%[^$\n]", &vcd->timescale.scale, vcd->timescale.unit)
            skip_whitespace(file)?;
            let scale_str = read_digits(file)?;
            let scale: usize = scale_str.parse().unwrap_or(0);
            vcd.timescale.scale = scale;
            // %[^$\n] - read non-$ non-\n; do not consume
            let unit = read_until(file, |b| b == b'$' || b == b'\n')?;
            copy_str_to_array(&unit, &mut vcd.timescale.unit);
            Ok(())
        }
        _ => Err(io_err("unknown instruction")),
    }
}

pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    // fscanf(file, "%u", timestamp) - %u skips leading whitespace.
    skip_whitespace(file)?;
    let digits = read_digits(file)?;
    if digits.is_empty() {
        return Err(io_err("expected timestamp digits"));
    }
    digits
        .parse::<Timestamp>()
        .map_err(|_| io_err("invalid timestamp"))
}

pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    // fscanf(file, "%[^\n]", buffer) - read up to (but not including) newline.
    let line = read_until(file, |b| b == b'\n')?;
    if line.is_empty() {
        return Err(io_err("empty assignment line"));
    }
    let bytes = line.as_bytes();
    let first = bytes[0];
    let is_vector = !matches!(first, b'0' | b'1' | b'x' | b'X' | b'z' | b'Z');

    let (value, signal_id) = if is_vector {
        // "%[^ ] %[^\n]" - split on first space char only.
        let mut split_idx: Option<usize> = None;
        for (i, &b) in bytes.iter().enumerate() {
            if b == b' ' {
                split_idx = Some(i);
                break;
            }
        }
        let idx = match split_idx {
            Some(i) => i,
            None => return Err(io_err("invalid vector assignment")),
        };
        let val = std::str::from_utf8(&bytes[..idx])
            .map_err(|_| io_err("non-utf8 value"))?
            .to_string();
        // skip the spaces, then take the rest until end (line has no \n).
        let mut j = idx;
        while j < bytes.len() && bytes[j] == b' ' {
            j += 1;
        }
        if j >= bytes.len() {
            return Err(io_err("missing signal id in vector assignment"));
        }
        let id = std::str::from_utf8(&bytes[j..])
            .map_err(|_| io_err("non-utf8 signal id"))?
            .to_string();
        (val, id)
    } else {
        // "%1s%[^\n]" - first char is value, the rest is signal id.
        if bytes.len() < 2 {
            return Err(io_err("missing signal id in scalar assignment"));
        }
        let val = std::str::from_utf8(&bytes[..1])
            .map_err(|_| io_err("non-utf8 value"))?
            .to_string();
        let id = std::str::from_utf8(&bytes[1..])
            .map_err(|_| io_err("non-utf8 signal id"))?
            .to_string();
        (val, id)
    };

    // For now, ignore longer signal ids (matches C behavior: return true).
    if signal_id.len() > 1 {
        return Ok(());
    }

    let index = match get_signal_index(&signal_id) {
        Some(i) => i,
        None => return Ok(()),
    };
    if index >= vcd.signals.len() {
        return Ok(());
    }

    let mut value_arr = [0u8; VCD_SIGNAL_SIZE];
    copy_str_to_array(&value, &mut value_arr);
    vcd.signals[index].value_changes.push(ValueChange {
        timestamp: *timestamp,
        value: value_arr,
    });

    Ok(())
}

pub fn get_signal_index(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let c = bytes[0];
    if c < b'!' {
        return None;
    }
    let id = (c - b'!') as usize;
    if id >= VCD_SIGNAL_COUNT {
        None
    } else {
        Some(id)
    }
}
