use std::fs::File;
use std::io::{BufReader, Read};

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

fn copy_to_fixed<const N: usize>(dest: &mut [u8; N], src: &str) {
    let bytes = src.as_bytes();
    let len = bytes.len().min(N - 1);
    dest[..len].copy_from_slice(&bytes[..len]);
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut content = String::new();
        reader.read_to_string(&mut content)?;

        let mut vcd = VCD {
            signals: Vec::new(),
            date: [0u8; VCD_DATE_SIZE],
            version: [0u8; VCD_VERSION_SIZE],
            timescale: Timescale {
                unit: [0u8; VCD_TIME_UNIT_SIZE],
                scale: 0,
            },
        };

        let mut state = State::BeforeModuleDefinitions;
        let mut current_timestamp: Timestamp = 0;
        let chars: Vec<char> = content.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];
            if c == '$' {
                i += 1;
                match parse_instruction_from_chars(&chars, &mut i, &mut vcd, &mut state) {
                    Ok(()) => continue,
                    Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error")),
                }
            } else if c == '#' {
                i += 1;
                match parse_timestamp_from_chars(&chars, &mut i) {
                    Ok(ts) => { current_timestamp = ts; continue; }
                    Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error")),
                }
            } else if isexpression(c) {
                match parse_assignment_from_chars(&chars, &mut i, &mut vcd, current_timestamp) {
                    Ok(()) => continue,
                    Err(_) => return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "parse error")),
                }
            } else if c.is_whitespace() {
                i += 1;
                continue;
            } else {
                return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unexpected character"));
            }
        }

        Ok(vcd)
    }
    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for sig in &self.signals {
            let name_len = sig.name.iter().position(|&b| b == 0).unwrap_or(sig.name.len());
            if &sig.name[..name_len] == signal_name.as_bytes() {
                return Some(sig);
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

// Helper: read a whitespace-delimited word from chars starting at pos
fn read_word(chars: &[char], pos: &mut usize) -> String {
    // skip whitespace
    while *pos < chars.len() && chars[*pos].is_whitespace() {
        *pos += 1;
    }
    let mut word = String::new();
    while *pos < chars.len() && !chars[*pos].is_whitespace() {
        word.push(chars[*pos]);
        *pos += 1;
    }
    word
}

// Skip until we find '$'
fn skip_until_dollar(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos] != '$' {
        *pos += 1;
    }
}

// Read rest of line
fn read_line(chars: &[char], pos: &mut usize) -> String {
    let mut line = String::new();
    while *pos < chars.len() && chars[*pos] != '\n' {
        line.push(chars[*pos]);
        *pos += 1;
    }
    if *pos < chars.len() {
        *pos += 1; // skip newline
    }
    line
}

// Read until '$' or newline, trimming
fn read_until_dollar_or_newline(chars: &[char], pos: &mut usize) -> String {
    // skip leading newline
    if *pos < chars.len() && chars[*pos] == '\n' {
        *pos += 1;
    }
    let mut result = String::new();
    while *pos < chars.len() && chars[*pos] != '$' && chars[*pos] != '\n' {
        result.push(chars[*pos]);
        *pos += 1;
    }
    result
}

fn parse_instruction_from_chars(
    chars: &[char],
    pos: &mut usize,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let instruction = read_word(chars, pos);
    if instruction.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "empty instruction"));
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
        skip_until_dollar(chars, pos);
        return Ok(());
    }

    if instruction == "upscope" || instruction == "enddefinitions" || instruction == "comment" {
        skip_until_dollar(chars, pos);
        return Ok(());
    }

    if instruction == "var" {
        if *state == State::InsideInnerModules {
            // skip to end of line
            read_line(chars, pos);
            return Ok(());
        }

        // Parse: type size signal_id name
        let _var_type = read_word(chars, pos); // e.g. "wire", "reg"
        let size_str = read_word(chars, pos);
        let size: usize = size_str.parse().unwrap_or(0);
        let signal_id = read_word(chars, pos);

        // Read name - everything until '$' or end, trimmed
        // skip whitespace
        while *pos < chars.len() && chars[*pos] == ' ' {
            *pos += 1;
        }
        let mut name = String::new();
        while *pos < chars.len() && chars[*pos] != '$' && chars[*pos] != '\n' {
            name.push(chars[*pos]);
            *pos += 1;
        }
        let name = name.trim().to_string();
        // Remove trailing stuff after space that might be array notation like [8:0]
        // C format: %[^ $] means read until space or $
        // The name in C is read with %[^ $] which stops at space or $
        let name = name.split_whitespace().next().unwrap_or("").to_string();

        let index = get_signal_index(&signal_id);

        // Add signal to vcd
        let mut sig = Signal {
            name: [0u8; VCD_NAME_SIZE],
            size,
            value_changes: Vec::new(),
        };
        copy_to_fixed(&mut sig.name, &name);
        vcd.signals.push(sig);

        // Check if alias (signal at index already has size != 0)
        if let Some(idx) = index {
            if idx < vcd.signals.len() - 1 && vcd.signals[idx].size != 0 {
                // alias - already added, that's fine per C code
                return Ok(());
            }
        }

        return Ok(());
    }

    if instruction == "date" {
        let text = read_until_dollar_or_newline(chars, pos);
        copy_to_fixed(&mut vcd.date, text.trim_start_matches('\t'));
        return Ok(());
    }

    if instruction == "version" {
        let text = read_until_dollar_or_newline(chars, pos);
        copy_to_fixed(&mut vcd.version, text.trim_start_matches('\t'));
        return Ok(());
    }

    if instruction == "timescale" {
        let text = read_until_dollar_or_newline(chars, pos);
        let text = text.trim();
        // Parse scale (digits) and unit (rest)
        let mut scale_str = String::new();
        let mut unit_str = String::new();
        let mut in_digits = true;
        for ch in text.chars() {
            if in_digits && ch.is_ascii_digit() {
                scale_str.push(ch);
            } else {
                in_digits = false;
                unit_str.push(ch);
            }
        }
        vcd.timescale.scale = scale_str.parse().unwrap_or(0);
        copy_to_fixed(&mut vcd.timescale.unit, &unit_str);
        return Ok(());
    }

    Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "unknown instruction"))
}

