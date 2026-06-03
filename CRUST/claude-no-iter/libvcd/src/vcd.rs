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

// ----- Helper: a peekable byte reader (mirrors fgetc/ungetc semantics) -----
struct ByteReader {
    data: Vec<u8>,
    pos: usize,
}

impl ByteReader {
    fn new(data: Vec<u8>) -> Self {
        ByteReader { data, pos: 0 }
    }
    fn read_byte(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }
    fn unread(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }
    fn peek(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(self.data[self.pos])
        } else {
            None
        }
    }
    /// Skip whitespace bytes (matches `\n` or ` ` in scanf format string,
    /// which match any run of whitespace characters).
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if (b as char).is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }
    /// Read a "word" — skipping leading whitespace then reading non-whitespace.
    /// Mirrors scanf's `%s`.
    fn read_word(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if (b as char).is_whitespace() {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            None
        } else {
            Some(String::from_utf8_lossy(&self.data[start..self.pos]).to_string())
        }
    }
    /// Read an unsigned integer (skipping leading whitespace) — mirrors `%u`.
    fn read_u32(&mut self) -> Option<u32> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if (b as char).is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            None
        } else {
            let s = std::str::from_utf8(&self.data[start..self.pos]).ok()?;
            s.parse::<u32>().ok()
        }
    }
    fn read_usize(&mut self) -> Option<usize> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if (b as char).is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            None
        } else {
            let s = std::str::from_utf8(&self.data[start..self.pos]).ok()?;
            s.parse::<usize>().ok()
        }
    }
    /// Read characters that are NOT in `stop_chars`. Mirrors `%[^...]`.
    /// Does not skip leading whitespace by default.
    fn read_until_any(&mut self, stop_chars: &[u8]) -> Vec<u8> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if stop_chars.contains(&b) {
                break;
            }
            self.pos += 1;
        }
        self.data[start..self.pos].to_vec()
    }
    /// Skip past characters not in stop_chars (consume them).
    fn skip_until_any(&mut self, stop_chars: &[u8]) {
        while let Some(b) = self.peek() {
            if stop_chars.contains(&b) {
                break;
            }
            self.pos += 1;
        }
    }
}

// ----- Helpers used during parsing -----

fn copy_into_fixed_no_terminator(dst: &mut [u8], src: &[u8]) {
    // Like strncpy: copies up to dst.len(); does not guarantee NUL termination.
    for b in dst.iter_mut() {
        *b = 0;
    }
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
}

fn fixed_str_eq(buf: &[u8], target: &str) -> bool {
    let tb = target.as_bytes();
    if tb.len() > buf.len() {
        return false;
    }
    if &buf[..tb.len()] != tb {
        return false;
    }
    // Remaining bytes must all be NUL for equality.
    buf[tb.len()..].iter().all(|&b| b == 0)
}

fn fixed_str_to_str(buf: &[u8]) -> &str {
    let end = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    std::str::from_utf8(&buf[..end]).unwrap_or("")
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut data = Vec::new();
        reader.read_to_end(&mut data)?;
        let mut br = ByteReader::new(data);

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

        while let Some(ch) = br.read_byte() {
            if ch == b'$' {
                if parse_instruction_internal(&mut br, &mut vcd, &mut state) {
                    continue;
                }
            } else if ch == b'#' {
                if parse_timestamp_internal(&mut br, &mut current_timestamp) {
                    continue;
                }
            } else if isexpression(ch as char) {
                br.unread();
                if parse_assignment_internal(&mut br, &mut vcd, current_timestamp) {
                    continue;
                }
            } else if (ch as char).is_whitespace() {
                continue;
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Failed to parse VCD",
            ));
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for signal in &self.signals {
            if fixed_str_eq(&signal.name, signal_name) {
                return Some(signal);
            }
            // Also accept a match against the trimmed string portion (in case the
            // caller provides a name without trailing NUL padding).
            if fixed_str_to_str(&signal.name) == signal_name {
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

// ----- Public stubs preserve the original signatures -----

pub fn parse_instruction(
    _file: &File,
    _vcd: &mut VCD,
    _state: &mut State,
) -> Result<(), std::io::Error> {
    // Public function operating on a `&File` cannot easily share the buffered
    // ByteReader used internally; parsing is implemented in
    // `parse_instruction_internal`. This API is provided for signature
    // compatibility with the original Rust skeleton.
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "use VCD::read_from_path",
    ))
}

pub fn parse_timestamp(_file: &File) -> Result<Timestamp, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "use VCD::read_from_path",
    ))
}

