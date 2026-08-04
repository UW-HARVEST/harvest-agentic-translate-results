use std::fs::File;
use std::io::{ErrorKind, Read, Seek, SeekFrom};

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

fn parse_error(msg: &str) -> std::io::Error {
    std::io::Error::new(ErrorKind::InvalidData, msg)
}

fn is_ws(b: u8) -> bool {
    matches!(b, b' ' | b'\t' | b'\n' | b'\r' | 0x0b | 0x0c)
}

fn read_byte(file: &File) -> std::io::Result<Option<u8>> {
    let mut f = file;
    let mut buf = [0u8; 1];
    match f.read(&mut buf)? {
        0 => Ok(None),
        _ => Ok(Some(buf[0])),
    }
}

fn unread_byte(file: &File) -> std::io::Result<()> {
    let mut f = file;
    f.seek(SeekFrom::Current(-1))?;
    Ok(())
}

fn skip_whitespace(file: &File) -> std::io::Result<()> {
    while let Some(b) = read_byte(file)? {
        if !is_ws(b) {
            unread_byte(file)?;
            break;
        }
    }
    Ok(())
}

// Skip leading whitespace then read a non-whitespace word.
fn read_word(file: &File) -> std::io::Result<Vec<u8>> {
    skip_whitespace(file)?;
    let mut result = Vec::new();
    while let Some(b) = read_byte(file)? {
        if is_ws(b) {
            unread_byte(file)?;
            break;
        }
        result.push(b);
    }
    Ok(result)
}

// Read characters that are NOT in the terminator set. Stops on EOF or terminator.
// File position is left at the terminator (or at EOF).
fn read_until_any(file: &File, terminators: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut result = Vec::new();
    while let Some(b) = read_byte(file)? {
        if terminators.contains(&b) {
            unread_byte(file)?;
            break;
        }
        result.push(b);
    }
    Ok(result)
}

// Read an unsigned decimal number (sequence of digits).
fn read_digits(file: &File) -> std::io::Result<Vec<u8>> {
    let mut result = Vec::new();
    while let Some(b) = read_byte(file)? {
        if !b.is_ascii_digit() {
            unread_byte(file)?;
            break;
        }
        result.push(b);
    }
    Ok(result)
}

