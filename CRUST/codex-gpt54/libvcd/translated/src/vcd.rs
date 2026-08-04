use std::fs::File;
use std::io::{self, ErrorKind, Read, Seek, SeekFrom};
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
            signals: Vec::with_capacity(VCD_SIGNAL_COUNT),
            date: [0; VCD_DATE_SIZE],
            version: [0; VCD_VERSION_SIZE],
            timescale: Timescale {
                unit: [0; VCD_TIME_UNIT_SIZE],
                scale: 0,
            },
        };
        let mut current_timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        while let Some(byte) = read_byte(&file)? {
            let ch = byte as char;
            if ch == '$' {
                parse_instruction(&file, &mut vcd, &mut state)?;
            } else if ch == '#' {
                current_timestamp = parse_timestamp(&file)?;
            } else if isexpression(ch) {
                unread_byte(&file)?;
                parse_assignment(&file, &mut vcd, &current_timestamp)?;
            } else if ch.is_ascii_whitespace() {
                continue;
            } else {
                return Err(io::Error::new(
                    ErrorKind::InvalidData,
                    format!("unexpected character in VCD: {ch:?}"),
                ));
            }
        }

        Ok(vcd)
    }
    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        let requested = signal_name.trim();
        let requested_base = base_signal_name(requested);

        self.signals.iter().find(|signal| {
            let name = read_c_string(&signal.name);
            name == requested || base_signal_name(name) == requested || name == requested_base
        })
    }
}
impl Signal {
    pub fn get_value_at_timestamp(&self, timestamp: Timestamp) -> Option<&[u8; VCD_SIGNAL_SIZE]> {
        let mut previous_value = None;
        for value_change in &self.value_changes {
            if timestamp < value_change.timestamp {
                break;
            }
            previous_value = Some(&value_change.value);
        }
        previous_value
    }
}
pub const BUFFER_LENGTH: usize = 512;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum State {
    BeforeModuleDefinitions,
    InsideTopModule,
    InsideInnerModules,
}
#[allow(private_interfaces)]
pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let instruction = read_token(file)?.ok_or_else(|| {
        io::Error::new(ErrorKind::UnexpectedEof, "expected VCD instruction after '$'")
    })?;

    match instruction.as_str() {
        "end" | "dumpvars" | "dumpall" => Ok(()),
        "scope" => {
            match *state {
                State::BeforeModuleDefinitions => *state = State::InsideTopModule,
                State::InsideTopModule => *state = State::InsideInnerModules,
                State::InsideInnerModules => {}
            }
            skip_until_dollar(file)?;
            Ok(())
        }
        "upscope" | "enddefinitions" | "comment" => {
            skip_until_dollar(file)?;
            Ok(())
        }
        "var" => {
            if *state == State::InsideInnerModules {
                skip_to_line_end(file)?;
                return Ok(());
            }

            let _var_type = read_token(file)?.ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidData, "missing VCD var type")
            })?;
            let size = read_token(file)?
                .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing VCD signal size"))?
                .parse::<usize>()
                .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid VCD signal size"))?;
            let signal_id = read_token(file)?.ok_or_else(|| {
                io::Error::new(ErrorKind::InvalidData, "missing VCD signal id")
            })?;
            let signal_name_raw = read_until_dollar(file)?;
            let signal_name = signal_name_raw.trim().trim_end_matches("$end").trim();

            if let Some(index) = get_signal_index(&signal_id) {
                if index < vcd.signals.len() && vcd.signals[index].size != 0 {
                    return Ok(());
                }
            }

            if vcd.signals.len() < VCD_SIGNAL_COUNT {
                let mut signal = Signal {
                    name: [0; VCD_NAME_SIZE],
                    size,
                    value_changes: Vec::with_capacity(VCD_VALUE_CHANGE_COUNT),
                };
                write_c_string(&mut signal.name, signal_name);
                vcd.signals.push(signal);
            }

            Ok(())
        }
        "date" => {
            let date = read_metadata_value(file)?;
            write_c_string(&mut vcd.date, &date);
            Ok(())
        }
        "version" => {
            let version = read_metadata_value(file)?;
            write_c_string(&mut vcd.version, &version);
            Ok(())
        }
        "timescale" => {
            let timescale = read_metadata_value(file)?;
            let split_at = timescale
                .find(|c: char| !c.is_ascii_digit())
                .unwrap_or(timescale.len());
            let (scale, unit) = timescale.split_at(split_at);
            vcd.timescale.scale = scale
                .trim()
                .parse::<usize>()
                .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid VCD timescale"))?;
            write_c_string(&mut vcd.timescale.unit, unit.trim());
            Ok(())
        }
        _ => Err(io::Error::new(
            ErrorKind::InvalidData,
            format!("unsupported VCD instruction: {instruction}"),
        )),
    }
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let timestamp = read_token(file)?.ok_or_else(|| {
        io::Error::new(ErrorKind::UnexpectedEof, "missing VCD timestamp")
    })?;
    timestamp
        .parse::<Timestamp>()
        .map_err(|_| io::Error::new(ErrorKind::InvalidData, "invalid VCD timestamp"))
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let buffer = read_until_line_end(file)?;
    let line = buffer.trim();
    if line.is_empty() {
        return Ok(());
    }

    let (value, signal_id) = if matches!(line.as_bytes()[0], b'0' | b'1' | b'x' | b'X' | b'z' | b'Z')
    {
        (&line[..1], line[1..].trim())
    } else {
        let mut parts = line.split_whitespace();
        let value = parts
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing VCD assignment value"))?;
        let signal_id = parts
            .next()
            .ok_or_else(|| io::Error::new(ErrorKind::InvalidData, "missing VCD assignment signal id"))?;
        (value, signal_id)
    };

    if signal_id.len() != 1 {
        return Ok(());
    }

    let Some(index) = get_signal_index(signal_id) else {
        return Ok(());
    };
    if index >= vcd.signals.len() {
        return Ok(());
    }

    let signal = &mut vcd.signals[index];
    if signal.value_changes.len() >= VCD_VALUE_CHANGE_COUNT {
        return Ok(());
    }

    let mut value_change = ValueChange {
        timestamp: *timestamp,
        value: [0; VCD_SIGNAL_SIZE],
    };
    write_c_string(&mut value_change.value, normalize_value(value));
    signal.value_changes.push(value_change);
    Ok(())
}
pub fn get_signal_index(s: &str) -> Option<usize> {
    let first = s.as_bytes().first().copied()?;
    let index = first.checked_sub(b'!')? as usize;
    (index < VCD_SIGNAL_COUNT).then_some(index)
}