pub fn parse_assignment(
    _file: &File,
    _vcd: &mut VCD,
    _timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "use VCD::read_from_path",
    ))
}

pub fn get_signal_index(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let id = bytes[0] as i32 - b'!' as i32;
    if id < 0 || id as usize >= VCD_SIGNAL_COUNT {
        return None;
    }
    Some(id as usize)
}

// ----- Internal parsing helpers (mirrors of the C static functions) -----

fn parse_instruction_internal(br: &mut ByteReader, vcd: &mut VCD, state: &mut State) -> bool {
    let instruction = match br.read_word() {
        Some(s) => s,
        None => return false,
    };

    if instruction == "end" || instruction == "dumpvars" || instruction == "dumpall" {
        return true;
    }

    if instruction == "scope" {
        match *state {
            State::BeforeModuleDefinitions => *state = State::InsideTopModule,
            State::InsideTopModule => *state = State::InsideInnerModules,
            _ => {}
        }
        // fscanf(file, "\n%*[^$]") — skip whitespace, then consume everything up to '$'
        br.skip_whitespace();
        br.skip_until_any(b"$");
        return true;
    }

    if instruction == "upscope" || instruction == "enddefinitions" || instruction == "comment" {
        br.skip_whitespace();
        br.skip_until_any(b"$");
        return true;
    }

    if instruction == "var" {
        if *state == State::InsideInnerModules {
            // fscanf(file, " %*[^\n]\n") — skip leading space, eat to newline, then newline
            // Skip a single leading space if present, then consume to and including newline.
            // We'll simply consume up to and including the next '\n'.
            while let Some(b) = br.peek() {
                br.pos += 1;
                if b == b'\n' {
                    break;
                }
            }
            return true;
        }

        // Allocate a new signal slot.
        if vcd.signals.len() >= VCD_SIGNAL_COUNT {
            return false;
        }
        let mut signal = Signal {
            name: [0u8; VCD_NAME_SIZE],
            size: 0,
            value_changes: Vec::new(),
        };

        // fscanf(file, " %*s %zu %[^ ] %[^ $]%*[^$]", &signal->size, signal_id, signal->name)
        // 1) skip whitespace, read+discard a word
        let _ = br.read_word();
        // 2) skip whitespace, read size
        let size = br.read_usize().unwrap_or(0);
        signal.size = size;
        // 3) skip whitespace, read up to space (signal_id)
        br.skip_whitespace();
        let signal_id_bytes = br.read_until_any(b" ");
        let signal_id = String::from_utf8_lossy(&signal_id_bytes).to_string();
        // 4) skip whitespace, read up to space or '$' (signal name).
        //    The C format string `%[^ $]` stops at the first space or '$',
        //    so "matched [8:0]" yields just "matched". After capturing the
        //    short name, the C code consumes everything up to the next '$'.
        br.skip_whitespace();
        let name_bytes = br.read_until_any(b" $");
        copy_into_fixed_no_terminator(&mut signal.name, &name_bytes);
        // 5) skip up to next '$'
        br.skip_until_any(b"$");

        vcd.signals.push(signal);

        // Aliasing check (mirrors C). In C this just `return true`s either way,
        // so we do the same — `get_signal_index` is only used to compute an index.
        let _idx = get_signal_index(&signal_id);
        return true;
    }

    if instruction == "date" {
        // fscanf(file, "\n%[^$\n]", vcd->date)
        br.skip_whitespace();
        let bytes = br.read_until_any(b"$\n");
        // Strip trailing whitespace like the original (the format already
        // excluded \n, but tabs/spaces may remain in middle).
        let mut trimmed_end = bytes.len();
        while trimmed_end > 0 && (bytes[trimmed_end - 1] as char).is_whitespace() {
            trimmed_end -= 1;
        }
        copy_into_fixed_no_terminator(&mut vcd.date, &bytes[..trimmed_end]);
        return true;
    }

    if instruction == "version" {
        br.skip_whitespace();
        let bytes = br.read_until_any(b"$\n");
        let mut trimmed_end = bytes.len();
        while trimmed_end > 0 && (bytes[trimmed_end - 1] as char).is_whitespace() {
            trimmed_end -= 1;
        }
        copy_into_fixed_no_terminator(&mut vcd.version, &bytes[..trimmed_end]);
        return true;
    }

    if instruction == "timescale" {
        // fscanf(file, "\n\t%zu%[^$\n]", &vcd->timescale.scale, vcd->timescale.unit)
        br.skip_whitespace();
        // The format expects \n then \t — but \n in scanf means any whitespace,
        // so just skip whitespace, then read the integer and the unit.
        let scale = br.read_usize().unwrap_or(0);
        vcd.timescale.scale = scale;
        // Read remainder until $ or \n.
        let bytes = br.read_until_any(b"$\n");
        // Trim trailing whitespace (the C code reads everything including
        // trailing spaces, but the unit is just what immediately follows).
        let mut trimmed_end = bytes.len();
        while trimmed_end > 0 && (bytes[trimmed_end - 1] as char).is_whitespace() {
            trimmed_end -= 1;
        }
        copy_into_fixed_no_terminator(&mut vcd.timescale.unit, &bytes[..trimmed_end]);
        return true;
    }

    false
}