fn copy_into_fixed(src: &[u8], dst: &mut [u8]) {
    // Mimic strncpy with a buffer pre-zeroed: copy up to dst.len() bytes
    // (no null terminator added if src fills the buffer; dst is assumed to be
    // pre-zeroed so a shorter src is naturally null-padded).
    let n = src.len().min(dst.len());
    dst[..n].copy_from_slice(&src[..n]);
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

        while let Some(b) = read_byte(&file)? {
            if b == b'$' {
                parse_instruction(&file, &mut vcd, &mut state)?;
            } else if b == b'#' {
                current_timestamp = parse_timestamp(&file)?;
            } else if isexpression(b as char) {
                unread_byte(&file)?;
                parse_assignment(&file, &mut vcd, &current_timestamp)?;
            } else if is_ws(b) {
                continue;
            } else {
                return Err(parse_error("unexpected character in VCD file"));
            }
        }

        Ok(vcd)
    }

    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let target = signal_name.as_bytes();
        self.signals.iter().find(|s| {
            let len = s
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(s.name.len());
            &s.name[..len] == target
        })
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
    let instruction_bytes = read_word(file)?;
    if instruction_bytes.is_empty() {
        return Err(parse_error("expected instruction word"));
    }
    let instruction = std::str::from_utf8(&instruction_bytes)
        .map_err(|_| parse_error("invalid utf-8 in instruction"))?;

    match instruction {
        "end" | "dumpvars" | "dumpall" => Ok(()),

        "scope" => {
            *state = match *state {
                State::BeforeModuleDefinitions => State::InsideTopModule,
                State::InsideTopModule => State::InsideInnerModules,
                other => other,
            };
            // fscanf(file, "\n%*[^$]") -- skip whitespace, then consume up to '$'
            skip_whitespace(file)?;
            read_until_any(file, b"$")?;
            Ok(())
        }

        "upscope" | "enddefinitions" | "comment" => {
            skip_whitespace(file)?;
            read_until_any(file, b"$")?;
            Ok(())
        }

        "var" => {
            if *state == State::InsideInnerModules {
                // " %*[^\n]\n" -- skip whitespace, then consume the rest of the line including '\n'
                skip_whitespace(file)?;
                read_until_any(file, b"\n")?;
                // consume the '\n' itself
                read_byte(file)?;
                return Ok(());
            }

            // " %*s %zu %[^ ] %[^ $]%*[^$]"
            // skip ws, var type word, skip ws, size, skip ws, signal_id, skip ws, name, skip until '$'
            let _var_type = read_word(file)?;
            skip_whitespace(file)?;
            let size_bytes = read_digits(file)?;
            if size_bytes.is_empty() {
                return Err(parse_error("expected var size"));
            }
            let size_str = std::str::from_utf8(&size_bytes)
                .map_err(|_| parse_error("invalid utf-8 in size"))?;
            let size: usize = size_str
                .parse()
                .map_err(|_| parse_error("invalid var size"))?;
            skip_whitespace(file)?;
            let _signal_id_bytes = read_until_any(file, b" ")?;
            skip_whitespace(file)?;
            let name_bytes = read_until_any(file, b" $")?;
            // consume the rest up to '$'
            read_until_any(file, b"$")?;

            let mut name_arr = [0u8; VCD_NAME_SIZE];
            // Leave room for the null terminator so get_signal_by_name finds the end.
            let max_name_bytes = if VCD_NAME_SIZE > 0 {
                VCD_NAME_SIZE - 1
            } else {
                0
            };
            copy_into_fixed(&name_bytes[..name_bytes.len().min(max_name_bytes)], &mut name_arr);

            vcd.signals.push(Signal {
                name: name_arr,
                size,
                value_changes: Vec::new(),
            });

            Ok(())
        }

        "date" => {
            // "\n%[^$\n]"
            skip_whitespace(file)?;
            let date_bytes = read_until_any(file, b"$\n")?;
            vcd.date = [0u8; VCD_DATE_SIZE];
            let max = if VCD_DATE_SIZE > 0 { VCD_DATE_SIZE - 1 } else { 0 };
            copy_into_fixed(&date_bytes[..date_bytes.len().min(max)], &mut vcd.date);
            Ok(())
        }

        "version" => {
            skip_whitespace(file)?;
            let version_bytes = read_until_any(file, b"$\n")?;
            vcd.version = [0u8; VCD_VERSION_SIZE];
            let max = if VCD_VERSION_SIZE > 0 {
                VCD_VERSION_SIZE - 1
            } else {
                0
            };
            copy_into_fixed(
                &version_bytes[..version_bytes.len().min(max)],
                &mut vcd.version,
            );
            Ok(())
        }

        "timescale" => {
            // "\n\t%zu%[^$\n]"
            skip_whitespace(file)?;
            let scale_bytes = read_digits(file)?;
            if scale_bytes.is_empty() {
                return Err(parse_error("expected timescale value"));
            }
            let scale_str = std::str::from_utf8(&scale_bytes)
                .map_err(|_| parse_error("invalid utf-8 in timescale"))?;
            let scale: usize = scale_str
                .parse()
                .map_err(|_| parse_error("invalid timescale value"))?;
            let unit_bytes = read_until_any(file, b"$\n")?;
            vcd.timescale.scale = scale;
            vcd.timescale.unit = [0u8; VCD_TIME_UNIT_SIZE];
            let max = if VCD_TIME_UNIT_SIZE > 0 {
                VCD_TIME_UNIT_SIZE - 1
            } else {
                0
            };
            copy_into_fixed(
                &unit_bytes[..unit_bytes.len().min(max)],
                &mut vcd.timescale.unit,
            );
            Ok(())
        }

        _ => Err(parse_error("unknown instruction")),
    }
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    skip_whitespace(file)?;
    let bytes = read_digits(file)?;
    if bytes.is_empty() {
        return Err(parse_error("expected timestamp digits"));
    }
    let s = std::str::from_utf8(&bytes)
        .map_err(|_| parse_error("invalid utf-8 in timestamp"))?;
    let ts: Timestamp = s
        .parse()
        .map_err(|_| parse_error("invalid timestamp number"))?;
    Ok(ts)
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    // fscanf(file, "%[^\n]", buffer) -- read everything up to (but not including) '\n'
    let buffer = read_until_any(file, b"\n")?;

    if buffer.is_empty() {
        return Err(parse_error("empty assignment line"));
    }

    let first = buffer[0];
    let is_vector = !b"01xXzZ".contains(&first);

    let (value, signal_id): (&[u8], &[u8]) = if is_vector {
        // sscanf(buffer, "%[^ ] %[^\n]", value, signal_id)
        // The first conversion needs at least one non-space char.
        if first == b' ' {
            return Err(parse_error("malformed vector assignment"));
        }
        let space_pos = match buffer.iter().position(|&b| b == b' ') {
            Some(p) => p,
            None => return Err(parse_error("vector assignment missing signal id")),
        };
        let value = &buffer[..space_pos];
        let rest = &buffer[space_pos..];
        // The literal ' ' in the format matches one or more whitespace characters.
        let non_ws = match rest.iter().position(|&b| !is_ws(b)) {
            Some(p) => p,
            None => return Err(parse_error("vector assignment missing signal id")),
        };
        let signal_id_part = &rest[non_ws..];
        if signal_id_part.is_empty() {
            return Err(parse_error("vector assignment missing signal id"));
        }
        (value, signal_id_part)
    } else {
        // sscanf(buffer, "%1s%[^\n]", value, signal_id)
        // Our buffer begins with a non-whitespace expression character so
        // %1s reads exactly that single byte.
        if buffer.len() < 2 {
            return Err(parse_error("scalar assignment missing signal id"));
        }
        let value = &buffer[..1];
        let signal_id_part = &buffer[1..];
        (value, signal_id_part)
    };

    // Match the C: ignore signal ids longer than one character (still report success).
    if signal_id.len() > 1 {
        return Ok(());
    }

    let signal_id_str = std::str::from_utf8(signal_id)
        .map_err(|_| parse_error("invalid utf-8 in signal id"))?;
    let index = match get_signal_index(signal_id_str) {
        Some(i) => i,
        None => return Ok(()),
    };

    if index >= vcd.signals.len() {
        return Ok(());
    }

    let mut value_arr = [0u8; VCD_SIGNAL_SIZE];
    copy_into_fixed(value, &mut value_arr);

    vcd.signals[index].value_changes.push(ValueChange {
        timestamp: *timestamp,
        value: value_arr,
    });

    Ok(())
}
pub fn get_signal_index(s: &str) -> Option<usize> {
    let first = *s.as_bytes().first()?;
    let id = (first as i32) - (b'!' as i32);
    if id < 0 || id >= VCD_SIGNAL_COUNT as i32 {
        return None;
    }
    Some(id as usize)
}
