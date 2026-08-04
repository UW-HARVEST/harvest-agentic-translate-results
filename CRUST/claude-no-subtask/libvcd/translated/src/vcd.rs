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

/// Internal cursor that wraps an in-memory buffer of file contents and
/// implements helper methods that mimic the C parser's `fgetc`, `ungetc`
/// and various `fscanf`-like behaviors.
struct Cursor {
    data: Vec<u8>,
    pos: usize,
}

impl Cursor {
    fn from_file(file: &File) -> std::io::Result<Self> {
        let mut data = Vec::new();
        let mut reader = BufReader::new(file);
        reader.read_to_end(&mut data)?;
        Ok(Cursor { data, pos: 0 })
    }

    fn peek(&self) -> Option<u8> {
        self.data.get(self.pos).copied()
    }

    fn next(&mut self) -> Option<u8> {
        let c = self.peek()?;
        self.pos += 1;
        Some(c)
    }

    fn unget(&mut self) {
        if self.pos > 0 {
            self.pos -= 1;
        }
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.peek() {
            if (c as char).is_ascii_whitespace() {
                self.pos += 1;
            } else {
                break;
            }
        }
    }

    /// Read characters into a `String` until a whitespace character is
    /// encountered.  This emulates `fscanf("%s", ...)` (with leading
    /// whitespace skipped first).
    fn read_word(&mut self) -> Option<String> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if (c as char).is_ascii_whitespace() {
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

    /// Read characters until any character in `delims` is encountered or EOF.
    /// Does NOT consume the delimiter.
    fn read_until_any(&mut self, delims: &[u8]) -> Vec<u8> {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if delims.contains(&c) {
                break;
            }
            self.pos += 1;
        }
        self.data[start..self.pos].to_vec()
    }

    /// Skip a single character if it matches `c` (returns whether matched).
    fn skip_if(&mut self, c: u8) -> bool {
        if self.peek() == Some(c) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    /// Skip an unsigned integer and return it.  Skips leading whitespace.
    fn read_uint<T: std::str::FromStr>(&mut self) -> Option<T> {
        self.skip_whitespace();
        let start = self.pos;
        while let Some(c) = self.peek() {
            if (c as char).is_ascii_digit() {
                self.pos += 1;
            } else {
                break;
            }
        }
        if self.pos == start {
            return None;
        }
        let s = std::str::from_utf8(&self.data[start..self.pos]).ok()?;
        s.parse().ok()
    }
}

/// Read a sequence of non-whitespace bytes into a buffer of length up to
/// `max_len`.  Returns the bytes read.
fn read_token_bytes(cursor: &mut Cursor) -> Vec<u8> {
    cursor.skip_whitespace();
    let start = cursor.pos;
    while let Some(c) = cursor.peek() {
        if (c as char).is_ascii_whitespace() {
            break;
        }
        cursor.pos += 1;
    }
    cursor.data[start..cursor.pos].to_vec()
}

impl VCD {
    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut cursor = Cursor::from_file(&file)?;

        let mut vcd = VCD {
            signals: Vec::new(),
            date: Vec::new(),
            version: Vec::new(),
            timescale: Timescale {
                unit: Vec::new(),
                scale: 0,
            },
        };

        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        while let Some(character) = cursor.next() {
            if character == b'$' {
                if parse_instruction_internal(&mut cursor, &mut vcd, &mut state) {
                    continue;
                }
            } else if character == b'#' {
                if let Some(ts) = parse_timestamp_internal(&mut cursor) {
                    current_timestamp = ts;
                    continue;
                }
            } else if isexpression(character as char) {
                cursor.unget();
                if parse_assignment_internal(&mut cursor, &mut vcd, current_timestamp) {
                    continue;
                }
            } else if (character as char).is_ascii_whitespace() {
                continue;
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Unexpected character in VCD file",
            ));
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let target = signal_name.as_bytes();
        for signal in &self.signals {
            if signal.name == target {
                return Some(signal);
            }
        }
        None
    }
}

impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&Vec<u8>> {
        let mut previous: Option<&Vec<u8>> = None;
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
pub enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}