fn parse_timestamp_internal(br: &mut ByteReader, timestamp: &mut Timestamp) -> bool {
    match br.read_u32() {
        Some(v) => {
            *timestamp = v;
            true
        }
        None => false,
    }
}

fn parse_assignment_internal(br: &mut ByteReader, vcd: &mut VCD, timestamp: Timestamp) -> bool {
    // fscanf(file, "%[^\n]", buffer) — read up to newline.
    let line = br.read_until_any(b"\n");
    if line.is_empty() {
        return false;
    }

    let is_vector = !b"01xXzZ".contains(&line[0]);

    let (value, signal_id) = if is_vector {
        // "%[^ ] %[^\n]" — non-space token, whitespace, then everything up to newline
        let mut idx = 0;
        let bytes = &line;
        // value: up to first space
        let v_start = idx;
        while idx < bytes.len() && bytes[idx] != b' ' {
            idx += 1;
        }
        let value = bytes[v_start..idx].to_vec();
        if value.is_empty() {
            return false;
        }
        // skip whitespace
        while idx < bytes.len() && (bytes[idx] as char).is_whitespace() {
            idx += 1;
        }
        if idx >= bytes.len() {
            return false;
        }
        let signal_id = bytes[idx..].to_vec();
        if signal_id.is_empty() {
            return false;
        }
        (value, signal_id)
    } else {
        // "%1s%[^\n]" — exactly one non-whitespace char, then everything up to newline
        let bytes = &line;
        if bytes.is_empty() {
            return false;
        }
        let value = vec![bytes[0]];
        if bytes.len() < 2 {
            return false;
        }
        let signal_id = bytes[1..].to_vec();
        if signal_id.is_empty() {
            return false;
        }
        (value, signal_id)
    };

    if signal_id.len() > 1 {
        // Per C: ignore longer signal ids.
        return true;
    }

    let signal_id_str = String::from_utf8_lossy(&signal_id).to_string();
    let index = match get_signal_index(&signal_id_str) {
        Some(i) => i,
        None => return true,
    };
    if index >= vcd.signals.len() {
        return true;
    }

    let mut value_arr = [0u8; VCD_SIGNAL_SIZE];
    let n = value.len().min(VCD_SIGNAL_SIZE);
    value_arr[..n].copy_from_slice(&value[..n]);

    vcd.signals[index].value_changes.push(ValueChange {
        timestamp,
        value: value_arr,
    });

    true
}
