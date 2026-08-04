use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};

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

// ---- Helpers for byte-level parsing on a shared `&File` ----

fn read_byte(file: &File) -> io::Result<Option<u8>> {
    let mut buf = [0u8; 1];
    let mut f: &File = file;
    match f.read(&mut buf)? {
        0 => Ok(None),
        _ => Ok(Some(buf[0])),
    }
}

fn unread_byte(file: &File) -> io::Result<()> {
    let mut f: &File = file;
    f.seek(SeekFrom::Current(-1))?;
    Ok(())
}

fn skip_whitespace(file: &File) -> io::Result<()> {
    loop {
        match read_byte(file)? {
            Some(b) if (b as char).is_ascii_whitespace() => continue,
            Some(_) => {
                unread_byte(file)?;
                break;
            }
            None => break,
        }
    }
    Ok(())
}

fn read_while<F: Fn(u8) -> bool>(file: &File, pred: F) -> io::Result<Vec<u8>> {
    let mut s = Vec::new();
    loop {
        match read_byte(file)? {
            Some(b) if pred(b) => s.push(b),
            Some(_) => {
                unread_byte(file)?;
                break;
            }
            None => break,
        }
    }
    Ok(s)
}

fn read_word(file: &File) -> io::Result<String> {
    skip_whitespace(file)?;
    let bytes = read_while(file, |b| !(b as char).is_ascii_whitespace())?;
    Ok(String::from_utf8_lossy(&bytes).into_owned())
}

