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
        Base122Error { message: message.to_string() }
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
        BitReader { input, byte_pos: 0, bit_pos: 0 }
    }
    /// Read up to `nbits` from the input, returns (bits_read, value)
    pub fn read(&mut self, nbits: u8) -> (u8, u8) {
        let bit_len = self.input.len() * 8;
        let cur_bit = self.byte_pos * 8 + self.bit_pos;
        let max_nbits = if bit_len > cur_bit { bit_len - cur_bit } else { 0 };
        let nbits = (nbits as usize).min(max_nbits);

        if nbits == 0 {
            return (0, 0);
        }

        let first_byte_index = cur_bit / 8;
        let first_byte_cur_bit = cur_bit % 8;
        let mut two_bytes: u16 = (self.input[first_byte_index] as u16) << 8;
        if first_byte_index + 1 < self.input.len() {
            two_bytes |= self.input[first_byte_index + 1] as u16;
        }
        two_bytes >>= (8 - first_byte_cur_bit) + (8 - nbits);
        let mask: u8 = !((0xFFu16 << nbits) as u8);
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
        BitWriter { output, len, cur_bit: 0, count_only }
    }
    /// Write `nbits` bits from `value` to the output
    /// Returns the number of bytes used so far or an error
    pub fn write(&mut self, nbits: u8, value: u8) -> Result<usize, Base122Error> {
        let nbits = nbits as usize;

        if self.count_only {
            self.cur_bit += nbits;
            return Ok(self.cur_bit / 8);
        }

        let bit_len = self.len * 8;
        if self.cur_bit + nbits > bit_len {
            return Err(Base122Error::new("Output does not have sufficient size"));
        }

        let first_byte_index = self.cur_bit / 8;
        let first_byte_cur_bit = self.cur_bit % 8;

        let out = self.output.as_mut().unwrap();

        let mask_keep: u8 = !(0xFFu8 >> first_byte_cur_bit);
        let mut two_bytes: u16 = ((out[first_byte_index] & mask_keep) as u16) << 8;

        let mask_in: u8 = !((0xFFu16 << nbits) as u8);
        two_bytes |= ((value & mask_in) as u16) << (8 + (8 - first_byte_cur_bit) - nbits);

        out[first_byte_index] = (two_bytes >> 8) as u8;
        if first_byte_cur_bit + nbits > 8 {
            out[first_byte_index + 1] = two_bytes as u8;
        }
        self.cur_bit += nbits;
        Ok(self.cur_bit / 8)
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
    ILLEGALS.contains(&val)
}
/// Get the index of an illegal character in the ILLEGALS array
fn get_illegal_index(val: u8) -> u8 {
    ILLEGALS.iter().position(|&v| v == val).expect("unreachable") as u8
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
    // First pass: count only
    let mut count = 0usize;
    encode_internal(input, None, &mut count)?;
    // Second pass: write
    let mut out = vec![0u8; count];
    let mut written = 0usize;
    encode_internal(input, Some(&mut out), &mut written)?;
    Ok(out)
}
/// Internal function to perform the encoding
fn encode_internal(input: &[u8], mut output: Option<&mut [u8]>, out_written: &mut usize) -> Result<(), Base122Error> {
    let mut reader = BitReader::new(input);
    let mut out_index = 0usize;
    let count_only = output.is_none();
    *out_written = 0;

    macro_rules! output_byte {
        ($b:expr) => {
            if count_only {
                *out_written += 1;
            } else {
                let out = output.as_mut().unwrap();
                if out_index == out.len() {
                    return Err(Base122Error::new("output does not have sufficient size"));
                }
                out[out_index] = $b;
                *out_written += 1;
                out_index += 1;
            }
        };
    }

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
            next_bits <<= 7 - next_nbits;

            if next_nbits == 0 {
                b1 |= 0x7 << 2; // 11100
                next_bits = bits;
            } else {
                b1 |= illegal_index << 2;
            }

            let first_bit = (next_bits >> 6) & 1;
            b1 |= first_bit;
            b2 |= next_bits & 0x3F;

            output_byte!(b1);
            output_byte!(b2);
        } else {
            output_byte!(bits);
        }
    }

    Ok(())
}
/// Write the last 7 bits of byteVal for decoding.
/// Returns an error if byteVal has 1 bits exceeding the last byte boundary.
fn write_last_7(writer: &mut BitWriter, byte_val: u8, _error: &mut Base122Error) -> Result<(), Base122Error> {
    let nbits = 8 - (writer.cur_bit % 8);
    if nbits == 8 {
        return Err(Base122Error::new("Decoded data is not a byte multiple"));
    }
    let mask: u8 = !(0xFFu8 << (7 - nbits));
    if (byte_val & mask) > 0 {
        return Err(Base122Error::new("Encoded data is malformed. Last byte has extra data."));
    }
    let shifted = byte_val >> (7 - nbits);
    writer.write(nbits as u8, shifted).map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
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
    // First pass: count only
    let mut count = 0usize;
    {
        let mut writer = BitWriter::new(None, 0);
        decode_internal(input, &mut writer, &mut count)?;
    }
    // Second pass: write
    let mut out = vec![0u8; count];
    let mut written = 0usize;
    {
        let len = out.len();
        let mut writer = BitWriter::new(Some(&mut out), len);
        decode_internal(input, &mut writer, &mut written)?;
    }
    Ok(out)
}
/// Internal function to perform the decoding
fn decode_internal(input: &[u8], writer: &mut BitWriter, out_written: &mut usize) -> Result<(), Base122Error> {
    let mut dummy_error = Base122Error::new("");
    let in_len = input.len();
    let mut cur_byte = 0usize;

    while cur_byte < in_len {
        if input[cur_byte] >> 7 == 0 {
            // One byte sequence
            let cur_byte_val = input[cur_byte];
            if cur_byte + 1 == in_len {
                write_last_7(writer, cur_byte_val, &mut dummy_error)?;
            } else {
                writer.write(7, cur_byte_val).map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
            }
        } else {
            // Two byte sequence
            let cur_byte_val = input[cur_byte];
            if (cur_byte_val & 0xE2) != 0xC2 {
                return Err(Base122Error::new("First byte of two byte sequence malformed"));
            }
            if cur_byte + 1 == in_len {
                return Err(Base122Error::new("Two byte sequence is missing second byte"));
            }
            cur_byte += 1;
            let next_byte_val = input[cur_byte];
            if (next_byte_val & 0xC0) != 0x80 {
                return Err(Base122Error::new("Second byte of two byte sequence malformed"));
            }

            let illegal_index = (cur_byte_val & 0x1C) >> 2;
            if illegal_index == 0x7 {
                // Shortened two byte sequence
                if cur_byte + 1 != in_len {
                    return Err(Base122Error::new("Got unexpected extra data after shortened two byte sequence"));
                }
                let last_byte_val = (cur_byte_val << 6) | (next_byte_val & 0x3F);

                if cur_byte + 1 == in_len {
                    write_last_7(writer, last_byte_val, &mut dummy_error)?;
                } else {
                    writer.write(7, last_byte_val).map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
                }
            } else if (illegal_index as usize) < ILLEGALS.len() {
                writer.write(7, ILLEGALS[illegal_index as usize]).map_err(|_| Base122Error::new("Output does not have sufficient size"))?;

                let second_byte_val = (cur_byte_val << 6) | (next_byte_val & 0x3F);

                if cur_byte + 1 == in_len {
                    write_last_7(writer, second_byte_val, &mut dummy_error)?;
                } else {
                    writer.write(7, second_byte_val).map_err(|_| Base122Error::new("Output does not have sufficient size"))?;
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
