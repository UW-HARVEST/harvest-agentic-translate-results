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

#[derive(Debug, Clone)]
pub struct ValueChange {
    pub timestamp: Timestamp,
    pub value: Vec<u8>,
}

#[derive(Debug)]
pub struct Signal {
    pub name: Vec<u8>,
    pub size: usize,
    pub value_changes: Vec<ValueChange>,
}

#[derive(Debug)]
pub struct Timescale {
    pub unit: Vec<u8>,
    pub scale: usize,
}

#[derive(Debug)]
pub struct VCD {
    pub signals: Vec<Signal>,
    pub date: Vec<u8>,
    pub version: Vec<u8>,
    pub timescale: Timescale,
}

fn new_vcd() -> VCD {
    VCD {
        signals: Vec::new(),
        date: Vec::new(),
        version: Vec::new(),
        timescale: Timescale {
            unit: Vec::new(),
            scale: 0,
        },
    }
}

fn read_byte(file: &File) -> io::Result<Option<u8>> {
    let mut buf = [0u8; 1];
    let mut f: &File = file;
    let n = f.read(&mut buf)?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(buf[0]))
    }
}

fn unread_byte(file: &File) -> io::Result<()> {
    let mut f: &File = file;
    f.seek(SeekFrom::Current(-1))?;
    Ok(())
}

/// Skip leading whitespace and read characters until the next whitespace.
/// Returns an empty string at EOF.
fn read_token(file: &File) -> io::Result<String> {
    let mut tok = String::new();
    // Skip leading whitespace.
    loop {
        match read_byte(file)? {
            Some(b) if (b as char).is_ascii_whitespace() => continue,
            Some(b) => {
                tok.push(b as char);
                break;
            }
            None => return Ok(tok),
        }
    }
    // Read non-whitespace.
    loop {
        match read_byte(file)? {
            Some(b) if (b as char).is_ascii_whitespace() => break,
            Some(b) => tok.push(b as char),
            None => break,
        }
    }
    Ok(tok)
}

/// Skip any leading whitespace, then read characters until the next '$'
/// (without consuming the '$'). Trims trailing whitespace before returning.
fn read_until_dollar(file: &File) -> io::Result<String> {
    let mut s = String::new();
    let mut started = false;
    loop {
        match read_byte(file)? {
            Some(b'$') => {
                unread_byte(file)?;
                break;
            }
            Some(b) => {
                if !started && (b as char).is_ascii_whitespace() {
                    continue;
                }
                started = true;
                s.push(b as char);
            }
            None => break,
        }
    }
    while let Some(c) = s.chars().last() {
        if c.is_ascii_whitespace() {
            s.pop();
        } else {
            break;
        }
    }
    Ok(s)
}

/// Skip any leading whitespace, then read characters until '$' or '\n'
/// (without consuming the terminator). Trims trailing whitespace.
fn read_until_dollar_or_newline(file: &File) -> io::Result<String> {
    let mut s = String::new();
    let mut started = false;
    loop {
        match read_byte(file)? {
            Some(b'$') => {
                unread_byte(file)?;
                break;
            }
            Some(b'\n') => {
                unread_byte(file)?;
                break;
            }
            Some(b) => {
                if !started && (b as char).is_ascii_whitespace() {
                    continue;
                }
                started = true;
                s.push(b as char);
            }
            None => break,
        }
    }
    while let Some(c) = s.chars().last() {
        if c.is_ascii_whitespace() {
            s.pop();
        } else {
            break;
        }
    }
    Ok(s)
}

/// Read up to but not including the next newline. Does not skip leading
/// whitespace and does not trim.
fn read_until_newline(file: &File) -> io::Result<String> {
    let mut s = String::new();
    loop {
        match read_byte(file)? {
            Some(b'\n') => {
                unread_byte(file)?;
                break;
            }
            Some(b) => s.push(b as char),
            None => break,
        }
    }
    Ok(s)
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut vcd = new_vcd();
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        loop {
            let byte = match read_byte(&file)? {
                Some(b) => b,
                None => break,
            };

            if byte == b'$' {
                parse_instruction(&file, &mut vcd, &mut state)?;
                continue;
            } else if byte == b'#' {
                current_timestamp = parse_timestamp(&file)?;
                continue;
            } else if isexpression(byte as char) {
                unread_byte(&file)?;
                parse_assignment(&file, &mut vcd, &current_timestamp)?;
                continue;
            } else if (byte as char).is_ascii_whitespace() {
                continue;
            }

            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("Unexpected character: {:?}", byte as char),
            ));
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let target = signal_name.as_bytes();
        self.signals.iter().find(|s| s.name == target)
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&[u8]> {
        let mut previous: Option<&[u8]> = None;
        for change in &self.value_changes {
            if timestamp < change.timestamp {
                break;
            }
            previous = Some(change.value.as_slice());
        }
        previous
    }
}

pub const BUFFER_LENGTH: usize = 512;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}

pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let instruction = read_token(file)?;
    if instruction.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::UnexpectedEof,
            "Expected instruction word",
        ));
    }

    match instruction.as_str() {
        "end" | "dumpvars" | "dumpall" => Ok(()),

        "scope" => {
            *state = match *state {
                State::BeforeModuleDefinitions => State::InsideTopModule,
                State::InsideTopModule => State::InsideInnerModules,
                other => other,
            };
            // Consume everything up to (but not including) the next '$'.
            let _ = read_until_dollar(file)?;
            Ok(())
        }

        "upscope" | "enddefinitions" | "comment" => {
            let _ = read_until_dollar(file)?;
            Ok(())
        }

        "var" => {
            if *state == State::InsideInnerModules {
                // Inner modules: ignore the var and skip up to the trailing
                // `$end`.
                let _ = read_until_dollar(file)?;
                return Ok(());
            }

            // Parse: <type> <size> <id> <name...>
            let _type_word = read_token(file)?;
            let size_word = read_token(file)?;
            let id_word = read_token(file)?;
            let name_str = read_until_dollar(file)?;

            let size: usize = size_word.parse().map_err(|_| {
                io::Error::new(
                    io::ErrorKind::InvalidData,
                    format!("Invalid signal size: {:?}", size_word),
                )
            })?;

            let index = match get_signal_index(&id_word) {
                Some(i) => i,
                None => return Ok(()),
            };

            // Grow the signals vector so we can address `index` directly,
            // mirroring the C array indexing semantics.
            while vcd.signals.len() <= index {
                vcd.signals.push(Signal {
                    name: Vec::new(),
                    size: 0,
                    value_changes: Vec::new(),
                });
            }

            // If the slot is already populated, this is an alias and we leave
            // the existing definition alone (matches the C behavior).
            if vcd.signals[index].size == 0 {
                vcd.signals[index].size = size;
                vcd.signals[index].name = name_str.into_bytes();
            }

            Ok(())
        }

        "date" => {
            let value = read_until_dollar_or_newline(file)?;
            vcd.date = value.into_bytes();
            Ok(())
        }

        "version" => {
            let value = read_until_dollar_or_newline(file)?;
            vcd.version = value.into_bytes();
            Ok(())
        }

        "timescale" => {
            // Read the scale (an unsigned integer) followed by the unit.
            // Skip whitespace until we hit a digit.
            let mut scale_str = String::new();
            loop {
                match read_byte(file)? {
                    Some(b) if (b as char).is_ascii_whitespace() => continue,
                    Some(b) if (b as char).is_ascii_digit() => {
                        scale_str.push(b as char);
                        break;
                    }
                    Some(_) => {
                        unread_byte(file)?;
                        break;
                    }
                    None => break,
                }
            }
            // Continue reading digits.
            loop {
                match read_byte(file)? {
                    Some(b) if (b as char).is_ascii_digit() => scale_str.push(b as char),
                    Some(_) => {
                        unread_byte(file)?;
                        break;
                    }
                    None => break,
                }
            }
            if !scale_str.is_empty() {
                vcd.timescale.scale = scale_str.parse().unwrap_or(0);
            }
            // Now the unit, ending at '$' or newline.
            let unit = read_until_dollar_or_newline(file)?;
            vcd.timescale.unit = unit.into_bytes();
            Ok(())
        }

        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Unknown instruction: {:?}", instruction),
        )),
    }
}

pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let mut s = String::new();
    // Skip whitespace, then read consecutive digits.
    loop {
        match read_byte(file)? {
            Some(b) if (b as char).is_ascii_whitespace() => continue,
            Some(b) if (b as char).is_ascii_digit() => {
                s.push(b as char);
                break;
            }
            Some(_) => {
                unread_byte(file)?;
                break;
            }
            None => break,
        }
    }
    loop {
        match read_byte(file)? {
            Some(b) if (b as char).is_ascii_digit() => s.push(b as char),
            Some(_) => {
                unread_byte(file)?;
                break;
            }
            None => break,
        }
    }
    if s.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "Failed to parse timestamp",
        ));
    }
    s.parse::<Timestamp>().map_err(|_| {
        io::Error::new(
            io::ErrorKind::InvalidData,
            format!("Invalid timestamp: {:?}", s),
        )
    })
}

pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    // Read the rest of the current line.
    let line = read_until_newline(file)?;
    if line.is_empty() {
        return Ok(());
    }

    let first = line.as_bytes()[0] as char;
    let is_vector = !matches!(first, '0' | '1' | 'x' | 'X' | 'z' | 'Z');

    let (value_str, signal_id): (&str, &str) = if is_vector {
        // <value> <signal_id>
        match line.find(' ') {
            Some(i) => (&line[..i], line[i + 1..].trim_end()),
            None => return Ok(()),
        }
    } else {
        // <single-char value><signal_id>
        (&line[..1], line[1..].trim_end())
    };

    // Ignore signal ids longer than one character (matches C behavior).
    if signal_id.len() != 1 {
        return Ok(());
    }

    let index = match get_signal_index(signal_id) {
        Some(i) => i,
        None => return Ok(()),
    };
    if index >= vcd.signals.len() {
        return Ok(());
    }

    // Strip the leading 'b' for vector values so consumers receive the bit
    // string itself (e.g. "10010" instead of "b10010"), matching the
    // expected test output.
    let stored_value = if is_vector && value_str.starts_with('b') {
        &value_str[1..]
    } else {
        value_str
    };

    vcd.signals[index].value_changes.push(ValueChange {
        timestamp: *timestamp,
        value: stored_value.as_bytes().to_vec(),
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
