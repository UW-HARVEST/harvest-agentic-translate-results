use std::fs::File;
use std::io::{self, Read};

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

fn new_vcd() -> VCD {
    VCD {
        signals: Vec::new(),
        date: [0u8; VCD_DATE_SIZE],
        version: [0u8; VCD_VERSION_SIZE],
        timescale: Timescale {
            unit: [0u8; VCD_TIME_UNIT_SIZE],
            scale: 0,
        },
    }
}

/// Reads the entire file into a single string. We do this once and then
/// operate on a cursor (`Reader`) below.
fn read_all(path: &str) -> io::Result<Vec<u8>> {
    let mut file = File::open(path)?;
    let mut buf = Vec::new();
    file.read_to_end(&mut buf)?;
    Ok(buf)
}

/// A simple byte-cursor that supports reads similar to FILE* APIs the
/// C version uses (`fgetc`, `ungetc`, `fscanf` patterns).
struct Reader<'a> {
    data: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(data: &'a [u8]) -> Self {
        Reader { data, pos: 0 }
    }

    fn fgetc(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let b = self.data[self.pos];
            self.pos += 1;
            Some(b)
        } else {
            None
        }
    }

    fn ungetc(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    /// Skip whitespace bytes (matches C's `isspace`).
    fn skip_whitespace(&mut self) {
        while let Some(b) = self.peek() {
            if (b as char).is_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// `%s` from fscanf: skip whitespace, then read until whitespace or EOF.
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
            Some(String::from_utf8_lossy(&self.data[start..self.pos]).into_owned())
        }
    }

    /// Reads bytes until a stop condition (without skipping leading whitespace).
    fn read_until<F: Fn(u8) -> bool>(&mut self, stop: F) -> Vec<u8> {
        let start = self.pos;
        while let Some(b) = self.peek() {
            if stop(b) {
                break;
            }
            self.pos += 1;
        }
        self.data[start..self.pos].to_vec()
    }

    /// Skip `[^X]` style format: consume bytes until a character matching `stop` is seen.
    fn skip_until<F: Fn(u8) -> bool>(&mut self, stop: F) {
        while let Some(b) = self.peek() {
            if stop(b) {
                break;
            }
            self.pos += 1;
        }
    }

    /// Reads a `usize` integer with leading whitespace skipping (matches `%zu`).
    fn read_usize(&mut self) -> Option<usize> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            None
        } else {
            std::str::from_utf8(&self.data[start..self.pos])
                .ok()
                .and_then(|s| s.parse().ok())
        }
    }

    /// Reads a `u32` integer with leading whitespace skipping (matches `%u`).
    fn read_u32(&mut self) -> Option<u32> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(b) = self.peek() {
            if b.is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            None
        } else {
            std::str::from_utf8(&self.data[start..self.pos])
                .ok()
                .and_then(|s| s.parse().ok())
        }
    }
}

/// Copy a byte slice into a fixed-size array, zero-padding the rest.
fn copy_to_fixed<const N: usize>(dst: &mut [u8; N], src: &[u8]) {
    let n = src.len().min(N);
    for i in 0..n {
        dst[i] = src[i];
    }
    for i in n..N {
        dst[i] = 0;
    }
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let data = read_all(path)?;
        let mut reader = Reader::new(&data);
        let mut vcd = new_vcd();
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        while let Some(c) = reader.fgetc() {
            if c == b'$' {
                if parse_instruction_inner(&mut reader, &mut vcd, &mut state) {
                    continue;
                }
            } else if c == b'#' {
                if let Some(ts) = parse_timestamp_inner(&mut reader) {
                    current_timestamp = ts;
                    continue;
                }
            } else if isexpression(c as char) {
                reader.ungetc();
                if parse_assignment_inner(&mut reader, &mut vcd, current_timestamp) {
                    continue;
                }
            } else if (c as char).is_whitespace() {
                continue;
            }

            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "Failed to parse VCD",
            ));
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for signal in &self.signals {
            if name_matches(&signal.name, signal_name) {
                return Some(signal);
            }
        }
        None
    }
}

/// Compares a fixed-size, null-terminated byte buffer against a target string.
fn name_matches(buf: &[u8], target: &str) -> bool {
    let nul = buf.iter().position(|&b| b == 0).unwrap_or(buf.len());
    &buf[..nul] == target.as_bytes()
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
pub enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}

pub fn parse_instruction(
    _file: &File,
    _vcd: &mut VCD,
    _state: &mut State,
) -> Result<(), std::io::Error> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "use VCD::read_from_path",
    ))
}

