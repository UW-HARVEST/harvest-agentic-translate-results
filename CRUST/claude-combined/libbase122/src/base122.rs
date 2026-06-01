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
        debug_assert!(nbits > 0);
        debug_assert!(nbits <= 8);
        let mut nbits = nbits as usize;
        let cur_bit = self.byte_pos * 8 + self.bit_pos;
        let bit_len = self.input.len() * 8;
        let max_nbits = bit_len.saturating_sub(cur_bit);

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
        two_bytes >>= (8 - first_byte_cur_bit) + (8 - nbits);
        let mask: u8 = (!(255u16 << nbits)) as u8;
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
        debug_assert!(nbits > 0);
        debug_assert!(nbits <= 8);
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

        if let Some(out) = self.output.as_mut() {
            let mask1: u8 = (!(255u16 >> first_byte_cur_bit)) as u8;
            let preserved = out[first_byte_index] & mask1;
            let mut two_bytes: u16 = (preserved as u16) << 8;

            let mask2: u8 = (!(255u16 << nbits)) as u8;
            let shift = 8 + (8 - first_byte_cur_bit) - nbits;
            two_bytes |= ((value & mask2) as u16) << shift;

            out[first_byte_index] = (two_bytes >> 8) as u8;
            if first_byte_cur_bit + nbits > 8 {
                out[first_byte_index + 1] = two_bytes as u8;
            }
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
    panic!("unreachable: get_illegal_index called with non-illegal value")
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
    let mut out_size = 0usize;
    encode_internal(input, None, &mut out_size)?;
    let mut output = vec![0u8; out_size];
    let mut written = 0usize;
    encode_internal(input, Some(&mut output), &mut written)?;
    Ok(output)
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
    let out_len = match &output {
        Some(o) => o.len(),
        None => 0,
    };

    // Helper to emit a byte; modeled after the OUTPUT_BYTE macro in C.
    // We inline the logic by calling a closure capturing the relevant locals.
    let emit = |b: u8,
                    out_index: &mut usize,
                    out_written: &mut usize,
                    output: &mut Option<&mut [u8]>|
     -> Result<(), Base122Error> {
        if count_only {
            *out_written += 1;
        } else {
            if *out_index == out_len {
                return Err(Base122Error::new("output does not have sufficient size"));
            }
            if let Some(out) = output.as_mut() {
                out[*out_index] = b;
            }
            *out_written += 1;
            *out_index += 1;
        }
        Ok(())
    };

    loop {
        let (nbits, mut bits) = reader.read(7);
        if nbits == 0 {
            break;
        }
        if nbits < 7 {
            // Align the first bit to start at position 6.
            bits = bits << (7 - nbits);
        }

        if is_illegal(bits) {
            let illegal_index = get_illegal_index(bits);
            let mut b1: u8 = 0xC2; // 11000010
            let mut b2: u8 = 0x80; // 10000000

            let (next_nbits, mut next_bits) = reader.read(7);
            if next_nbits > 0 {
                next_bits = next_bits << (7 - next_nbits);
            }

            if next_nbits == 0 {
                b1 |= 0x7 << 2; // 11100
                next_bits = bits;
            } else {
                b1 |= illegal_index << 2;
            }

            // Push the first bit onto the first byte
            let first_bit = (next_bits >> 6) & 1;
            b1 |= first_bit;
            b2 |= next_bits & 0x3F;

            emit(b1, &mut out_index, out_written, &mut output)?;
            emit(b2, &mut out_index, out_written, &mut output)?;
        } else {
            emit(bits, &mut out_index, out_written, &mut output)?;
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
    // Do not write extra bytes. Write up to the nearest bit boundary.
    let nbits = 8 - (writer.cur_bit % 8);
    if nbits == 8 {
        return Err(Base122Error::new("Decoded data is not a byte multiple"));
    }
    // Error if any bits after the last input bits are 1.
    let mask: u8 = (!(0xFFu16 << (7 - nbits))) as u8;
    if (byte_val & mask) > 0 {
        return Err(Base122Error::new(
            "Encoded data is malformed. Last byte has extra data.",
        ));
    }
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
    // First pass: figure out required size by using count-only writer.
    let mut size_writer = BitWriter::new(None, 0);
    let mut needed = 0usize;
    decode_internal(input, &mut size_writer, &mut needed)?;

    let mut output = vec![0u8; needed];
    let out_len = output.len();
    let mut writer = BitWriter::new(Some(&mut output), out_len);
    let mut written = 0usize;
    decode_internal(input, &mut writer, &mut written)?;
    output.truncate(written);
    Ok(output)
}
/// Internal function to perform the decoding
fn decode_internal(
    input: &[u8],
    writer: &mut BitWriter,
    out_written: &mut usize,
) -> Result<(), Base122Error> {
    let mut error = Base122Error::new("");
    let in_len = input.len();
    let mut cur_byte: usize = 0;

    while cur_byte < in_len {
        let cur_byte_val = input[cur_byte];
        if cur_byte_val >> 7 == 0 {
            // One byte sequence.
            if cur_byte + 1 == in_len {
                write_last_7(writer, cur_byte_val, &mut error)?;
            } else {
                writer
                    .write(7, cur_byte_val)
                    .map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
            }
        } else {
            // Two byte sequence.
            // Expect first byte to have form 110xxx1y.
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
                    (cur_byte_val.wrapping_shl(6)) | (next_byte_val & 0x3F);
                write_last_7(writer, last_byte_val, &mut error)?;
            } else if (illegal_index as usize) < ILLEGALS.len() {
                writer
                    .write(7, ILLEGALS[illegal_index as usize])
                    .map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
                let second_byte_val =
                    (cur_byte_val.wrapping_shl(6)) | (next_byte_val & 0x3F);
                if cur_byte + 1 == in_len {
                    write_last_7(writer, second_byte_val, &mut error)?;
                } else {
                    writer.write(7, second_byte_val).map_err(|_| {
                        Base122Error::new("Output does not have sufficient size")
                    })?;
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
