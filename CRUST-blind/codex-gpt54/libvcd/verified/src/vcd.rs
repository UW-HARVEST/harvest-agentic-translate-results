use std::fs::File;
use std::io::{self, Error, ErrorKind, Read, Seek, SeekFrom};
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
        let mut file = File::open(path)?;
        let mut vcd = new_vcd();
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;
        let mut byte = [0_u8; 1];

        loop {
            match file.read(&mut byte)? {
                0 => return Ok(vcd),
                _ => {
                    let character = byte[0] as char;
                    if character == '$' {
                        if parse_instruction(&file, &mut vcd, &mut state).is_ok() {
                            continue;
                        }
                    } else if character == '#' {
                        if let Ok(timestamp) = parse_timestamp(&file) {
                            current_timestamp = timestamp;
                            continue;
                        }
                    } else if isexpression(character) {
                        file.seek(SeekFrom::Current(-1))?;
                        if parse_assignment(&file, &mut vcd, &current_timestamp).is_ok() {
                            continue;
                        }
                    } else if character.is_whitespace() {
                        continue;
                    }

                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        "failed to parse VCD input",
                    ));
                }
            }
        }
    }
    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        self.signals
            .iter()
            .find(|signal| fixed_bytes_to_str(&signal.name) == signal_name)
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
    let mut file = file.try_clone()?;
    let instruction =
        read_token(&mut file)?.ok_or_else(|| Error::new(ErrorKind::UnexpectedEof, "missing instruction"))?;

    if matches!(instruction.as_str(), "end" | "dumpvars" | "dumpall") {
        return Ok(());
    }

    if instruction == "scope" {
        match *state {
            State::BeforeModuleDefinitions => *state = State::InsideTopModule,
            State::InsideTopModule => *state = State::InsideInnerModules,
            State::InsideInnerModules => {}
        }
        skip_until_dollar(&mut file)?;
        return Ok(());
    }

    if matches!(
        instruction.as_str(),
        "upscope" | "enddefinitions" | "comment"
    ) {
        skip_until_dollar(&mut file)?;
        return Ok(());
    }

    if instruction == "var" {
        if *state == State::InsideInnerModules {
            skip_until_newline(&mut file, true)?;
            return Ok(());
        }

        let mut signal = Signal {
            name: [0; VCD_NAME_SIZE],
            size: 0,
            value_changes: Vec::new(),
        };

        let _signal_type =
            read_token(&mut file)?.ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing signal type"))?;
        let size_token =
            read_token(&mut file)?.ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing signal size"))?;
        signal.size = size_token
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid signal size"))?;
        let signal_id =
            read_token(&mut file)?.ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing signal id"))?;
        let signal_name = read_until_space_or_dollar(&mut file)?;
        write_c_string(&mut signal.name, signal_name.trim_start());
        skip_until_dollar(&mut file)?;

        if let Some(index) = get_signal_index(&signal_id) {
            vcd.signals.push(signal);
            if vcd.signals.get(index).map(|signal| signal.size != 0).unwrap_or(false) {
                return Ok(());
            }
            return Ok(());
        }

        vcd.signals.push(signal);
        return Ok(());
    }

    if instruction == "date" {
        let date = read_value_line(&mut file)?;
        write_c_string(&mut vcd.date, &date);
        return Ok(());
    }

    if instruction == "version" {
        let version = read_value_line(&mut file)?;
        write_c_string(&mut vcd.version, &version);
        return Ok(());
    }

    if instruction == "timescale" {
        skip_leading_whitespace(&mut file)?;
        let scale_token =
            read_token(&mut file)?.ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing timescale scale"))?;
        vcd.timescale.scale = scale_token
            .parse::<usize>()
            .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid timescale scale"))?;
        let unit = read_until_dollar_or_newline(&mut file)?;
        write_c_string(&mut vcd.timescale.unit, &unit);
        return Ok(());
    }

    Err(Error::new(
        ErrorKind::InvalidData,
        "unknown VCD instruction",
    ))
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    let mut file = file.try_clone()?;
    skip_leading_whitespace(&mut file)?;
    let token =
        read_digits(&mut file)?.ok_or_else(|| Error::new(ErrorKind::InvalidData, "missing timestamp"))?;
    token
        .parse::<Timestamp>()
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid timestamp"))
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let mut file = file.try_clone()?;
    let buffer = skip_until_newline(&mut file, false)?;
    if buffer.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, "empty assignment"));
    }

    let is_vector = !matches!(buffer.as_bytes()[0] as char, '0' | '1' | 'x' | 'X' | 'z' | 'Z');
    let (value, signal_id) = if is_vector {
        let mut parts = buffer.splitn(2, ' ');
        let value = parts.next().unwrap_or_default();
        let signal_id = parts.next().unwrap_or_default().trim_start();
        if value.is_empty() || signal_id.is_empty() {
            return Err(Error::new(ErrorKind::InvalidData, "invalid vector assignment"));
        }
        (value, signal_id)
    } else {
        if buffer.len() < 2 {
            return Err(Error::new(ErrorKind::InvalidData, "invalid scalar assignment"));
        }
        (&buffer[..1], &buffer[1..])
    };

    if signal_id.len() > 1 {
        return Ok(());
    }

    let Some(index) = get_signal_index(signal_id) else {
        return Ok(());
    };
    if index >= vcd.signals.len() {
        return Ok(());
    }

    if vcd.signals[index].value_changes.len() < VCD_VALUE_CHANGE_COUNT {
        let mut stored_value = [0_u8; VCD_SIGNAL_SIZE];
        write_c_string(&mut stored_value, value);
        vcd.signals[index].value_changes.push(ValueChange {
            timestamp: *timestamp,
            value: stored_value,
        });
    }

    Ok(())
}
pub fn get_signal_index(s: &str) -> Option<usize> {
    let byte = *s.as_bytes().first()?;
    let id = byte as i16 - b'!' as i16;
    if !(0..VCD_SIGNAL_COUNT as i16).contains(&id) {
        return None;
    }
    Some(id as usize)
}

