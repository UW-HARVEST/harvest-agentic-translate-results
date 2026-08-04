//! Base122 encoder/decoder
//!
//! Base122 is a binary-to-text encoding scheme that avoids certain illegal characters
//! such as null, newline, carriage return, double quote, ampersand, and backslash.
use std::error::Error;
use std::fmt;
/// Error type for Base122 operations
#[derive(Debug, Clone)]
pub struct Base122Error {
    pub message: String,
}
impl Base122Error {
    fn new(message: &str) -> Self {
        Self {
            message: message.to_string(),
        }
    }
}
impl fmt::Display for Base122Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
impl Error for Base122Error {}
/// BitReader is used to read bits from a byte array
pub struct BitReader<'a> {
    pub input: &'a [u8],
    pub byte_pos: usize,
    pub bit_pos: usize,
}
impl<'a> BitReader<'a> {
    pub fn new(input: &'a [u8]) -> Self {
        Self {
            input,
            byte_pos: 0,
            bit_pos: 0,
        }
    }
    /// Read up to `nbits` from the input, returns (bits_read, value)
    pub fn read(&mut self, nbits: u8) -> (u8, u8) {
        assert!(nbits >= 1 && nbits <= 8);

        let bit_len = self.input.len() * 8;
        let cur_bit = self.byte_pos * 8 + self.bit_pos;
        let max_nbits = bit_len.saturating_sub(cur_bit);
        let nbits_actual = (nbits as usize).min(max_nbits);

        if nbits_actual == 0 {
            return (0, 0);
        }

        let first_byte_index = cur_bit / 8;
        let first_byte_cur_bit = cur_bit % 8;

        // Read up to two bytes into a u32 to avoid u16 shift edge cases.
        let mut two_bytes: u32 = self.input[first_byte_index] as u32;
        two_bytes <<= 8;
        if first_byte_index + 1 < self.input.len() {
            two_bytes |= self.input[first_byte_index + 1] as u32;
        }

        let shift = (8 - first_byte_cur_bit) + (8 - nbits_actual);
        two_bytes >>= shift;

        let mask: u32 = if nbits_actual >= 8 {
            0xFF
        } else {
            (1u32 << nbits_actual) - 1
        };
        let out = (two_bytes & mask) as u8;

        let new_cur_bit = cur_bit + nbits_actual;
        self.byte_pos = new_cur_bit / 8;
        self.bit_pos = new_cur_bit % 8;

        (nbits_actual as u8, out)
    }
}
/// BitWriter is used to write bits to a byte array
pub struct BitWriter<'a> {
    pub output: Option<&'a mut [u8]>,
    pub len: usize,
    pub cur_bit: usize,
    pub count_only: bool,
}
impl<'a> BitWriter<'a> {
    pub fn new(output: Option<&'a mut [u8]>, len: usize) -> Self {
        let count_only = output.is_none();
        Self {
            output,
            len,
            cur_bit: 0,
            count_only,
        }
    }
    /// Write `nbits` bits from `value` to the output
    /// Returns the number of bytes used so far or an error
    pub fn write(&mut self, nbits: u8, value: u8) -> Result<usize, Base122Error> {
        assert!(nbits >= 1 && nbits <= 8);
        let nbits_usize = nbits as usize;

        if self.count_only {
            self.cur_bit += nbits_usize;
            return Ok(self.cur_bit.div_ceil(8));
        }

        let bit_len = self.len * 8;
        if self.cur_bit + nbits_usize > bit_len {
            return Err(Base122Error::new("output does not have sufficient size"));
        }

        let first_byte_index = self.cur_bit / 8;
        let first_byte_cur_bit = self.cur_bit % 8;

        let buf = self
            .output
            .as_deref_mut()
            .expect("output should be set when not count_only");

        // Mask of bits to preserve in the current byte (bits before first_byte_cur_bit).
        let preserve_mask: u8 = (!(0xFFu32 >> first_byte_cur_bit)) as u8;
        let mut two_bytes: u32 = (buf[first_byte_index] & preserve_mask) as u32;
        two_bytes <<= 8;

        let value_mask: u32 = if nbits >= 8 {
            0xFF
        } else {
            (1u32 << nbits_usize) - 1
        };
        let shift_amount = 8 + (8 - first_byte_cur_bit) - nbits_usize;
        two_bytes |= ((value as u32) & value_mask) << shift_amount;

        buf[first_byte_index] = (two_bytes >> 8) as u8;
        if first_byte_cur_bit + nbits_usize > 8 {
            buf[first_byte_index + 1] = two_bytes as u8;
        }
        self.cur_bit += nbits_usize;
        Ok(self.cur_bit.div_ceil(8))
    }
}
const ILLEGALS: [u8; 6] = [
    0,  // null
    10, // newline
    13, // carriage return
    34, // double quote
    38, // ampersand
    92, // backslash
];
/// Check if a byte value is one of the illegal characters
fn is_illegal(val: u8) -> bool {
    ILLEGALS.iter().any(|&x| x == val)
}
/// Get the index of an illegal character in the ILLEGALS array
fn get_illegal_index(val: u8) -> u8 {
    for (i, &x) in ILLEGALS.iter().enumerate() {
        if x == val {
            return i as u8;
        }
    }
    panic!("unreachable: get_illegal_index called with non-illegal value");
}