fn write_c_string<const N: usize>(dst: &mut [u8; N], src: &str) {
    dst.fill(0);
    let bytes = src.as_bytes();
    let len = bytes.len().min(N);
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn read_c_string(bytes: &[u8]) -> &str {
    let len = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..len]).unwrap_or("")
}

fn base_signal_name(name: &str) -> &str {
    name.split_once(" [").map_or(name, |(base, _)| base)
}

fn normalize_value(value: &str) -> &str {
    if value.len() > 1 && matches!(value.as_bytes()[0], b'b' | b'B') {
        &value[1..]
    } else {
        value
    }
}

fn read_byte(file: &File) -> io::Result<Option<u8>> {
    let mut file = file;
    let mut buf = [0_u8; 1];
    match file.read(&mut buf)? {
        0 => Ok(None),
        _ => Ok(Some(buf[0])),
    }
}

fn unread_byte(file: &File) -> io::Result<()> {
    let mut file = file;
    file.seek(SeekFrom::Current(-1))?;
    Ok(())
}

fn read_token(file: &File) -> io::Result<Option<String>> {
    skip_ascii_whitespace(file)?;

    let mut token = String::new();
    while let Some(byte) = read_byte(file)? {
        let ch = byte as char;
        if ch.is_ascii_whitespace() {
            break;
        }
        token.push(ch);
    }

    if token.is_empty() {
        Ok(None)
    } else {
        Ok(Some(token))
    }
}

fn skip_ascii_whitespace(file: &File) -> io::Result<()> {
    while let Some(byte) = read_byte(file)? {
        if !(byte as char).is_ascii_whitespace() {
            unread_byte(file)?;
            break;
        }
    }
    Ok(())
}

fn skip_until_dollar(file: &File) -> io::Result<()> {
    while let Some(byte) = read_byte(file)? {
        if byte == b'$' {
            unread_byte(file)?;
            break;
        }
    }
    Ok(())
}

fn skip_to_line_end(file: &File) -> io::Result<()> {
    while let Some(byte) = read_byte(file)? {
        if byte == b'\n' {
            break;
        }
    }
    Ok(())
}

fn read_until_dollar(file: &File) -> io::Result<String> {
    let mut out = String::new();
    while let Some(byte) = read_byte(file)? {
        if byte == b'$' {
            unread_byte(file)?;
            break;
        }
        out.push(byte as char);
    }
    Ok(out)
}

fn read_until_line_end(file: &File) -> io::Result<String> {
    let mut out = String::new();
    while let Some(byte) = read_byte(file)? {
        if byte == b'\n' {
            break;
        }
        out.push(byte as char);
    }
    Ok(out)
}

fn read_metadata_value(file: &File) -> io::Result<String> {
    skip_ascii_whitespace(file)?;
    let mut out = String::new();
    while let Some(byte) = read_byte(file)? {
        if matches!(byte, b'$' | b'\n' | b'\r') {
            if byte == b'$' {
                unread_byte(file)?;
            }
            break;
        }
        out.push(byte as char);
    }
    Ok(out.trim().to_string())
}