fn new_vcd() -> VCD {
    VCD {
        signals: Vec::new(),
        date: [0; VCD_DATE_SIZE],
        version: [0; VCD_VERSION_SIZE],
        timescale: Timescale {
            unit: [0; VCD_TIME_UNIT_SIZE],
            scale: 0,
        },
    }
}

fn fixed_bytes_to_str(bytes: &[u8]) -> &str {
    let len = bytes.iter().position(|&byte| byte == 0).unwrap_or(bytes.len());
    std::str::from_utf8(&bytes[..len]).unwrap_or("")
}

fn write_c_string<const N: usize>(dst: &mut [u8; N], src: &str) {
    dst.fill(0);
    let bytes = src.as_bytes();
    let len = bytes.len().min(N.saturating_sub(1));
    dst[..len].copy_from_slice(&bytes[..len]);
}

fn read_byte(file: &mut File) -> io::Result<Option<u8>> {
    let mut byte = [0_u8; 1];
    match file.read(&mut byte)? {
        0 => Ok(None),
        _ => Ok(Some(byte[0])),
    }
}

fn unread_byte(file: &mut File) -> io::Result<()> {
    file.seek(SeekFrom::Current(-1))?;
    Ok(())
}

fn skip_leading_whitespace(file: &mut File) -> io::Result<()> {
    while let Some(byte) = read_byte(file)? {
        if !(byte as char).is_whitespace() {
            unread_byte(file)?;
            break;
        }
    }
    Ok(())
}

fn read_token(file: &mut File) -> io::Result<Option<String>> {
    skip_leading_whitespace(file)?;
    let mut bytes = Vec::new();

    while let Some(byte) = read_byte(file)? {
        if (byte as char).is_whitespace() {
            unread_byte(file)?;
            break;
        }
        bytes.push(byte);
    }

    if bytes.is_empty() {
        return Ok(None);
    }

    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid utf-8 token"))
}

fn read_digits(file: &mut File) -> io::Result<Option<String>> {
    let mut bytes = Vec::new();

    while let Some(byte) = read_byte(file)? {
        if !(byte as char).is_ascii_digit() {
            unread_byte(file)?;
            break;
        }
        bytes.push(byte);
    }

    if bytes.is_empty() {
        return Ok(None);
    }

    String::from_utf8(bytes)
        .map(Some)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid digit sequence"))
}

fn skip_until_dollar(file: &mut File) -> io::Result<()> {
    while let Some(byte) = read_byte(file)? {
        if byte == b'$' {
            unread_byte(file)?;
            break;
        }
    }
    Ok(())
}

fn skip_until_newline(file: &mut File, consume_newline: bool) -> io::Result<String> {
    let mut bytes = Vec::new();

    while let Some(byte) = read_byte(file)? {
        if byte == b'\n' {
            if !consume_newline {
                unread_byte(file)?;
            }
            break;
        }
        bytes.push(byte);
    }

    String::from_utf8(bytes)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid utf-8 line"))
}

fn read_value_line(file: &mut File) -> io::Result<String> {
    skip_leading_whitespace(file)?;
    read_until_dollar_or_newline(file)
}

fn read_until_dollar_or_newline(file: &mut File) -> io::Result<String> {
    let mut bytes = Vec::new();

    while let Some(byte) = read_byte(file)? {
        if matches!(byte, b'$' | b'\n') {
            unread_byte(file)?;
            break;
        }
        bytes.push(byte);
    }

    String::from_utf8(bytes)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid utf-8 value"))
}

fn read_until_space_or_dollar(file: &mut File) -> io::Result<String> {
    skip_leading_whitespace(file)?;
    let mut bytes = Vec::new();

    while let Some(byte) = read_byte(file)? {
        if matches!(byte, b' ' | b'$') {
            unread_byte(file)?;
            break;
        }
        bytes.push(byte);
    }

    String::from_utf8(bytes)
        .map_err(|_| Error::new(ErrorKind::InvalidData, "invalid utf-8 field"))
}
