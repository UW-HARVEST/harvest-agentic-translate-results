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
impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut vcd = VCD {
            signals: Vec::new(),
            date: [0u8; VCD_DATE_SIZE],
            version: [0u8; VCD_VERSION_SIZE],
            timescale: Timescale {
                unit: [0u8; VCD_TIME_UNIT_SIZE],
                scale: 0,
            },
        };
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        loop {
            let character = match read_byte(&file) {
                Some(b) => b,
                None => break,
            };

            if character == b'$' {
                if parse_instruction(&file, &mut vcd, &mut state).is_ok() {
                    continue;
                }
            } else if character == b'#' {
                if let Ok(t) = parse_timestamp(&file) {
                    current_timestamp = t;
                    continue;
                }
            } else if isexpression(character as char) {
                unget_byte(&file);
                if parse_assignment(&file, &mut vcd, &current_timestamp).is_ok() {
                    continue;
                }
            } else if (character as char).is_ascii_whitespace() {
                continue;
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "parse error",
            ));
        }

        Ok(vcd)
    }
    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let target = signal_name.as_bytes();
        for signal in &self.signals {
            let name_len = signal
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(VCD_NAME_SIZE);
            if &signal.name[..name_len] == target {
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

// ---------- File reading helpers ----------

fn read_byte(mut file: &File) -> Option<u8> {
    let mut buf = [0u8; 1];
    match file.read(&mut buf) {
        Ok(1) => Some(buf[0]),
        _ => None,
    }
}

fn unget_byte(mut file: &File) {
    let _ = file.seek(SeekFrom::Current(-1));
}

fn skip_whitespace(file: &File) -> Option<u8> {
    loop {
        match read_byte(file) {
            Some(b) if (b as char).is_ascii_whitespace() => continue,
            other => return other,
        }
    }
}

/// Read a non-whitespace word (skipping leading whitespace, leaving trailing
/// whitespace in the stream).
fn read_word(file: &File) -> Option<String> {
    let first = skip_whitespace(file)?;
    let mut s = String::new();
    s.push(first as char);
    while let Some(b) = read_byte(file) {
        if (b as char).is_ascii_whitespace() {
            unget_byte(file);
            break;
        }
        s.push(b as char);
    }
    Some(s)
}

/// Discard input until the next `$` is reached, leaving the `$` unconsumed.
fn read_until_dollar(file: &File) {
    while let Some(b) = read_byte(file) {
        if b == b'$' {
            unget_byte(file);
            break;
        }
    }
}

/// Read and discard input through the next newline (consuming the newline).
fn skip_to_after_newline(file: &File) {
    while let Some(b) = read_byte(file) {
        if b == b'\n' {
            break;
        }
    }
}

/// Read characters until newline, leaving the newline unconsumed.
fn read_until_newline_no_consume(file: &File) -> String {
    let mut s = String::new();
    while let Some(b) = read_byte(file) {
        if b == b'\n' {
            unget_byte(file);
            break;
        }
        s.push(b as char);
    }
    s
}

/// Skip leading whitespace, then read characters that are neither `$` nor
/// `\n` (as in the `%[^$\n]` scanf directive).
fn read_field_until_dollar_or_newline(file: &File) -> String {
    let mut s = String::new();
    let first = match skip_whitespace(file) {
        Some(b) => b,
        None => return s,
    };
    if first == b'$' || first == b'\n' {
        unget_byte(file);
        return s;
    }
    s.push(first as char);
    while let Some(b) = read_byte(file) {
        if b == b'$' || b == b'\n' {
            unget_byte(file);
            break;
        }
        s.push(b as char);
    }
    s
}

fn io_err(msg: &str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::InvalidData, msg.to_string())
}

fn copy_into_fixed(dst: &mut [u8], src: &[u8]) {
    let copy_len = src.len().min(dst.len().saturating_sub(1));
    dst[..copy_len].copy_from_slice(&src[..copy_len]);
}

pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let instruction = read_word(file).ok_or_else(|| io_err("EOF in instruction"))?;

    match instruction.as_str() {
        "end" | "dumpvars" | "dumpall" => Ok(()),
        "scope" => {
            *state = match *state {
                State::BeforeModuleDefinitions => State::InsideTopModule,
                State::InsideTopModule => State::InsideInnerModules,
                State::InsideInnerModules => State::InsideInnerModules,
            };
            read_until_dollar(file);
            Ok(())
        }
        "upscope" | "enddefinitions" | "comment" => {
            read_until_dollar(file);
            Ok(())
        }
        "var" => {
            if *state == State::InsideInnerModules {
                skip_to_after_newline(file);
                return Ok(());
            }

            // Parse: <type> <size> <signal_id> <name> [<range>] $end
            let _type_str = read_word(file).ok_or_else(|| io_err("var: missing type"))?;
            let size_str = read_word(file).ok_or_else(|| io_err("var: missing size"))?;
            let size: usize = size_str
                .parse()
                .map_err(|_| io_err("var: bad size"))?;
            let signal_id = read_word(file).ok_or_else(|| io_err("var: missing signal id"))?;

            // Read name: like "%[^ $]" — read until a space or `$`.
            let mut name = String::new();
            let first = skip_whitespace(file).ok_or_else(|| io_err("var: missing name"))?;
            if first == b'$' {
                unget_byte(file);
            } else {
                name.push(first as char);
                while let Some(b) = read_byte(file) {
                    if b == b' ' || b == b'$' {
                        unget_byte(file);
                        break;
                    }
                    name.push(b as char);
                }
            }

            // Discard everything up to the next `$` (e.g., `[8:0] `).
            read_until_dollar(file);

            // The C code allocates the signal at position `signals_count` and
            // then performs an alias check that is effectively a no-op (it
            // just returns true regardless). We mirror that by always pushing.
            let _ = signal_id;

            let mut signal = Signal {
                name: [0u8; VCD_NAME_SIZE],
                size,
                value_changes: Vec::new(),
            };
            copy_into_fixed(&mut signal.name, name.as_bytes());
            vcd.signals.push(signal);
            Ok(())
        }
        "date" => {
            let date = read_field_until_dollar_or_newline(file);
            copy_into_fixed(&mut vcd.date, date.as_bytes());
            Ok(())
        }
        "version" => {
            let version = read_field_until_dollar_or_newline(file);
            copy_into_fixed(&mut vcd.version, version.as_bytes());
            Ok(())
        }
        "timescale" => {
            // Format: " %zu%[^$\n]" — skip whitespace, parse a usize, then
            // capture the remaining unit string up to `$` or newline.
            let first = skip_whitespace(file).ok_or_else(|| io_err("timescale: empty"))?;
            if !first.is_ascii_digit() {
                return Err(io_err("timescale: scale not a digit"));
            }
            let mut digits = String::new();
            digits.push(first as char);
            while let Some(b) = read_byte(file) {
                if b.is_ascii_digit() {
                    digits.push(b as char);
                } else {
                    unget_byte(file);
                    break;
                }
            }
            let scale: usize = digits
                .parse()
                .map_err(|_| io_err("timescale: bad scale"))?;
            vcd.timescale.scale = scale;

            let mut unit = String::new();
            while let Some(b) = read_byte(file) {
                if b == b'$' || b == b'\n' {
                    unget_byte(file);
                    break;
                }
                unit.push(b as char);
            }
            copy_into_fixed(&mut vcd.timescale.unit, unit.as_bytes());
            Ok(())
        }
        _ => Err(io_err("unknown instruction")),
    }
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let first = skip_whitespace(file).ok_or_else(|| io_err("timestamp: EOF"))?;
    if !first.is_ascii_digit() {
        unget_byte(file);
        return Err(io_err("timestamp: not a digit"));
    }
    let mut s = String::new();
    s.push(first as char);
    while let Some(b) = read_byte(file) {
        if b.is_ascii_digit() {
            s.push(b as char);
        } else {
            unget_byte(file);
            break;
        }
    }
    s.parse::<Timestamp>()
        .map_err(|_| io_err("timestamp: parse error"))
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let buffer = read_until_newline_no_consume(file);
    if buffer.is_empty() {
        return Err(io_err("assignment: empty line"));
    }

    let first = buffer.as_bytes()[0];
    let is_vector = !matches!(first, b'0' | b'1' | b'x' | b'X' | b'z' | b'Z');

    let (value, signal_id) = if is_vector {
        // Format: "%[^ ] %[^\n]" — value is non-space chars, then any
        // whitespace, then the signal id (rest of the line).
        match buffer.find(' ') {
            Some(p) => {
                let value = buffer[..p].to_string();
                let rest = buffer[p..].trim_start().to_string();
                if rest.is_empty() {
                    return Err(io_err("vector assignment: missing signal id"));
                }
                (value, rest)
            }
            None => return Err(io_err("vector assignment: no space separator")),
        }
    } else {
        // Format: "%1s%[^\n]" — first non-whitespace char is the value, the
        // remainder of the line is the signal id.
        let trimmed = buffer.trim_start();
        if trimmed.len() < 2 {
            return Err(io_err("scalar assignment: too short"));
        }
        let (v, r) = trimmed.split_at(1);
        (v.to_string(), r.to_string())
    };

    // Ignore long signal ids — only single-character ids are stored.
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

    let mut value_array = [0u8; VCD_SIGNAL_SIZE];
    let value_bytes = value.as_bytes();
    let copy_len = value_bytes.len().min(VCD_SIGNAL_SIZE);
    value_array[..copy_len].copy_from_slice(&value_bytes[..copy_len]);

    if vcd.signals[index].value_changes.len() < VCD_VALUE_CHANGE_COUNT {
        vcd.signals[index].value_changes.push(ValueChange {
            timestamp: *timestamp,
            value: value_array,
        });
    }

    Ok(())
}
pub fn get_signal_index(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let id = (bytes[0] as i32) - (b'!' as i32);
    if id < 0 || id >= VCD_SIGNAL_COUNT as i32 {
        return None;
    }
    Some(id as usize)
}
