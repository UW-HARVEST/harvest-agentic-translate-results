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
        f.write_str(&self.message)
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
        debug_assert!((1..=8).contains(&nbits));

        let cur_bit = self.byte_pos * 8 + self.bit_pos;
        let bit_len = self.input.len() * 8;
        let mut nbits = nbits as usize;
        let max_nbits = bit_len.saturating_sub(cur_bit);
        if nbits > max_nbits {
            nbits = max_nbits;
        }

        if nbits == 0 {
            return (0, 0);
        }

        let first_byte_index = cur_bit / 8;
        let first_byte_cur_bit = cur_bit % 8;
        let mut two_bytes = (self.input[first_byte_index] as u16) << 8;
        if first_byte_index + 1 < self.input.len() {
            two_bytes |= self.input[first_byte_index + 1] as u16;
        }
        two_bytes >>= (8 - first_byte_cur_bit) + (8 - nbits);
        let mask = (1u16 << nbits) - 1;
        let out = (two_bytes & mask) as u8;

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
        Self {
            output,
            len,
            cur_bit: 0,
            count_only: false,
        }
    }
    /// Write `nbits` bits from `value` to the output
    /// Returns the number of bytes used so far or an error
    pub fn write(&mut self, nbits: u8, value: u8) -> Result<usize, Base122Error> {
        debug_assert!((1..=8).contains(&nbits));

        let nbits = nbits as usize;
        if self.count_only {
            self.cur_bit += nbits;
            return Ok(self.cur_bit.div_ceil(8));
        }

        if self.cur_bit + nbits > self.len * 8 {
            return Err(Base122Error::new("Output does not have sufficient size"));
        }

        let first_byte_index = self.cur_bit / 8;
        let first_byte_cur_bit = self.cur_bit % 8;

        let output = self
            .output
            .as_deref_mut()
            .expect("BitWriter output missing while not in count_only mode");

        let preserve_mask = if first_byte_cur_bit == 0 {
            0
        } else {
            (!0u8) << (8 - first_byte_cur_bit)
        };
        let mut two_bytes = ((output[first_byte_index] & preserve_mask) as u16) << 8;

        let value_mask = if nbits == 8 {
            u8::MAX
        } else {
            ((1u16 << nbits) - 1) as u8
        };
        two_bytes |= ((value & value_mask) as u16) << (16 - first_byte_cur_bit - nbits);

        output[first_byte_index] = (two_bytes >> 8) as u8;
        if first_byte_cur_bit + nbits > 8 {
            output[first_byte_index + 1] = two_bytes as u8;
        }

        self.cur_bit += nbits;
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
    ILLEGALS.contains(&val)
}
/// Get the index of an illegal character in the ILLEGALS array
fn get_illegal_index(val: u8) -> u8 {
    ILLEGALS
        .iter()
        .position(|&illegal| illegal == val)
        .map(|idx| idx as u8)
        .expect("illegal character index must exist")
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
    let mut out_written = 0;
    encode_internal(input, None, &mut out_written)?;

    let mut output = vec![0; out_written];
    let mut written = 0;
    encode_internal(input, Some(&mut output), &mut written)?;
    debug_assert_eq!(written, out_written);
    Ok(output)
}
/// Internal function to perform the encoding
fn encode_internal(input: &[u8], mut output: Option<&mut [u8]>, out_written: &mut usize) -> Result<(), Base122Error> {
    let mut reader = BitReader::new(input);
    let mut out_index = 0usize;
    let count_only = output.is_none();
    *out_written = 0;

    while let (bits_read @ 1..=7, mut bits) = reader.read(7) {
        if bits_read < 7 {
            bits <<= 7 - bits_read;
        }

        if is_illegal(bits) {
            let illegal_index = get_illegal_index(bits);
            let (next_bits_read, mut next_bits) = reader.read(7);
            next_bits <<= 7 - next_bits_read;

            let mut b1 = 0xC2u8;
            let mut b2 = 0x80u8;

            if next_bits_read == 0 {
                b1 |= 0x7 << 2;
                next_bits = bits;
            } else {
                b1 |= illegal_index << 2;
            }

            let first_bit = (next_bits >> 6) & 1;
            b1 |= first_bit;
            b2 |= next_bits & 0x3F;

            for byte in [b1, b2] {
                if count_only {
                    *out_written += 1;
                } else {
                    let out = output.as_deref_mut().expect("output must exist");
                    if out_index == out.len() {
                        return Err(Base122Error::new("output does not have sufficient size"));
                    }
                    out[out_index] = byte;
                    out_index += 1;
                    *out_written += 1;
                }
            }
        } else if count_only {
            *out_written += 1;
        } else {
            let out = output.as_deref_mut().expect("output must exist");
            if out_index == out.len() {
                return Err(Base122Error::new("output does not have sufficient size"));
            }
            out[out_index] = bits;
            out_index += 1;
            *out_written += 1;
        }
    }

    Ok(())
}
/// Write the last 7 bits of byteVal for decoding.
/// Returns an error if byteVal has 1 bits exceeding the last byte boundary.
fn write_last_7(writer: &mut BitWriter, byte_val: u8, error: &mut Base122Error) -> Result<(), Base122Error> {
    let nbits = 8 - (writer.cur_bit % 8);
    if nbits == 8 {
        let err = Base122Error::new("Decoded data is not a byte multiple");
        *error = err.clone();
        return Err(err);
    }

    let mask = if 7 == nbits {
        0
    } else {
        ((1u16 << (7 - nbits)) - 1) as u8
    };
    if (byte_val & mask) > 0 {
        let err = Base122Error::new("Encoded data is malformed. Last byte has extra data.");
        *error = err.clone();
        return Err(err);
    }

    let shifted = byte_val >> (7 - nbits);
    if let Err(_) = writer.write(nbits as u8, shifted) {
        let err = Base122Error::new("Output does not have sufficient size");
        *error = err.clone();
        return Err(err);
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
    let mut count_writer = BitWriter::new(None, 0);
    count_writer.count_only = true;
    let mut out_written = 0;
    decode_internal(input, &mut count_writer, &mut out_written)?;

    let mut output = vec![0; out_written];
    let output_len = output.len();
    let mut writer = BitWriter::new(Some(&mut output), output_len);
    let mut written = 0;
    decode_internal(input, &mut writer, &mut written)?;
    debug_assert_eq!(written, out_written);
    Ok(output)
}
/// Internal function to perform the decoding
fn decode_internal(input: &[u8], writer: &mut BitWriter, out_written: &mut usize) -> Result<(), Base122Error> {
    let mut error = Base122Error::new("");
    let mut cur_byte = 0usize;

    while cur_byte < input.len() {
        let cur_byte_val = input[cur_byte];
        if cur_byte_val >> 7 == 0 {
            if cur_byte + 1 == input.len() {
                write_last_7(writer, cur_byte_val, &mut error)?;
            } else if writer.write(7, cur_byte_val).is_err() {
                return Err(Base122Error::new("Output does not have sufficient size"));
            }
        } else {
            if (cur_byte_val & 0xE2) != 0xC2 {
                return Err(Base122Error::new(
                    "First byte of two byte sequence malformed",
                ));
            }
            if cur_byte + 1 == input.len() {
                return Err(Base122Error::new(
                    "Two byte sequence is missing second byte",
                ));
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
                if cur_byte + 1 != input.len() {
                    return Err(Base122Error::new(
                        "Got unexpected extra data after shortened two byte sequence",
                    ));
                }

                let last_byte_val = (cur_byte_val << 6) | (next_byte_val & 0x3F);
                write_last_7(writer, last_byte_val, &mut error)?;
            } else if (illegal_index as usize) < ILLEGALS.len() {
                if writer.write(7, ILLEGALS[illegal_index as usize]).is_err() {
                    return Err(Base122Error::new("Output does not have sufficient size"));
                }

                let second_byte_val = (cur_byte_val << 6) | (next_byte_val & 0x3F);
                if cur_byte + 1 == input.len() {
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
