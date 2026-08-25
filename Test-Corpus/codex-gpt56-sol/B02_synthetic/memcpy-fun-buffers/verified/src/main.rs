use std::io::{self, Read, Write};
use std::process::ExitCode;

const MAX_BUFFER_LENGTH: usize = 256;

#[derive(Clone)]
struct Buffer {
    data: [u8; MAX_BUFFER_LENGTH],
    length: usize,
    checksum: u32,
}

impl Default for Buffer {
    fn default() -> Self {
        Self {
            data: [0; MAX_BUFFER_LENGTH],
            length: 0,
            checksum: 0,
        }
    }
}

struct Scanner {
    input: Vec<u8>,
    position: usize,
}

impl Scanner {
    fn from_stdin() -> Self {
        let mut input = Vec::new();
        let _ = io::stdin().read_to_end(&mut input);
        Self { input, position: 0 }
    }

    // Match scanf("%d"): skip C whitespace, accept a sign, and stop at the
    // first non-decimal byte without requiring the rest of a token to be valid.
    fn read_i32(&mut self) -> Option<i32> {
        while self
            .input
            .get(self.position)
            .is_some_and(|byte| is_c_whitespace(*byte))
        {
            self.position += 1;
        }

        let start = self.position;
        let negative = match self.input.get(self.position) {
            Some(b'-') => {
                self.position += 1;
                true
            }
            Some(b'+') => {
                self.position += 1;
                false
            }
            _ => false,
        };

        let digits_start = self.position;
        let mut magnitude = 0_u64;
        let mut overflowed = false;
        while let Some(byte @ b'0'..=b'9') = self.input.get(self.position) {
            if !overflowed {
                if let Some(value) = magnitude
                    .checked_mul(10)
                    .and_then(|value| value.checked_add(u64::from(*byte - b'0')))
                {
                    magnitude = value;
                } else {
                    overflowed = true;
                }
            }
            self.position += 1;
        }

        if self.position == digits_start {
            self.position = start;
            return None;
        }

        let value = if negative {
            if overflowed || magnitude >= (1_u64 << 63) {
                i64::MIN
            } else {
                -(magnitude as i64)
            }
        } else if overflowed || magnitude > i64::MAX as u64 {
            i64::MAX
        } else {
            magnitude as i64
        };
        Some(value as i32)
    }
}

fn is_c_whitespace(byte: u8) -> bool {
    matches!(byte, b' ' | b'\t' | b'\n' | 0x0b | 0x0c | b'\r')
}

fn calculate_checksum(data: &[u8]) -> u32 {
    data.iter()
        .fold(0_u32, |sum, byte| sum.wrapping_shl(3) ^ u32::from(*byte))
}

fn validate_buffer(buffer: &Buffer) -> bool {
    if buffer.length > MAX_BUFFER_LENGTH {
        eprintln!("Error: Buffer length {} exceeds maximum 256", buffer.length);
        return false;
    }

    let expected = calculate_checksum(&buffer.data[..buffer.length]);
    if buffer.checksum != expected {
        eprintln!(
            "Warning: Checksum mismatch. Expected {}, got {}",
            expected, buffer.checksum
        );
    }
    true
}

fn buffer_copy(source: &Buffer, destination: &mut Buffer) -> i32 {
    if !validate_buffer(source) {
        return -1;
    }

    destination.data[..source.length].copy_from_slice(&source.data[..source.length]);
    destination.length = source.length;
    destination.checksum = calculate_checksum(&destination.data[..destination.length]);
    0
}

fn buffer_reverse(buffer: &mut Buffer) -> i32 {
    if buffer.length == 0 {
        return 0;
    }

    buffer.data[..buffer.length].reverse();
    buffer.checksum = calculate_checksum(&buffer.data[..buffer.length]);
    0
}

fn buffer_merge(source1: &Buffer, source2: &Buffer, destination: &mut Buffer) -> i32 {
    if source1.length + source2.length > MAX_BUFFER_LENGTH {
        eprintln!(
            "Error: Merged length {} exceeds maximum",
            source1.length + source2.length
        );
        return -1;
    }

    destination.data[..source1.length].copy_from_slice(&source1.data[..source1.length]);
    destination.data[source1.length..source1.length + source2.length]
        .copy_from_slice(&source2.data[..source2.length]);
    destination.length = source1.length + source2.length;
    destination.checksum = calculate_checksum(&destination.data[..destination.length]);
    0
}

fn buffer_split(
    source: &Buffer,
    split_position: usize,
    destination1: &mut Buffer,
    destination2: &mut Buffer,
) -> i32 {
    if split_position > source.length {
        eprintln!(
            "Error: Split position {} exceeds length {}",
            split_position, source.length
        );
        return -1;
    }

    if split_position > 0 {
        destination1.data[..split_position].copy_from_slice(&source.data[..split_position]);
    }
    destination1.length = split_position;
    destination1.checksum = calculate_checksum(&destination1.data[..destination1.length]);

    let remaining = source.length - split_position;
    if remaining > 0 {
        destination2.data[..remaining].copy_from_slice(&source.data[split_position..source.length]);
    }
    destination2.length = remaining;
    destination2.checksum = calculate_checksum(&destination2.data[..destination2.length]);
    0
}

