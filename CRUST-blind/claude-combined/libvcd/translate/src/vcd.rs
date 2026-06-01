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

// ----- Internal IO helpers -----

fn read_one_byte(file: &File) -> std::io::Result<Option<u8>> {
    let mut f: &File = file;
    let mut buf = [0u8; 1];
    let n = f.read(&mut buf)?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(buf[0]))
    }
}

fn unread_one_byte(file: &File) -> std::io::Result<()> {
    let mut f: &File = file;
    f.seek(SeekFrom::Current(-1))?;
    Ok(())
}

fn skip_whitespace(file: &File) -> std::io::Result<()> {
    loop {
        match read_one_byte(file)? {
            None => break,
            Some(b) => {
                if !(b as char).is_ascii_whitespace() {
                    unread_one_byte(file)?;
                    break;
                }
            }
        }
    }
    Ok(())
}

// Reads characters (skipping leading whitespace) until any whitespace.
// Mimics fscanf %s into a Vec<u8>.
fn read_word(file: &File) -> std::io::Result<Vec<u8>> {
    skip_whitespace(file)?;
    let mut out = Vec::new();
    loop {
        match read_one_byte(file)? {
            None => break,
            Some(b) => {
                if (b as char).is_ascii_whitespace() {
                    unread_one_byte(file)?;
                    break;
                }
                out.push(b);
            }
        }
    }
    Ok(out)
}

// Reads until any of the stop bytes (does NOT consume the stop byte).
fn read_until_any(file: &File, stop: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut out = Vec::new();
    loop {
        match read_one_byte(file)? {
            None => break,
            Some(b) => {
                if stop.contains(&b) {
                    unread_one_byte(file)?;
                    break;
                }
                out.push(b);
            }
        }
    }
    Ok(out)
}

// Skip until we encounter the target byte (does NOT consume it).
fn skip_until(file: &File, target: u8) -> std::io::Result<()> {
    loop {
        match read_one_byte(file)? {
            None => break,
            Some(b) => {
                if b == target {
                    unread_one_byte(file)?;
                    break;
                }
            }
        }
    }
    Ok(())
}

// Read a decimal unsigned integer, skipping leading whitespace.
fn read_decimal(file: &File) -> std::io::Result<usize> {
    skip_whitespace(file)?;
    let mut n: usize = 0;
    let mut got = false;
    loop {
        match read_one_byte(file)? {
            None => break,
            Some(b) => {
                if (b as char).is_ascii_digit() {
                    n = n.wrapping_mul(10).wrapping_add((b - b'0') as usize);
                    got = true;
                } else {
                    unread_one_byte(file)?;
                    break;
                }
            }
        }
    }
    if got {
        Ok(n)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "expected number",
        ))
    }
}

// ----- Public API -----

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
            let c = match read_one_byte(&file)? {
                None => break,
                Some(b) => b,
            };

            if c == b'$' {
                match parse_instruction(&file, &mut vcd, &mut state) {
                    Ok(()) => continue,
                    Err(e) => return Err(e),
                }
            } else if c == b'#' {
                match parse_timestamp(&file) {
                    Ok(ts) => {
                        current_timestamp = ts;
                        continue;
                    }
                    Err(e) => return Err(e),
                }
            } else if isexpression(c as char) {
                // ungetc the byte
                unread_one_byte(&file)?;
                match parse_assignment(&file, &mut vcd, &current_timestamp) {
                    Ok(()) => continue,
                    Err(e) => return Err(e),
                }
            } else if (c as char).is_ascii_whitespace() {
                continue;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "unexpected character",
                ));
            }
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for s in &self.signals {
            let nul = s.name.iter().position(|&b| b == 0).unwrap_or(s.name.len());
            if &s.name[..nul] == signal_name.as_bytes() {
                return Some(s);
            }
        }
        None
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&[u8; VCD_SIGNAL_SIZE]> {
        let mut prev: Option<&[u8; VCD_SIGNAL_SIZE]> = None;
        for vc in &self.value_changes {
            if timestamp < vc.timestamp {
                break;
            }
            prev = Some(&vc.value);
        }
        prev
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
    let word_bytes = read_word(file)?;
    if word_bytes.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no instruction word",
        ));
    }
    let instruction = std::str::from_utf8(&word_bytes).map_err(|e| {
        std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())
    })?;

    match instruction {
        "end" | "dumpvars" | "dumpall" => Ok(()),
        "scope" => {
            *state = match *state {
                State::BeforeModuleDefinitions => State::InsideTopModule,
                State::InsideTopModule => State::InsideInnerModules,
                State::InsideInnerModules => State::InsideInnerModules,
            };
            // fscanf("\n%*[^$]") -> skip whitespace then skip until '$'
            skip_whitespace(file)?;
            skip_until(file, b'$')?;
            Ok(())
        }
        "upscope" | "enddefinitions" | "comment" => {
            skip_whitespace(file)?;
            skip_until(file, b'$')?;
            Ok(())
        }
        "var" => {
            if *state == State::InsideInnerModules {
                // Skip until newline, consuming the newline.
                loop {
                    match read_one_byte(file)? {
                        None => break,
                        Some(b) => {
                            if b == b'\n' {
                                break;
                            }
                        }
                    }
                }
                return Ok(());
            }

            // Format mirrors fscanf(" %*s %zu %[^ ] %[^ $]%*[^$]")
            // Skip the type word.
            let _type_word = read_word(file)?;
            let size = read_decimal(file)?;
            skip_whitespace(file)?;
            let signal_id_bytes = read_until_any(file, b" \t\n")?;
            skip_whitespace(file)?;
            let name_bytes = read_until_any(file, b" \t\n$")?;
            skip_until(file, b'$')?;

            // Push new signal.
            let mut name_arr = [0u8; VCD_NAME_SIZE];
            let copy_len = name_bytes.len().min(VCD_NAME_SIZE - 1);
            name_arr[..copy_len].copy_from_slice(&name_bytes[..copy_len]);
            // Touch signal_id_bytes to silence unused warning while keeping behavior identical.
            let _ = signal_id_bytes;

            vcd.signals.push(Signal {
                name: name_arr,
                size,
                value_changes: Vec::new(),
            });

            Ok(())
        }
        "date" => {
            skip_whitespace(file)?;
            let bytes = read_until_any(file, b"$\n")?;
            let copy_len = bytes.len().min(VCD_DATE_SIZE - 1);
            // Reset the field to zeros first for safety, then copy.
            for b in vcd.date.iter_mut() {
                *b = 0;
            }
            vcd.date[..copy_len].copy_from_slice(&bytes[..copy_len]);
            Ok(())
        }
        "version" => {
            skip_whitespace(file)?;
            let bytes = read_until_any(file, b"$\n")?;
            let copy_len = bytes.len().min(VCD_VERSION_SIZE - 1);
            for b in vcd.version.iter_mut() {
                *b = 0;
            }
            vcd.version[..copy_len].copy_from_slice(&bytes[..copy_len]);
            Ok(())
        }
        "timescale" => {
            // Format: "\n\t%zu%[^$\n]"
            skip_whitespace(file)?;
            let scale = read_decimal(file)?;
            let bytes = read_until_any(file, b"$\n")?;
            let copy_len = bytes.len().min(VCD_TIME_UNIT_SIZE - 1);
            vcd.timescale.scale = scale;
            for b in vcd.timescale.unit.iter_mut() {
                *b = 0;
            }
            vcd.timescale.unit[..copy_len].copy_from_slice(&bytes[..copy_len]);
            Ok(())
        }
        _ => Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "unknown instruction",
        )),
    }
}

pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    // C fscanf("%u", ...) skips leading whitespace, then reads digits.
    skip_whitespace(file)?;
    let mut ts: Timestamp = 0;
    let mut got = false;
    loop {
        match read_one_byte(file)? {
            None => break,
            Some(b) => {
                if (b as char).is_ascii_digit() {
                    ts = ts.wrapping_mul(10).wrapping_add((b - b'0') as Timestamp);
                    got = true;
                } else {
                    unread_one_byte(file)?;
                    break;
                }
            }
        }
    }
    if got {
        Ok(ts)
    } else {
        Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "no timestamp digits",
        ))
    }
}

pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    // Read the line up to (but not including) '\n'.
    let buffer_bytes = read_until_any(file, b"\n")?;
    if buffer_bytes.is_empty() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "empty assignment line",
        ));
    }

    let first = buffer_bytes[0];
    let is_vector = !matches!(first, b'0' | b'1' | b'x' | b'X' | b'z' | b'Z');

    let (value_bytes, signal_id_bytes): (Vec<u8>, Vec<u8>) = if is_vector {
        // Mirror sscanf("%[^ ] %[^\n]")
        let space_pos = match buffer_bytes.iter().position(|&b| b == b' ') {
            Some(p) => p,
            None => {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    "vector assignment missing space",
                ));
            }
        };
        let value = buffer_bytes[..space_pos].to_vec();
        let mut start = space_pos;
        while start < buffer_bytes.len()
            && (buffer_bytes[start] as char).is_ascii_whitespace()
        {
            start += 1;
        }
        if start >= buffer_bytes.len() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "vector assignment missing signal id",
            ));
        }
        let signal_id = buffer_bytes[start..].to_vec();
        (value, signal_id)
    } else {
        // Mirror sscanf("%1s%[^\n]")
        if buffer_bytes.len() < 2 {
            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "scalar assignment missing signal id",
            ));
        }
        (vec![buffer_bytes[0]], buffer_bytes[1..].to_vec())
    };

    // Ignore longer signal ids (the C code does the same).
    if signal_id_bytes.len() > 1 {
        return Ok(());
    }

    let signal_id_str = match std::str::from_utf8(&signal_id_bytes) {
        Ok(s) => s,
        Err(_) => return Ok(()),
    };

    let index = match get_signal_index(signal_id_str) {
        Some(i) => i,
        None => return Ok(()),
    };

    if index >= vcd.signals.len() {
        return Ok(());
    }

    let mut value_arr = [0u8; VCD_SIGNAL_SIZE];
    let copy_len = value_bytes.len().min(VCD_SIGNAL_SIZE);
    value_arr[..copy_len].copy_from_slice(&value_bytes[..copy_len]);

    vcd.signals[index].value_changes.push(ValueChange {
        timestamp: *timestamp,
        value: value_arr,
    });

    Ok(())
}

pub fn get_signal_index(s: &str) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let first = s.as_bytes()[0] as i32;
    let id = first - (b'!' as i32);
    if id < 0 || id >= VCD_SIGNAL_COUNT as i32 {
        None
    } else {
        Some(id as usize)
    }
}