fn parse_timestamp_from_chars(
    chars: &[char],
    pos: &mut usize,
) -> Result<Timestamp, std::io::Error> {
    let mut num_str = String::new();
    while *pos < chars.len() && chars[*pos].is_ascii_digit() {
        num_str.push(chars[*pos]);
        *pos += 1;
    }
    num_str.parse::<Timestamp>()
        .map_err(|_| std::io::Error::new(std::io::ErrorKind::InvalidData, "bad timestamp"))
}

fn parse_assignment_from_chars(
    chars: &[char],
    pos: &mut usize,
    vcd: &mut VCD,
    timestamp: Timestamp,
) -> Result<(), std::io::Error> {
    // Read rest of line
    let mut buffer = String::new();
    while *pos < chars.len() && chars[*pos] != '\n' {
        buffer.push(chars[*pos]);
        *pos += 1;
    }
    if *pos < chars.len() {
        *pos += 1; // skip newline
    }

    let buffer = buffer.trim().to_string();
    if buffer.is_empty() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "empty assignment"));
    }

    let first = buffer.as_bytes()[0] as char;
    let is_vector = !"01xXzZ".contains(first);

    let (value, signal_id) = if is_vector {
        // format: "value signal_id" split by space
        let mut parts = buffer.splitn(2, ' ');
        let v = parts.next().unwrap_or("");
        let s = parts.next().unwrap_or("").trim();
        if s.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad assignment"));
        }
        (v.to_string(), s.to_string())
    } else {
        // format: first char is value, rest is signal_id
        let v = &buffer[..1];
        let s = &buffer[1..];
        if s.is_empty() {
            return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "bad assignment"));
        }
        (v.to_string(), s.to_string())
    };

    // Ignore longer signal ids
    if signal_id.len() > 1 {
        return Ok(());
    }

    let index = match get_signal_index(&signal_id) {
        Some(idx) => idx,
        None => return Ok(()),
    };

    if index >= vcd.signals.len() {
        return Ok(());
    }

    let mut vc = ValueChange {
        timestamp,
        value: [0u8; VCD_SIGNAL_SIZE],
    };
    let bytes = value.as_bytes();
    let len = bytes.len().min(VCD_SIGNAL_SIZE - 1);
    vc.value[..len].copy_from_slice(&bytes[..len]);

    vcd.signals[index].value_changes.push(vc);
    Ok(())
}

// These keep the original signatures but delegate to the char-based implementations.
// They're not used by read_from_path but must exist per the interface.
pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let mut content = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut content)?;
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    parse_instruction_from_chars(&chars, &mut pos, vcd, state)
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let mut content = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut content)?;
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    parse_timestamp_from_chars(&chars, &mut pos)
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let mut content = String::new();
    let mut reader = BufReader::new(file);
    reader.read_to_string(&mut content)?;
    let chars: Vec<char> = content.chars().collect();
    let mut pos = 0;
    parse_assignment_from_chars(&chars, &mut pos, vcd, *timestamp)
}
pub fn get_signal_index(s: &str) -> Option<usize> {
    let first = s.bytes().next()?;
    let id = first as i32 - b'!' as i32;
    if id < 0 || id as usize >= VCD_SIGNAL_COUNT {
        return None;
    }
    Some(id as usize)
}