pub fn parse_timestamp(_file: &File) -> Result<Timestamp, std::io::Error> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "use VCD::read_from_path",
    ))
}

pub fn parse_assignment(
    _file: &File,
    _vcd: &mut VCD,
    _timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "use VCD::read_from_path",
    ))
}

pub fn get_signal_index(s: &str) -> Option<usize> {
    let bytes = s.as_bytes();
    if bytes.is_empty() {
        return None;
    }
    let id = (bytes[0] as i32) - (b'!' as i32);
    if id < 0 || id as usize >= VCD_SIGNAL_COUNT {
        None
    } else {
        Some(id as usize)
    }
}

// -----------------------------------------------------------------------------
// Internal parser implementations operating on `Reader`.
// -----------------------------------------------------------------------------

fn parse_instruction_inner(reader: &mut Reader, vcd: &mut VCD, state: &mut State) -> bool {
    let instruction = match reader.read_word() {
        Some(s) => s,
        None => return false,
    };

    match instruction.as_str() {
        "end" | "dumpvars" | "dumpall" => true,
        "scope" => {
            match *state {
                State::BeforeModuleDefinitions => *state = State::InsideTopModule,
                State::InsideTopModule => *state = State::InsideInnerModules,
                _ => {}
            }
            // C: fscanf(file, "\n%*[^$]") -- skip a newline then read until '$'.
            // Here, we simply skip everything up to the next '$'.
            reader.skip_until(|b| b == b'$');
            true
        }
        "upscope" | "enddefinitions" | "comment" => {
            reader.skip_until(|b| b == b'$');
            true
        }
        "var" => {
            if *state == State::InsideInnerModules {
                // Skip the rest of the line (matches `fscanf(file, " %*[^\n]\n")`).
                reader.skip_until(|b| b == b'\n');
                if reader.peek() == Some(b'\n') {
                    reader.pos += 1;
                }
                return true;
            }

            // C: fscanf(file, " %*s %zu %[^ ] %[^ $]%*[^$]", &size, signal_id, name)
            // First %*s -- skip a whitespace-delimited word (the wire/reg type).
            let _wire_type = match reader.read_word() {
                Some(s) => s,
                None => return false,
            };
            // %zu -- size
            let size = match reader.read_usize() {
                Some(n) => n,
                None => return false,
            };
            // %[^ ] -- read until a space (after skipping whitespace).
            reader.skip_whitespace();
            let signal_id_bytes = reader.read_until(|b| b == b' ');
            // %[^ $] -- read until a space or '$'. The C scanf would skip a single
            // space between conversions; emulate by skipping whitespace.
            reader.skip_whitespace();
            let name_bytes = reader.read_until(|b| b == b' ' || b == b'$');
            // %*[^$] -- skip until '$'.
            reader.skip_until(|b| b == b'$');

            let signal_id = String::from_utf8_lossy(&signal_id_bytes).into_owned();
            let index = match get_signal_index(&signal_id) {
                Some(i) => i,
                None => return true,
            };

            // Ensure we have enough slots. The C version uses a fixed-size array;
            // we mirror it dynamically but place each signal at its derived index.
            while vcd.signals.len() <= index {
                vcd.signals.push(Signal {
                    name: [0u8; VCD_NAME_SIZE],
                    size: 0,
                    value_changes: Vec::new(),
                });
            }

            // If this slot already has a signal (alias case), keep the existing entry.
            if vcd.signals[index].size != 0 {
                return true;
            }

            let signal = &mut vcd.signals[index];
            // Trim a single trailing space from the name (the read_until stops on a
            // space, but if there's no space and a '$' came next, we still keep the
            // existing trailing whitespace handling).
            let mut trimmed_name = name_bytes.as_slice();
            while !trimmed_name.is_empty()
                && (trimmed_name[trimmed_name.len() - 1] == b' '
                    || trimmed_name[trimmed_name.len() - 1] == b'\t')
            {
                trimmed_name = &trimmed_name[..trimmed_name.len() - 1];
            }
            copy_to_fixed(&mut signal.name, trimmed_name);
            signal.size = size;

            true
        }
        "date" => {
            // C: fscanf(file, "\n%[^$\n]", vcd->date) -- consume a newline, then
            // read until '$' or '\n'.
            // Skip a single newline first.
            consume_optional_newline(reader);
            // The C format reads bytes until '$' or '\n'. Strip surrounding tabs/spaces.
            let bytes = reader.read_until(|b| b == b'$' || b == b'\n');
            let trimmed = trim_ascii_whitespace(&bytes);
            copy_to_fixed(&mut vcd.date, trimmed);
            true
        }
        "version" => {
            consume_optional_newline(reader);
            let bytes = reader.read_until(|b| b == b'$' || b == b'\n');
            let trimmed = trim_ascii_whitespace(&bytes);
            copy_to_fixed(&mut vcd.version, trimmed);
            true
        }
        "timescale" => {
            // C: fscanf(file, "\n\t%zu%[^$\n]", &scale, unit)
            consume_optional_newline(reader);
            // Skip an optional tab (the C format expects "\n\t").
            if reader.peek() == Some(b'\t') {
                reader.pos += 1;
            }
            // Skip any other whitespace (defensive).
            while let Some(b) = reader.peek() {
                if b == b' ' || b == b'\t' {
                    reader.pos += 1;
                } else {
                    break;
                }
            }
            let scale = match reader.read_usize() {
                Some(n) => n,
                None => return false,
            };
            vcd.timescale.scale = scale;
            let bytes = reader.read_until(|b| b == b'$' || b == b'\n');
            let trimmed = trim_ascii_whitespace(&bytes);
            copy_to_fixed(&mut vcd.timescale.unit, trimmed);
            true
        }
        _ => false,
    }
}