fn buffer_interleave(source1: &Buffer, source2: &Buffer, destination: &mut Buffer) -> i32 {
    let max_length = source1.length.max(source2.length);
    if source1.length + source2.length > MAX_BUFFER_LENGTH {
        eprintln!("Error: Interleaved length exceeds maximum");
        return -1;
    }

    let mut destination_position = 0;
    for index in 0..max_length {
        if index < source1.length {
            destination.data[destination_position] = source1.data[index];
            destination_position += 1;
        }
        if index < source2.length {
            destination.data[destination_position] = source2.data[index];
            destination_position += 1;
        }
    }

    destination.length = destination_position;
    destination.checksum = calculate_checksum(&destination.data[..destination.length]);
    0
}

fn buffer_rotate(buffer: &mut Buffer, mut positions: i32) -> i32 {
    if buffer.length == 0 || positions == 0 {
        return 0;
    }

    positions %= buffer.length as i32;
    if positions < 0 {
        positions += buffer.length as i32;
    }

    buffer.data[..buffer.length].rotate_left(positions as usize);
    buffer.checksum = calculate_checksum(&buffer.data[..buffer.length]);
    0
}

fn read_buffer(scanner: &mut Scanner, buffer: &mut Buffer) -> i32 {
    let Some(length) = scanner.read_i32() else {
        eprintln!("Error: Failed to read buffer length");
        return -1;
    };

    if !(0..=MAX_BUFFER_LENGTH as i32).contains(&length) {
        eprintln!("Error: Invalid buffer length {}", length);
        return -1;
    }

    buffer.length = length as usize;
    for index in 0..buffer.length {
        let Some(byte) = scanner.read_i32() else {
            eprintln!("Error: Failed to read byte {}", index);
            return -1;
        };
        buffer.data[index] = byte as u8;
    }

    buffer.checksum = calculate_checksum(&buffer.data[..buffer.length]);
    0
}

fn write_buffer(output: &mut impl Write, buffer: &Buffer) {
    let _ = write!(output, "{}", buffer.length);
    for byte in &buffer.data[..buffer.length] {
        let _ = write!(output, " {}", byte);
    }
    let _ = writeln!(output);
}

fn run() -> i32 {
    let mut scanner = Scanner::from_stdin();
    let stdout = io::stdout();
    let mut output = io::BufWriter::new(stdout.lock());

    let Some(operation) = scanner.read_i32() else {
        eprintln!("Error: Failed to read operation");
        return 1;
    };

    let Some(buffer_count) = scanner.read_i32() else {
        eprintln!("Error: Failed to read buffer count");
        return 1;
    };

    if buffer_count <= 0 || buffer_count > 100 {
        eprintln!("Error: Invalid buffer count {}", buffer_count);
        return 1;
    }

    let mut buffers = vec![Buffer::default(); buffer_count as usize];
    for buffer in &mut buffers {
        if read_buffer(&mut scanner, buffer) != 0 {
            return 1;
        }
    }

    let mut result = 0;
    match operation {
        0 => {
            if buffer_count >= 2 {
                let mut temporary = Buffer::default();
                result = buffer_copy(&buffers[0], &mut temporary);
                if result == 0 {
                    write_buffer(&mut output, &temporary);
                }
            } else {
                eprintln!("Error: Copy needs at least 2 buffers");
                result = -1;
            }
        }
        1 => {
            for buffer in &mut buffers {
                result = buffer_reverse(buffer);
                if result != 0 {
                    break;
                }
                write_buffer(&mut output, buffer);
            }
        }
        2 => {
            if buffer_count >= 2 {
                let mut merged = Buffer::default();
                result = buffer_merge(&buffers[0], &buffers[1], &mut merged);
                if result == 0 {
                    write_buffer(&mut output, &merged);
                }
            } else {
                eprintln!("Error: Merge needs at least 2 buffers");
                result = -1;
            }
        }
        3 => {
            if buffer_count >= 1 {
                if let Some(split_position) = scanner.read_i32() {
                    let mut part1 = Buffer::default();
                    let mut part2 = Buffer::default();
                    result =
                        buffer_split(&buffers[0], split_position as usize, &mut part1, &mut part2);
                    if result == 0 {
                        write_buffer(&mut output, &part1);
                        write_buffer(&mut output, &part2);
                    }
                } else {
                    eprintln!("Error: Failed to read split position");
                    result = -1;
                }
            }
        }
        4 => {
            if buffer_count >= 2 {
                let mut interleaved = Buffer::default();
                result = buffer_interleave(&buffers[0], &buffers[1], &mut interleaved);
                if result == 0 {
                    write_buffer(&mut output, &interleaved);
                }
            } else {
                eprintln!("Error: Interleave needs at least 2 buffers");
                result = -1;
            }
        }
        5 => {
            if let Some(positions) = scanner.read_i32() {
                for buffer in &mut buffers {
                    result = buffer_rotate(buffer, positions);
                    if result != 0 {
                        break;
                    }
                    write_buffer(&mut output, buffer);
                }
            } else {
                eprintln!("Error: Failed to read rotation amount");
                result = -1;
            }
        }
        6 => {
            for buffer in &buffers {
                let _ = writeln!(output, "{}", buffer.checksum);
            }
        }
        _ => {
            eprintln!("Error: Unknown operation {}", operation);
            result = -1;
        }
    }

    let _ = output.flush();
    if result != 0 {
        1
    } else {
        0
    }
}

fn main() -> ExitCode {
    ExitCode::from(run() as u8)
}