/// Helper that writes a single byte to the output, or just counts when in count-only mode.
fn output_byte(
    output: &mut Option<&mut [u8]>,
    out_index: &mut usize,
    out_written: &mut usize,
    b: u8,
) -> Result<(), Base122Error> {
    match output {
        None => {
            *out_written += 1;
        }
        Some(buf) => {
            if *out_index == buf.len() {
                return Err(Base122Error::new("output does not have sufficient size"));
            }
            buf[*out_index] = b;
            *out_written += 1;
            *out_index += 1;
        }
    }
    Ok(())
}

/// Encode binary data to Base122 encoding
///
/// # Arguments
///
/// * `input` - The binary data to encode
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - The encoded data
/// * `Err(Base122Error)` - If there was an error during encoding
pub fn encode(input: &[u8]) -> Result<Vec<u8>, Base122Error> {
    // First pass: determine required size.
    let mut required = 0usize;
    encode_internal(input, None, &mut required)?;

    // Second pass: actually encode.
    let mut buffer = vec![0u8; required];
    let mut written = 0usize;
    encode_internal(input, Some(&mut buffer[..]), &mut written)?;
    buffer.truncate(written);
    Ok(buffer)
}
/// Internal function to perform the encoding
fn encode_internal(input: &[u8], mut output: Option<&mut [u8]>, out_written: &mut usize) -> Result<(), Base122Error> {
    let mut reader = BitReader::new(input);
    let mut out_index = 0usize;
    *out_written = 0;

    loop {
        let (nbits, mut bits) = reader.read(7);
        if nbits == 0 {
            break;
        }
        if nbits < 7 {
            // Align the first bit to start at position 6.
            // E.g. if nbits = 3: 0abc0000
            bits <<= 7 - nbits;
        }

        if is_illegal(bits) {
            let illegal_index = get_illegal_index(bits);
            let mut b1: u8 = 0xC2; // 11000010
            let mut b2: u8 = 0x80; // 10000000

            // Try to read the next 7 bits.
            let (next_nbits, mut next_bits) = reader.read(7);
            // Align the first bit to start at position 6.
            next_bits = next_bits.wrapping_shl((7 - next_nbits) as u32);

            if next_nbits == 0 {
                b1 |= 0x7 << 2; // 11100
                next_bits = bits;
            } else {
                b1 |= illegal_index << 2;
            }

            // Push the first bit onto the first byte.
            let first_bit = (next_bits >> 6) & 1;
            b1 |= first_bit;
            b2 |= next_bits & 0x3F; // 00111111

            output_byte(&mut output, &mut out_index, out_written, b1)?;
            output_byte(&mut output, &mut out_index, out_written, b2)?;
        } else {
            output_byte(&mut output, &mut out_index, out_written, bits)?;
        }
    }

    Ok(())
}
/// Write the last 7 bits of byteVal for decoding.
/// Returns an error if byteVal has 1 bits exceeding the last byte boundary.
fn write_last_7(writer: &mut BitWriter, byte_val: u8, _error: &mut Base122Error) -> Result<(), Base122Error> {
    // Do not write extra bytes. Write up to the nearest bit boundary.
    let nbits = 8 - (writer.cur_bit % 8);
    if nbits == 8 {
        return Err(Base122Error::new("Decoded data is not a byte multiple"));
    }
    // Error if any bits after the last input bits are 1.
    // Example: nbits = 2
    // byteVal of 01100001 is an error. The rightmost 1 bit is unexpected.
    let mask: u8 = (!(0xFFu32 << (7 - nbits))) as u8;
    if (byte_val & mask) > 0 {
        return Err(Base122Error::new(
            "Encoded data is malformed. Last byte has extra data.",
        ));
    }
    // Shift bits to write.
    let shifted = byte_val >> (7 - nbits);
    writer
        .write(nbits as u8, shifted)
        .map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
    Ok(())
}
/// Decode Base122 encoded data to binary
///
/// # Arguments
///
/// * `input` - The Base122 encoded data
///
/// # Returns
///
/// * `Ok(Vec<u8>)` - The decoded binary data
/// * `Err(Base122Error)` - If there was an error during decoding
pub fn decode(input: &[u8]) -> Result<Vec<u8>, Base122Error> {
    // First pass: determine required size.
    let mut required = 0usize;
    {
        let mut writer = BitWriter::new(None, 0);
        decode_internal(input, &mut writer, &mut required)?;
    }
    // Second pass: actually decode.
    let mut buffer = vec![0u8; required];
    let mut written = 0usize;
    {
        let mut writer = BitWriter::new(Some(&mut buffer[..]), required);
        decode_internal(input, &mut writer, &mut written)?;
    }
    buffer.truncate(written);
    Ok(buffer)
}
/// Internal function to perform the decoding
fn decode_internal(input: &[u8], writer: &mut BitWriter, out_written: &mut usize) -> Result<(), Base122Error> {
    let in_len = input.len();
    let mut cur_byte = 0usize;
    let mut error = Base122Error::new("");

    while cur_byte < in_len {
        if input[cur_byte] >> 7 == 0 {
            // One byte sequence.
            let cur_byte_val = input[cur_byte];
            if cur_byte + 1 == in_len {
                write_last_7(writer, cur_byte_val, &mut error)?;
            } else {
                writer
                    .write(7, cur_byte_val)
                    .map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
            }
        } else {
            // Two byte sequence.
            let cur_byte_val = input[cur_byte];
            // Expect first byte to have form 110xxx1y.
            if (cur_byte_val & 0xE2) != 0xC2 {
                return Err(Base122Error::new(
                    "First byte of two byte sequence malformed",
                ));
            }
            if cur_byte + 1 == in_len {
                return Err(Base122Error::new(
                    "Two byte sequence is missing second byte",
                ));
            }
            cur_byte += 1;
            let next_byte_val = input[cur_byte];
            // Expect second byte to have form 10xxxxxx.
            if (next_byte_val & 0xC0) != 0x80 {
                return Err(Base122Error::new(
                    "Second byte of two byte sequence malformed",
                ));
            }

            let illegal_index = (cur_byte_val & 0x1C) >> 2;
            if illegal_index == 0x7 {
                // Shortened two byte sequence.
                if cur_byte + 1 != in_len {
                    return Err(Base122Error::new(
                        "Got unexpected extra data after shortened two byte sequence",
                    ));
                }
                let last_byte_val =
                    cur_byte_val.wrapping_shl(6) | (next_byte_val & 0x3F);
                write_last_7(writer, last_byte_val, &mut error)?;
            } else if (illegal_index as usize) < ILLEGALS.len() {
                writer
                    .write(7, ILLEGALS[illegal_index as usize])
                    .map_err(|_| Base122Error::new("Output does not have sufficient size"))?;

                let second_byte_val =
                    cur_byte_val.wrapping_shl(6) | (next_byte_val & 0x3F);
                if cur_byte + 1 == in_len {
                    write_last_7(writer, second_byte_val, &mut error)?;
                } else {
                    writer
                        .write(7, second_byte_val)
                        .map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
                }
            } else {
                return Err(Base122Error::new("Got unrecognized illegal index"));
            }
        }
        cur_byte += 1;
    }

    *out_written = writer.cur_bit / 8;
    Ok(())
}
