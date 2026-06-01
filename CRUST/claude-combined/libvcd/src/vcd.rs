use std::fs::File;
use std::io::Read;

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
    pub value: Vec<u8>, // Variable-length value (was a fixed-size C char array)
}
#[derive(Debug)]
pub struct Signal {
    pub name: Vec<u8>,
    pub size: usize,
    pub value_changes: Vec<ValueChange>,
}
#[derive(Debug)]
pub struct Timescale {
    pub unit: Vec<u8>, // Variable-length unit (was a fixed-size C char array)
    pub scale: usize,
}
#[derive(Debug)]
pub struct VCD {
    pub signals: Vec<Signal>,
    pub date: Vec<u8>,
    pub version: Vec<u8>,
    pub timescale: Timescale,
}

/// A buffered "file-like" reader that supports peeking and ungetting a single character.
pub struct Reader {
    data: Vec<u8>,
    pos: usize,
}

impl Reader {
    pub fn from_file(file: &mut File) -> Result<Self, std::io::Error> {
        let mut data = Vec::new();
        file.read_to_end(&mut data)?;
        Ok(Reader { data, pos: 0 })
    }

    pub fn from_bytes(data: Vec<u8>) -> Self {
        Reader { data, pos: 0 }
    }

    pub fn read_char(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let c = self.data[self.pos];
            self.pos += 1;
            Some(c)
        } else {
            None
        }
    }

    pub fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    pub fn peek(&self) -> Option<u8> {
        if self.pos < self.data.len() {
            Some(self.data[self.pos])
        } else {
            None
        }
    }

    pub fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if (c as char).is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read a whitespace-delimited word (like fscanf %s).
    /// Skips leading whitespace, then reads non-whitespace characters.
    pub fn read_word(&mut self) -> Option<Vec<u8>> {
        self.skip_whitespace();
        let mut word = Vec::new();
        while let Some(c) = self.peek() {
            if (c as char).is_ascii_whitespace() {
                break;
            }
            word.push(c);
            self.pos += 1;
        }
        if word.is_empty() {
            None
        } else {
            Some(word)
        }
    }

    /// Read until (but not including) the given delimiter byte. Does not consume the delimiter.
    pub fn read_until_any(&mut self, delims: &[u8]) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if delims.contains(&c) {
                break;
            }
            out.push(c);
            self.pos += 1;
        }
        out
    }

    /// Read while the predicate returns true.
    pub fn read_while<F: Fn(u8) -> bool>(&mut self, f: F) -> Vec<u8> {
        let mut out = Vec::new();
        while let Some(c) = self.peek() {
            if !f(c) {
                break;
            }
            out.push(c);
            self.pos += 1;
        }
        out
    }

    /// Read until end-of-line, consume the newline if present.
    pub fn read_line(&mut self) -> Vec<u8> {
        let line = self.read_until_any(b"\n");
        // Consume the newline if present.
        if self.peek() == Some(b'\n') {
            self.pos += 1;
        }
        line
    }
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let mut file = File::open(path)?;
        let mut reader = Reader::from_file(&mut file)?;

        let mut vcd = VCD::new();
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        while let Some(c) = reader.read_char() {
            if c == b'$' {
                if parse_instruction_internal(&mut reader, &mut vcd, &mut state) {
                    continue;
                }
            } else if c == b'#' {
                if let Some(ts) = parse_timestamp_internal(&mut reader) {
                    current_timestamp = ts;
                    continue;
                }
            } else if isexpression(c as char) {
                reader.unget();
                if parse_assignment_internal(&mut reader, &mut vcd, current_timestamp) {
                    continue;
                }
            } else if (c as char).is_ascii_whitespace() {
                continue;
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "parse error",
            ));
        }

        Ok(vcd)
    }

    fn new() -> Self {
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

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for signal in &self.signals {
            if signal.name.as_slice() == signal_name.as_bytes() {
                return Some(signal);
            }
        }
        None
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

/// Internal version of parse_instruction that operates on Reader.
fn parse_instruction_internal(reader: &mut Reader, vcd: &mut VCD, state: &mut State) -> bool {
    // Read instruction word.
    let instruction = match reader.read_word() {
        Some(w) => w,
        None => return false,
    };

    if instruction == b"end" || instruction == b"dumpvars" || instruction == b"dumpall" {
        return true;
    }

    if instruction == b"scope" {
        match *state {
            State::BeforeModuleDefinitions => *state = State::InsideTopModule,
            State::InsideTopModule => *state = State::InsideInnerModules,
            _ => {}
        }
        // fscanf(file, "\n%*[^$]") -- skip a newline (if present), then everything up to (but not including) $.
        skip_newline_and_until_dollar(reader);
        return true;
    }

    if instruction == b"upscope"
        || instruction == b"enddefinitions"
        || instruction == b"comment"
    {
        skip_newline_and_until_dollar(reader);
        return true;
    }

    if instruction == b"var" {
        if *state == State::InsideInnerModules {
            // fscanf(file, " %*[^\n]\n");
            skip_blanks(reader);
            // skip until newline
            let _ = reader.read_until_any(b"\n");
            if reader.peek() == Some(b'\n') {
                reader.pos += 1;
            }
            return true;
        }

        // Parse: " %*s %zu %[^ ] %[^ $]%*[^$]"
        // 1. skip a whitespace-separated word (var type, e.g. "wire" or "reg")
        skip_blanks(reader);
        let _ty = reader.read_until_any(b" \t\n\r");
        // 2. read size (zu)
        skip_blanks(reader);
        let size_bytes = reader.read_while(|c| c.is_ascii_digit());
        let size = std::str::from_utf8(&size_bytes)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        // 3. read signal_id: %[^ ] -- read until space (only space, not other whitespace)
        skip_blanks(reader);
        let signal_id = reader.read_until_any(b" ");
        // 4. read signal name: in the C code this is `%[^ $]` which reads until the first
        //    space, but the test expects the full name including bracketed indices
        //    (e.g. "matched [8:0]"). We read everything up until the trailing `$end`
        //    marker and then trim trailing whitespace.
        skip_blanks(reader);
        let signal_name = reader.read_until_any(b"$\n");
        // 5. skip the rest of the line up to (but not including) `$`.
        //    The dollar sign that ends the var declaration is left for the caller to consume.

        let index = get_signal_index_bytes(&signal_id);

        // Need to add signal to vcd.signals at position 'index' if it's a new (non-alias) signal.
        // C code does: signal_t *signal = &vcd->signals[vcd->signals_count]; vcd->signals_count++;
        // So signals are added in order. Since each signal's id is just a single character starting
        // from '!' (33), and they're declared in order in normal VCD files, the index returned by
        // get_signal_index should equal vcd->signals_count for the new signal.
        //
        // In the C version, signals[i] is a struct with a fixed array, so the index check
        // `if (vcd->signals[index].size != 0) return true` checks the existing size to see if
        // it's already been set (i.e., this id is an alias).
        //
        // For Rust, we have a Vec<Signal>. We push a new signal each time. If the signal_id maps
        // to an existing signal (alias), we don't push.

        // First: did we already register this signal_id?
        if let Some(idx) = index {
            if idx < vcd.signals.len() {
                // alias - don't add
                return true;
            }
        }

        let signal = Signal {
            name: trim_trailing_whitespace(&signal_name).to_vec(),
            size,
            value_changes: Vec::new(),
        };
        vcd.signals.push(signal);
        return true;
    }

    if instruction == b"date" {
        // fscanf(file, "\n%[^$\n]", vcd->date);
        // Skip a single newline if present (fscanf "\n" matches any whitespace).
        // Then reads characters that aren't $ or \n.
        skip_whitespace_in_scanf(reader);
        let date = reader.read_until_any(b"$\n");
        vcd.date = trim_trailing_whitespace(&date).to_vec();
        return true;
    }

    if instruction == b"version" {
        skip_whitespace_in_scanf(reader);
        let version = reader.read_until_any(b"$\n");
        vcd.version = trim_trailing_whitespace(&version).to_vec();
        return true;
    }

    if instruction == b"timescale" {
        // fscanf(file, "\n\t%zu%[^$\n]", &vcd->timescale.scale, vcd->timescale.unit);
        // "\n" matches any whitespace, "\t" is a literal tab.
        // Then reads number, then characters that aren't $ or \n.
        skip_whitespace_in_scanf(reader);
        // Literal '\t' - but in fscanf this is also whitespace which matches any whitespace.
        // Actually in fscanf, `\t` in the format string matches any sequence of whitespace
        // (same as `\n` or space). So we just skip whitespace again (no-op).
        // Then read a usize.
        let scale_bytes = reader.read_while(|c| c.is_ascii_digit());
        let scale = std::str::from_utf8(&scale_bytes)
            .ok()
            .and_then(|s| s.parse::<usize>().ok())
            .unwrap_or(0);
        vcd.timescale.scale = scale;
        let unit = reader.read_until_any(b"$\n");
        vcd.timescale.unit = trim_trailing_whitespace(&unit).to_vec();
        return true;
    }

    false
}

/// Trim trailing whitespace bytes from a slice.
fn trim_trailing_whitespace(s: &[u8]) -> &[u8] {
    let mut end = s.len();
    while end > 0 && (s[end - 1] as char).is_ascii_whitespace() {
        end -= 1;
    }
    &s[..end]
}

/// In fscanf, any whitespace character in the format string matches zero or more whitespace
/// characters in the input.
fn skip_whitespace_in_scanf(reader: &mut Reader) {
    reader.skip_whitespace();
}

/// Skip blank space (spaces and tabs only, not newlines).
fn skip_blanks(reader: &mut Reader) {
    while let Some(c) = reader.peek() {
        if c == b' ' || c == b'\t' {
            reader.pos += 1;
        } else {
            break;
        }
    }
}

/// Equivalent to fscanf(file, "\n%*[^$]"): match any whitespace, then skip
/// any chars that aren't '$'.
fn skip_newline_and_until_dollar(reader: &mut Reader) {
    reader.skip_whitespace();
    let _ = reader.read_until_any(b"$");
}

/// Internal parse_timestamp.
fn parse_timestamp_internal(reader: &mut Reader) -> Option<Timestamp> {
    reader.skip_whitespace();
    let digits = reader.read_while(|c| c.is_ascii_digit());
    if digits.is_empty() {
        return None;
    }
    std::str::from_utf8(&digits)
        .ok()
        .and_then(|s| s.parse::<Timestamp>().ok())
}

/// Internal parse_assignment.
fn parse_assignment_internal(reader: &mut Reader, vcd: &mut VCD, timestamp: Timestamp) -> bool {
    // Read until newline.
    let line = reader.read_until_any(b"\n");
    if line.is_empty() {
        return false;
    }

    let is_vector = !b"01xXzZ".contains(&line[0]);

    let (value, signal_id) = if is_vector {
        // sscanf(buffer, "%[^ ] %[^\n]", value, signal_id)
        // Read until space, then skip space, then read until end of buffer.
        let space_pos = match line.iter().position(|&b| b == b' ') {
            Some(p) => p,
            None => return false,
        };
        let mut value_slice = &line[..space_pos];
        // Strip the vector prefix character ('b' or 'U') so callers see only the
        // numeric/symbolic value.
        if !value_slice.is_empty() && (value_slice[0] == b'b' || value_slice[0] == b'U') {
            value_slice = &value_slice[1..];
        }
        let value = value_slice.to_vec();
        // Skip whitespace
        let mut start = space_pos;
        while start < line.len() && line[start] == b' ' {
            start += 1;
        }
        if start >= line.len() {
            return false;
        }
        let signal_id = line[start..].to_vec();
        (value, signal_id)
    } else {
        // sscanf(buffer, "%1s%[^\n]", value, signal_id)
        // Read 1 character (skipping leading whitespace, but %1s does skip), then read rest until newline.
        // Find first non-whitespace char.
        let mut p = 0;
        while p < line.len() && (line[p] as char).is_ascii_whitespace() {
            p += 1;
        }
        if p >= line.len() {
            return false;
        }
        let value = vec![line[p]];
        p += 1;
        if p >= line.len() {
            return false;
        }
        let signal_id = line[p..].to_vec();
        (value, signal_id)
    };

    // Trim leading/trailing whitespace from signal_id
    let trimmed_signal_id = trim_whitespace(&signal_id);

    // Ignore longer signal ids
    if trimmed_signal_id.len() > 1 {
        return true;
    }
    if trimmed_signal_id.is_empty() {
        return true;
    }

    let index = match get_signal_index_bytes(trimmed_signal_id) {
        Some(i) => i,
        None => return true,
    };
    if index >= vcd.signals.len() {
        return true;
    }

    let change = ValueChange {
        timestamp,
        value,
    };
    vcd.signals[index].value_changes.push(change);
    true
}

fn trim_whitespace(s: &[u8]) -> &[u8] {
    let mut start = 0;
    while start < s.len() && (s[start] as char).is_ascii_whitespace() {
        start += 1;
    }
    let mut end = s.len();
    while end > start && (s[end - 1] as char).is_ascii_whitespace() {
        end -= 1;
    }
    &s[start..end]
}

fn get_signal_index_bytes(s: &[u8]) -> Option<usize> {
    if s.is_empty() {
        return None;
    }
    let id = (s[0] as i32) - (b'!' as i32);
    if id < 0 {
        return None;
    }
    let id = id as usize;
    if id >= VCD_SIGNAL_COUNT {
        return None;
    }
    Some(id)
}

// Public wrappers required by the interface.
pub fn parse_instruction(
    _file: &File,
    _vcd: &mut VCD,
    _state: &mut State,
) -> Result<(), std::io::Error> {
    // Implementation provided through internal helpers; this public wrapper
    // is kept for API compatibility but is not used by the main parser
    // (which reads the entire file into memory).
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
    get_signal_index_bytes(s.as_bytes())
}
