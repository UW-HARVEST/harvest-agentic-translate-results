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
        Base122Error {
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
        BitReader {
            input,
            byte_pos: 0,
            bit_pos: 0,
        }
    }
    /// Read up to `nbits` from the input, returns (bits_read, value)
    pub fn read(&mut self, nbits: u8) -> (u8, u8) {
        debug_assert!(nbits >= 1 && nbits <= 8);

        let cur_bit = self.byte_pos * 8 + self.bit_pos;
        let bit_len = self.input.len() * 8;
        let max_nbits = bit_len.saturating_sub(cur_bit);
        let mut nbits = nbits as usize;

        if nbits > max_nbits {
            nbits = max_nbits;
        }

        if nbits == 0 {
            return (0, 0);
        }

        let first_byte_index = cur_bit / 8;
        let first_byte_cur_bit = cur_bit % 8;
        let mut two_bytes: u16 = self.input[first_byte_index] as u16;
        two_bytes <<= 8;
        if first_byte_index + 1 < self.input.len() {
            two_bytes |= self.input[first_byte_index + 1] as u16;
        }
        let shift = (8 - first_byte_cur_bit) + (8 - nbits);
        two_bytes >>= shift;
        let mask: u8 = !(0xFFu16 << nbits) as u8;
        let out = (two_bytes as u8) & mask;

        let new_cur_bit = cur_bit + nbits;
        self.byte_pos = new_cur_bit / 8;
        self.bit_pos = new_cur_bit % 8;

        (nbits as u8, out)
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
        BitWriter {
            output,
            len,
            cur_bit: 0,
            count_only,
        }
    }
    /// Write `nbits` bits from `value` to the output
    /// Returns the number of bytes used so far or an error
    pub fn write(&mut self, nbits: u8, value: u8) -> Result<usize, Base122Error> {
        debug_assert!(nbits >= 1 && nbits <= 8);
        let nbits = nbits as usize;

        if self.count_only {
            self.cur_bit += nbits;
            return Ok((self.cur_bit + 7) / 8);
        }

        let bit_len = self.len * 8;
        if self.cur_bit + nbits > bit_len {
            return Err(Base122Error::new("Output does not have sufficient size"));
        }

        let first_byte_index = self.cur_bit / 8;
        let first_byte_cur_bit = self.cur_bit % 8;

        let output = self
            .output
            .as_deref_mut()
            .ok_or_else(|| Base122Error::new("Output does not have sufficient size"))?;

        let preserve_mask: u8 = !(0xFFu16 >> first_byte_cur_bit) as u8;
        let mut two_bytes: u16 = (output[first_byte_index] & preserve_mask) as u16;
        two_bytes <<= 8;

        let value_mask: u8 = !(0xFFu16 << nbits) as u8;
        let shift = 8 + (8 - first_byte_cur_bit) - nbits;
        two_bytes |= ((value & value_mask) as u16) << shift;

        output[first_byte_index] = (two_bytes >> 8) as u8;
        if first_byte_cur_bit + nbits > 8 {
            output[first_byte_index + 1] = two_bytes as u8;
        }
        self.cur_bit += nbits;

        Ok((self.cur_bit + 7) / 8)
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
    ILLEGALS.iter().any(|&v| v == val)
}
/// Get the index of an illegal character in the ILLEGALS array
fn get_illegal_index(val: u8) -> u8 {
    for (i, &v) in ILLEGALS.iter().enumerate() {
        if v == val {
            return i as u8;
        }
    }
    panic!("get_illegal_index called on non-illegal value")
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
    let mut needed: usize = 0;
    encode_internal(input, None, &mut needed)?;
    let mut buffer = vec![0u8; needed];
    let mut written: usize = 0;
    encode_internal(input, Some(&mut buffer), &mut written)?;
    buffer.truncate(written);
    Ok(buffer)
}
/// Internal function to perform the encoding
fn encode_internal(
    input: &[u8],
    mut output: Option<&mut [u8]>,
    out_written: &mut usize,
) -> Result<(), Base122Error> {
    let mut reader = BitReader::new(input);
    *out_written = 0;
    let mut out_index: usize = 0;
    let count_only = output.is_none();
    let out_len = output.as_deref().map(|o| o.len()).unwrap_or(0);

    loop {
        let (nbits, mut bits) = reader.read(7);
        if nbits == 0 {
            break;
        }
        if nbits < 7 {
            bits <<= 7 - nbits;
        }

        if is_illegal(bits) {
            let illegal_index = get_illegal_index(bits);
            let mut b1: u8 = 0xC2;
            let mut b2: u8 = 0x80;

            let (next_nbits, mut next_bits) = reader.read(7);
            if next_nbits > 0 && next_nbits < 7 {
                next_bits <<= 7 - next_nbits;
            }

            if next_nbits == 0 {
                b1 |= 0x7 << 2;
                next_bits = bits;
            } else {
                b1 |= illegal_index << 2;
            }

            let first_bit = (next_bits >> 6) & 1;
            b1 |= first_bit;
            b2 |= next_bits & 0x3F;

            // Output b1
            if count_only {
                *out_written += 1;
            } else {
                if out_index == out_len {
                    return Err(Base122Error::new("output does not have sufficient size"));
                }
                if let Some(out) = output.as_deref_mut() {
                    out[out_index] = b1;
                }
                *out_written += 1;
                out_index += 1;
            }
            // Output b2
            if count_only {
                *out_written += 1;
            } else {
                if out_index == out_len {
                    return Err(Base122Error::new("output does not have sufficient size"));
                }
                if let Some(out) = output.as_deref_mut() {
                    out[out_index] = b2;
                }
                *out_written += 1;
                out_index += 1;
            }
        } else {
            // Output bits
            if count_only {
                *out_written += 1;
            } else {
                if out_index == out_len {
                    return Err(Base122Error::new("output does not have sufficient size"));
                }
                if let Some(out) = output.as_deref_mut() {
                    out[out_index] = bits;
                }
                *out_written += 1;
                out_index += 1;
            }
        }
    }
    Ok(())
}
/// Write the last 7 bits of byteVal for decoding.
/// Returns an error if byteVal has 1 bits exceeding the last byte boundary.
fn write_last_7(
    writer: &mut BitWriter,
    byte_val: u8,
    _error: &mut Base122Error,
) -> Result<(), Base122Error> {
    let nbits = 8 - (writer.cur_bit % 8);
    if nbits == 8 {
        return Err(Base122Error::new("Decoded data is not a byte multiple"));
    }
    let mask: u8 = !(0xFFu16 << (7 - nbits)) as u8;
    if (byte_val & mask) > 0 {
        return Err(Base122Error::new(
            "Encoded data is malformed. Last byte has extra data.",
        ));
    }
    let val = byte_val >> (7 - nbits);
    if writer.write(nbits as u8, val).is_err() {
        return Err(Base122Error::new("Output does not have sufficient size"));
    }
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
    // First pass: count
    let mut count_writer = BitWriter::new(None, 0);
    let mut needed: usize = 0;
    decode_internal(input, &mut count_writer, &mut needed)?;

    let mut buffer = vec![0u8; needed];
    let buffer_len = buffer.len();
    let mut writer = BitWriter::new(Some(&mut buffer), buffer_len);
    let mut written: usize = 0;
    decode_internal(input, &mut writer, &mut written)?;
    drop(writer);
    buffer.truncate(written);
    Ok(buffer)
}
/// Internal function to perform the decoding
fn decode_internal(
    input: &[u8],
    writer: &mut BitWriter,
    out_written: &mut usize,
) -> Result<(), Base122Error> {
    let in_len = input.len();
    let mut cur_byte: usize = 0;

    while cur_byte < in_len {
        let cur_byte_val = input[cur_byte];

        if cur_byte_val >> 7 == 0 {
            // One byte sequence
            if cur_byte + 1 == in_len {
                let mut error = Base122Error::new("");
                write_last_7(writer, cur_byte_val, &mut error)?;
            } else if writer.write(7, cur_byte_val).is_err() {
                return Err(Base122Error::new("Output does not have sufficient size"));
            }
        } else {
            // Two byte sequence
            if (cur_byte_val & 0xE2) != 0xC2 {
                return Err(Base122Error::new(
                    "First byte of two byte sequence malformed",
                ));
            }
            if cur_byte + 1 == in_len {
                return Err(Base122Error::new("Two byte sequence is missing second byte"));
            }
            cur_byte += 1;
            let next_byte_val = input[cur_byte];
            if (next_byte_val & 0xC0) != 0x80 {
                return Err(Base122Error::new(
                    "Second byte of two byte sequence malformed",
                ));
            }

            let illegal_index = (cur_byte_val & 0x1C) >> 2;
            if illegal_index == 0x7 {
                // Shortened two byte sequence
                if cur_byte + 1 != in_len {
                    return Err(Base122Error::new(
                        "Got unexpected extra data after shortened two byte sequence",
                    ));
                }
                let mut last_byte_val = cur_byte_val;
                last_byte_val = last_byte_val.wrapping_shl(6);
                last_byte_val |= next_byte_val & 0x3F;

                let mut error = Base122Error::new("");
                write_last_7(writer, last_byte_val, &mut error)?;
            } else if (illegal_index as usize) < ILLEGALS.len() {
                if writer.write(7, ILLEGALS[illegal_index as usize]).is_err() {
                    return Err(Base122Error::new("Output does not have sufficient size"));
                }
                let mut second_byte_val = cur_byte_val;
                second_byte_val = second_byte_val.wrapping_shl(6);
                second_byte_val |= next_byte_val & 0x3F;

                if cur_byte + 1 == in_len {
                    let mut error = Base122Error::new("");
                    write_last_7(writer, second_byte_val, &mut error)?;
                } else if writer.write(7, second_byte_val).is_err() {
                    return Err(Base122Error::new("Output does not have sufficient size"));
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