fn read_usize(file: &File) -> io::Result<usize> {
    skip_whitespace(file)?;
    let bytes = read_while(file, |b| b.is_ascii_digit())?;
    if bytes.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected digits"));
    }
    let s = std::str::from_utf8(&bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    s.parse::<usize>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn read_u32(file: &File) -> io::Result<u32> {
    skip_whitespace(file)?;
    let bytes = read_while(file, |b| b.is_ascii_digit())?;
    if bytes.is_empty() {
        return Err(io::Error::new(io::ErrorKind::InvalidData, "expected digits"));
    }
    let s = std::str::from_utf8(&bytes)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    s.parse::<u32>()
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
}

fn copy_with_nul(src: &[u8], dst: &mut [u8]) {
    if dst.is_empty() {
        return;
    }
    let n = src.len().min(dst.len() - 1);
    for i in 0..n {
        dst[i] = src[i];
    }
    dst[n] = 0;
}

fn copy_truncating(src: &[u8], dst: &mut [u8]) {
    let n = src.len().min(dst.len());
    for i in 0..n {
        dst[i] = src[i];
    }
    for i in n..dst.len() {
        dst[i] = 0;
    }
}

// ---- Public API ----

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
            let b = match read_byte(&file)? {
                Some(b) => b,
                None => break,
            };

            if b == b'$' {
                parse_instruction(&file, &mut vcd, &mut state)?;
                continue;
            } else if b == b'#' {
                current_timestamp = parse_timestamp(&file)?;
                continue;
            } else if isexpression(b as char) {
                unread_byte(&file)?;
                parse_assignment(&file, &mut vcd, &current_timestamp)?;
                continue;
            } else if (b as char).is_ascii_whitespace() {
                continue;
            } else {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("unexpected character: {:?}", b as char),
                ));
            }
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for signal in &self.signals {
            let end = signal
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(VCD_NAME_SIZE);
            let name_slice = &signal.name[..end];
            if name_slice == signal_name.as_bytes() {
                return Some(signal);
            }
        }
        None
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&[u8; VCD_SIGNAL_SIZE]> {
        let mut previous: Option<&[u8; VCD_SIGNAL_SIZE]> = None;
        for change in &self.value_changes {
            if timestamp < change.timestamp {
                break;
            }
            previous = Some(&change.value);
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
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "expected instruction",
        ));
    }

    if instruction == "end" || instruction == "dumpvars" || instruction == "dumpall" {
        return Ok(());
    }

    if instruction == "scope" {
        match *state {
            State::BeforeModuleDefinitions => *state = State::InsideTopModule,
            State::InsideTopModule => *state = State::InsideInnerModules,
            _ => {}
        }
        // Format: "\n%*[^$]" — skip whitespace then non-'$' chars (stop at '$').
        skip_whitespace(file)?;
        read_while(file, |b| b != b'$')?;
        return Ok(());
    }

    if instruction == "upscope" || instruction == "enddefinitions" || instruction == "comment" {
        skip_whitespace(file)?;
        read_while(file, |b| b != b'$')?;
        return Ok(());
    }

    if instruction == "var" {
        if *state == State::InsideInnerModules {
            // Format: " %*[^\n]\n" — skip ws, skip non-newline chars, skip ws (incl newline)
            skip_whitespace(file)?;
            read_while(file, |b| b != b'\n')?;
            skip_whitespace(file)?;
            return Ok(());
        }

        // Format: " %*s %zu %[^ ] %[^ $]%*[^$]"
        skip_whitespace(file)?;
        let _type_word = read_while(file, |b| !(b as char).is_ascii_whitespace())?;
        skip_whitespace(file)?;
        let size = read_usize(file)?;
        skip_whitespace(file)?;
        let _signal_id = read_while(file, |b| b != b' ')?;
        skip_whitespace(file)?;
        let name = read_while(file, |b| b != b' ' && b != b'$')?;
        // %*[^$] — skip chars not equal to '$'
        read_while(file, |b| b != b'$')?;

        let mut signal = Signal {
            name: [0u8; VCD_NAME_SIZE],
            size,
            value_changes: Vec::new(),
        };
        copy_with_nul(&name, &mut signal.name);
        vcd.signals.push(signal);

        return Ok(());
    }

    if instruction == "date" {
        // Format: "\n%[^$\n]"
        skip_whitespace(file)?;
        let bytes = read_while(file, |b| b != b'$' && b != b'\n')?;
        copy_with_nul(&bytes, &mut vcd.date);
        return Ok(());
    }

    if instruction == "version" {
        skip_whitespace(file)?;
        let bytes = read_while(file, |b| b != b'$' && b != b'\n')?;
        copy_with_nul(&bytes, &mut vcd.version);
        return Ok(());
    }

    if instruction == "timescale" {
        // Format: "\n\t%zu%[^$\n]"
        skip_whitespace(file)?;
        let scale = read_usize(file)?;
        let bytes = read_while(file, |b| b != b'$' && b != b'\n')?;
        vcd.timescale.scale = scale;
        copy_with_nul(&bytes, &mut vcd.timescale.unit);
        return Ok(());
    }

    Err(io::Error::new(
        io::ErrorKind::InvalidData,
        format!("unknown instruction: {}", instruction),
    ))
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    read_u32(file)
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    // Read until newline (do not consume newline).
    let buffer_bytes = read_while(file, |b| b != b'\n')?;
    if buffer_bytes.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "empty assignment line",
        ));
    }

    let first_char = buffer_bytes[0];
    let is_vector = !b"01xXzZ".contains(&first_char);

    let (value_bytes, signal_id_bytes): (Vec<u8>, Vec<u8>) = if is_vector {
        // "%[^ ] %[^\n]"
        let space_pos = match buffer_bytes.iter().position(|&b| b == b' ') {
            Some(p) => p,
            None => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "vector assignment missing separator",
                ));
            }
        };
        let value = buffer_bytes[..space_pos].to_vec();
        // Skip whitespace between value and signal_id
        let mut rest = &buffer_bytes[space_pos..];
        while !rest.is_empty() && (rest[0] as char).is_ascii_whitespace() {
            rest = &rest[1..];
        }
        if rest.is_empty() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "vector assignment missing signal id",
            ));
        }
        (value, rest.to_vec())
    } else {
        // "%1s%[^\n]"
        if buffer_bytes.len() < 2 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "scalar assignment too short",
            ));
        }
        let value = vec![buffer_bytes[0]];
        let signal_id = buffer_bytes[1..].to_vec();
        (value, signal_id)
    };

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
    copy_truncating(&value_bytes, &mut value_arr);

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
    let id = bytes[0] as i32 - b'!' as i32;
    if id < 0 || id >= VCD_SIGNAL_COUNT as i32 {
        return None;
    }
    Some(id as usize)
}