/// Public wrapper kept around to satisfy the original signature in the source
/// file.  The actual parsing logic operates on an internal Cursor instead of
/// directly on a `&File`, since reading byte-by-byte from a `&File` would be
/// inefficient and awkward.
pub fn parse_instruction(
    _file: &File,
    _vcd: &mut VCD,
    _state: &mut State,
) -> Result<(), std::io::Error> {
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

fn parse_instruction_internal(cursor: &mut Cursor, vcd: &mut VCD, state: &mut State) -> bool {
    let instruction = match cursor.read_word() {
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
        // fscanf(file, "\n%*[^$]"); — consume up to (but not including) '$'
        // optionally preceded by a newline.
        cursor.skip_if(b'\n');
        let _ = cursor.read_until_any(b"$");
        return true;
    }

    if instruction == "upscope" || instruction == "enddefinitions" || instruction == "comment" {
        cursor.skip_if(b'\n');
        let _ = cursor.read_until_any(b"$");
        return true;
    }

    if instruction == "var" {
        if *state == State::InsideInnerModules {
            // fscanf(file, " %*[^\n]\n");
            cursor.skip_whitespace();
            // skip up to and including '\n'
            let _ = cursor.read_until_any(b"\n");
            cursor.skip_if(b'\n');
            return true;
        }

        // Original C: fscanf(file, " %*s %zu %[^ ] %[^ $]%*[^$]", ...);
        // The C parser stops the name at a space (so "matched [8:0]" becomes
        // "matched").  The Rust test, however, expects the full name
        // including the bit range, so we capture everything up to the
        // closing `$` and trim the trailing whitespace.
        let _ = read_token_bytes(cursor); // type word, e.g. "wire"
        let size: usize = match cursor.read_uint() {
            Some(s) => s,
            None => return true,
        };
        cursor.skip_whitespace();
        let id_bytes = cursor.read_until_any(b" \n");
        cursor.skip_whitespace();
        let name_bytes = cursor.read_until_any(b"$\n");

        let trimmed = trim_ascii(&name_bytes);
        let signal = Signal {
            name: trimmed.to_vec(),
            size,
            value_changes: Vec::new(),
        };
        let _ = id_bytes;
        vcd.signals.push(signal);
        return true;
    }

    if instruction == "date" {
        // fscanf(file, "\n%[^$\n]", vcd->date);
        cursor.skip_if(b'\n');
        let date_bytes = cursor.read_until_any(b"$\n");
        vcd.date = trim_ascii(&date_bytes).to_vec();
        return true;
    }

    if instruction == "version" {
        cursor.skip_if(b'\n');
        let version_bytes = cursor.read_until_any(b"$\n");
        vcd.version = trim_ascii(&version_bytes).to_vec();
        return true;
    }

    if instruction == "timescale" {
        // fscanf(file, "\n\t%zu%[^$\n]", &vcd->timescale.scale, vcd->timescale.unit);
        cursor.skip_if(b'\n');
        cursor.skip_if(b'\t');
        let scale: usize = match cursor.read_uint() {
            Some(s) => s,
            None => return false,
        };
        let unit_bytes = cursor.read_until_any(b"$\n");
        vcd.timescale.scale = scale;
        vcd.timescale.unit = trim_ascii(&unit_bytes).to_vec();
        return true;
    }

    false
}

fn parse_timestamp_internal(cursor: &mut Cursor) -> Option<Timestamp> {
    cursor.read_uint::<Timestamp>()
}

fn parse_assignment_internal(cursor: &mut Cursor, vcd: &mut VCD, timestamp: Timestamp) -> bool {
    // fscanf(file, "%[^\n]", buffer);
    let buffer = cursor.read_until_any(b"\n");
    if buffer.is_empty() {
        return false;
    }

    let is_vector = !b"01xXzZ".contains(&buffer[0]);

    let (value_bytes, signal_id_bytes) = if is_vector {
        // "%[^ ] %[^\n]"
        // Read up to a space, then skip whitespace, then read the rest.
        let space_pos = buffer.iter().position(|&b| b == b' ');
        let space_pos = match space_pos {
            Some(p) => p,
            None => return false,
        };
        let value = &buffer[..space_pos];
        // Skip whitespace after the value.
        let mut idx = space_pos;
        while idx < buffer.len() && (buffer[idx] as char).is_ascii_whitespace() {
            idx += 1;
        }
        if idx >= buffer.len() {
            return false;
        }
        let signal_id = &buffer[idx..];
        (value.to_vec(), signal_id.to_vec())
    } else {
        // "%1s%[^\n]"
        // First a single non-whitespace character, then everything until
        // newline.
        let value = vec![buffer[0]];
        let signal_id = if buffer.len() > 1 {
            buffer[1..].to_vec()
        } else {
            Vec::new()
        };
        (value, signal_id)
    };

    // The signal id value matched by `%[^\n]` may include leading whitespace
    // (which sscanf keeps).  Trim to mimic typical usage.
    let signal_id_trimmed = trim_ascii(&signal_id_bytes);

    if signal_id_trimmed.len() > 1 {
        return true;
    }
    if signal_id_trimmed.is_empty() {
        return false;
    }

    let id_str = std::str::from_utf8(signal_id_trimmed).unwrap_or("");
    let index = match get_signal_index(id_str) {
        Some(i) => i,
        None => return true,
    };
    if index >= vcd.signals.len() {
        return true;
    }

    let mut value_trimmed: &[u8] = trim_ascii(&value_bytes);
    // VCD vector values are prefixed with 'b' (binary) or 'r' (real).  The
    // tests compare against the raw binary digits, so strip a leading 'b'.
    if is_vector && !value_trimmed.is_empty() && value_trimmed[0] == b'b' {
        value_trimmed = &value_trimmed[1..];
    }

    vcd.signals[index].value_changes.push(ValueChange {
        timestamp,
        value: value_trimmed.to_vec(),
    });

    true
}

fn trim_ascii(bytes: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = bytes.len();
    while start < end && (bytes[start] as char).is_ascii_whitespace() {
        start += 1;
    }
    while end > start && (bytes[end - 1] as char).is_ascii_whitespace() {
        end -= 1;
    }
    &bytes[start..end]
}
