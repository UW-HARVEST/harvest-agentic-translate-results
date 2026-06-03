use std::fs::File;
use std::io::{Read, Seek, SeekFrom};

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
    fn new() -> Self {
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

    pub fn read_from_path(path: &str) -> Result<Self, std::io::Error> {
        let file = File::open(path)?;
        let mut vcd = VCD::new();
        let mut current_timestamp: Timestamp = 0;
        let mut state = State::BeforeModuleDefinitions;

        loop {
            let c = match read_byte(&file)? {
                Some(b) => b,
                None => break,
            };

            if c == b'$' {
                parse_instruction(&file, &mut vcd, &mut state)?;
            } else if c == b'#' {
                current_timestamp = parse_timestamp(&file)?;
            } else if isexpression(c as char) {
                unget_byte(&file)?;
                parse_assignment(&file, &mut vcd, &current_timestamp)?;
            } else if (c as char).is_ascii_whitespace() {
                continue;
            } else {
                return Err(std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!("Unexpected character: {}", c as char),
                ));
            }
        }

        Ok(vcd)
    }
    pub fn get_signal_by_name(&self, signal_name: &str) -> Option<&Signal> {
        for signal in &self.signals {
            let len = signal
                .name
                .iter()
                .position(|&b| b == 0)
                .unwrap_or(signal.name.len());
            if let Ok(name_str) = std::str::from_utf8(&signal.name[..len]) {
                if name_str == signal_name {
                    return Some(signal);
                }
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

fn read_byte(file: &File) -> std::io::Result<Option<u8>> {
    let mut f: &File = file;
    let mut buf = [0u8; 1];
    let n = Read::read(&mut f, &mut buf)?;
    if n == 0 {
        Ok(None)
    } else {
        Ok(Some(buf[0]))
    }
}

fn unget_byte(file: &File) -> std::io::Result<()> {
    let mut f: &File = file;
    Seek::seek(&mut f, SeekFrom::Current(-1))?;
    Ok(())
}

fn skip_whitespace(file: &File) -> std::io::Result<()> {
    while let Some(b) = read_byte(file)? {
        if !(b as char).is_ascii_whitespace() {
            unget_byte(file)?;
            return Ok(());
        }
    }
    Ok(())
}

fn read_word(file: &File) -> std::io::Result<String> {
    skip_whitespace(file)?;
    let mut s = Vec::new();
    while let Some(b) = read_byte(file)? {
        if (b as char).is_ascii_whitespace() {
            unget_byte(file)?;
            break;
        }
        s.push(b);
    }
    Ok(String::from_utf8_lossy(&s).into_owned())
}

fn read_digits(file: &File) -> std::io::Result<String> {
    let mut s = Vec::new();
    while let Some(b) = read_byte(file)? {
        if !(b as char).is_ascii_digit() {
            unget_byte(file)?;
            break;
        }
        s.push(b);
    }
    Ok(String::from_utf8_lossy(&s).into_owned())
}

fn read_until_dollar(file: &File) -> std::io::Result<()> {
    while let Some(b) = read_byte(file)? {
        if b == b'$' {
            unget_byte(file)?;
            return Ok(());
        }
    }
    Ok(())
}

fn read_until_any(file: &File, delims: &[u8]) -> std::io::Result<Vec<u8>> {
    let mut s = Vec::new();
    while let Some(b) = read_byte(file)? {
        if delims.contains(&b) {
            unget_byte(file)?;
            break;
        }
        s.push(b);
    }
    Ok(s)
}

fn read_line(file: &File) -> std::io::Result<()> {
    while let Some(b) = read_byte(file)? {
        if b == b'\n' {
            break;
        }
    }
    Ok(())
}

pub fn parse_instruction(
    file: &File,
    vcd: &mut VCD,
    state: &mut State,
) -> Result<(), std::io::Error> {
    let instruction = read_word(file)?;

    match instruction.as_str() {
        "end" | "dumpvars" | "dumpall" => Ok(()),
        "scope" => {
            *state = match *state {
                State::BeforeModuleDefinitions => State::InsideTopModule,
                State::InsideTopModule => State::InsideInnerModules,
                State::InsideInnerModules => State::InsideInnerModules,
            };
            read_until_dollar(file)?;
            Ok(())
        }
        "upscope" | "enddefinitions" | "comment" => {
            read_until_dollar(file)?;
            Ok(())
        }
        "var" => {
            if *state == State::InsideInnerModules {
                read_line(file)?;
                return Ok(());
            }
            let _var_type = read_word(file)?;
            let size_str = read_word(file)?;
            let size: usize = size_str.parse().unwrap_or(0);
            let _signal_id = read_word(file)?;
            let name = read_word(file)?;
            read_until_dollar(file)?;

            let mut signal_name = [0u8; VCD_NAME_SIZE];
            let name_bytes = name.as_bytes();
            let copy_len = name_bytes.len().min(VCD_NAME_SIZE - 1);
            signal_name[..copy_len].copy_from_slice(&name_bytes[..copy_len]);

            vcd.signals.push(Signal {
                name: signal_name,
                size,
                value_changes: Vec::new(),
            });
            Ok(())
        }
        "date" => {
            skip_whitespace(file)?;
            let bytes = read_until_any(file, &[b'\n', b'$'])?;
            let copy_len = bytes.len().min(VCD_DATE_SIZE - 1);
            vcd.date[..copy_len].copy_from_slice(&bytes[..copy_len]);
            Ok(())
        }
        "version" => {
            skip_whitespace(file)?;
            let bytes = read_until_any(file, &[b'\n', b'$'])?;
            let copy_len = bytes.len().min(VCD_VERSION_SIZE - 1);
            vcd.version[..copy_len].copy_from_slice(&bytes[..copy_len]);
            Ok(())
        }
        "timescale" => {
            skip_whitespace(file)?;
            let scale_str = read_digits(file)?;
            vcd.timescale.scale = scale_str.parse().unwrap_or(0);
            let bytes = read_until_any(file, &[b'\n', b'$'])?;
            let copy_len = bytes.len().min(VCD_TIME_UNIT_SIZE - 1);
            vcd.timescale.unit[..copy_len].copy_from_slice(&bytes[..copy_len]);
            Ok(())
        }
        _ => Ok(()),
    }
}
pub fn parse_timestamp(file: &File) -> Result<Timestamp, std::io::Error> {
    skip_whitespace(file)?;
    let s = read_digits(file)?;
    Ok(s.parse().unwrap_or(0))
}
pub fn parse_assignment(
    file: &File,
    vcd: &mut VCD,
    timestamp: &Timestamp,
) -> Result<(), std::io::Error> {
    let mut buffer = Vec::new();
    while let Some(b) = read_byte(file)? {
        if b == b'\n' {
            unget_byte(file)?;
            break;
        }
        buffer.push(b);
    }

    if buffer.is_empty() {
        return Ok(());
    }

    let is_vector = !b"01xXzZ".contains(&buffer[0]);

    let (value, signal_id): (Vec<u8>, Vec<u8>) = if is_vector {
        let space_pos = buffer.iter().position(|&b| b == b' ');
        match space_pos {
            Some(pos) => {
                let value = buffer[..pos].to_vec();
                let mut start = pos + 1;
                while start < buffer.len() && buffer[start] == b' ' {
                    start += 1;
                }
                let signal_id = buffer[start..].to_vec();
                (value, signal_id)
            }
            None => return Ok(()),
        }
    } else {
        let value = vec![buffer[0]];
        let signal_id = buffer[1..].to_vec();
        (value, signal_id)
    };

    if signal_id.len() > 1 || signal_id.is_empty() {
        return Ok(());
    }

    let id_str = std::str::from_utf8(&signal_id).unwrap_or("");
    let index = match get_signal_index(id_str) {
        Some(i) => i,
        None => return Ok(()),
    };

    if index >= vcd.signals.len() {
        return Ok(());
    }

    let mut value_arr = [0u8; VCD_SIGNAL_SIZE];
    let copy_len = value.len().min(VCD_SIGNAL_SIZE);
    value_arr[..copy_len].copy_from_slice(&value[..copy_len]);

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