fn consume_optional_newline(reader: &mut Reader) {
    if reader.peek() == Some(b'\r') {
        reader.pos += 1;
    }
    if reader.peek() == Some(b'\n') {
        reader.pos += 1;
    }
}

fn trim_ascii_whitespace(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && (bytes[start] == b' ' || bytes[start] == b'\t') {
        start += 1;
    }
    while end > start
        && (bytes[end - 1] == b' '
            || bytes[end - 1] == b'\t'
            || bytes[end - 1] == b'\r'
            || bytes[end - 1] == b'\n')
    {
        end -= 1;
    }
    &bytes[start..end]
}

fn parse_timestamp_inner(reader: &mut Reader) -> Option<Timestamp> {
    reader.read_u32()
}

fn parse_assignment_inner(reader: &mut Reader, vcd: &mut VCD, timestamp: Timestamp) -> bool {
    // Read up to a newline (matches `fscanf(file, "%[^\n]", buffer)`).
    let buffer = reader.read_until(|b| b == b'\n');
    if buffer.is_empty() {
        return false;
    }

    let first = buffer[0] as char;
    let is_vector = !matches!(first, '0' | '1' | 'x' | 'X' | 'z' | 'Z');

    let (value_bytes, signal_id_bytes): (Vec<u8>, Vec<u8>) = if is_vector {
        // %[^ ] %[^\n] -- value is everything up to a space; signal id is the rest.
        let space_idx = match buffer.iter().position(|&b| b == b' ') {
            Some(i) => i,
            None => return false,
        };
        let value = buffer[..space_idx].to_vec();
        // Skip the space and any further leading whitespace.
        let mut rest_start = space_idx + 1;
        while rest_start < buffer.len()
            && (buffer[rest_start] == b' ' || buffer[rest_start] == b'\t')
        {
            rest_start += 1;
        }
        if rest_start >= buffer.len() {
            return false;
        }
        // Trim trailing whitespace from the signal id.
        let mut end = buffer.len();
        while end > rest_start
            && (buffer[end - 1] == b' '
                || buffer[end - 1] == b'\t'
                || buffer[end - 1] == b'\r')
        {
            end -= 1;
        }
        (value, buffer[rest_start..end].to_vec())
    } else {
        // %1s%[^\n] -- value is one char; signal id is the rest.
        if buffer.len() < 2 {
            return false;
        }
        let value = vec![buffer[0]];
        let mut end = buffer.len();
        while end > 1
            && (buffer[end - 1] == b' '
                || buffer[end - 1] == b'\t'
                || buffer[end - 1] == b'\r')
        {
            end -= 1;
        }
        (value, buffer[1..end].to_vec())
    };

    // For now, we ignore longer signal ids.
    if signal_id_bytes.len() > 1 {
        return true;
    }

    let signal_id = String::from_utf8_lossy(&signal_id_bytes).into_owned();
    let index = match get_signal_index(&signal_id) {
        Some(i) => i,
        None => return true,
    };
    if index >= vcd.signals.len() {
        return true;
    }

    let signal = &mut vcd.signals[index];
    let mut value_array = [0u8; VCD_SIGNAL_SIZE];
    let n = value_bytes.len().min(VCD_SIGNAL_SIZE);
    value_array[..n].copy_from_slice(&value_bytes[..n]);
    signal.value_changes.push(ValueChange {
        timestamp,
        value: value_array,
    });

    true
}
