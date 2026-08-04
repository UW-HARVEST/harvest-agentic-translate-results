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
        assert!((1..=8).contains(&nbits));

        let cur_bit = self.byte_pos * 8 + self.bit_pos;
        let bit_len = self.input.len() * 8;
        let max_nbits = bit_len.saturating_sub(cur_bit);
        let nbits = usize::min(nbits as usize, max_nbits);

        if nbits == 0 {
            return (0, 0);
        }

        let first_byte_index = cur_bit / 8;
        let first_byte_cur_bit = cur_bit % 8;
        let mut two_bytes = u16::from(self.input[first_byte_index]) << 8;
        if first_byte_index + 1 < self.input.len() {
            two_bytes |= u16::from(self.input[first_byte_index + 1]);
        }

        let shift = (8 - first_byte_cur_bit) + (8 - nbits);
        two_bytes >>= shift;
        let mask = if nbits == 8 {
            0xFF
        } else {
            ((1u16 << nbits) - 1) as u8
        };
        let out = (two_bytes as u8) & mask;

        let next_bit = cur_bit + nbits;
        self.byte_pos = next_bit / 8;
        self.bit_pos = next_bit % 8;

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
            count_only: output.is_none(),
            output,
            len,
            cur_bit: 0,
        }
    }
    /// Write `nbits` bits from `value` to the output
    /// Returns the number of bytes used so far or an error
    pub fn write(&mut self, nbits: u8, value: u8) -> Result<usize, Base122Error> {
        assert!((1..=8).contains(&nbits));

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
        let mask = !((0xFFu16 >> first_byte_cur_bit) as u8);
        let buf = self
            .output
            .as_deref_mut()
            .expect("BitWriter output must exist unless count_only is true");

        let mut two_bytes = u16::from(buf[first_byte_index] & mask) << 8;
        let value_mask = if nbits == 8 {
            0xFF
        } else {
            ((1u16 << nbits) - 1) as u8
        };
        let shift = 8 + (8 - first_byte_cur_bit) - nbits;
        two_bytes |= u16::from(value & value_mask) << shift;

        buf[first_byte_index] = (two_bytes >> 8) as u8;
        if first_byte_cur_bit + nbits > 8 {
            buf[first_byte_index + 1] = two_bytes as u8;
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
    for (idx, illegal) in ILLEGALS.iter().enumerate() {
        if *illegal == val {
            return idx as u8;
        }
    }
    debug_assert!(false, "unreachable");
    u8::MAX
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
    let mut encoded_len = 0;
    encode_internal(input, None, &mut encoded_len)?;

    let mut output = vec![0; encoded_len];
    let mut written = 0;
    encode_internal(input, Some(output.as_mut_slice()), &mut written)?;
    debug_assert_eq!(written, encoded_len);
    Ok(output)
}
/// Internal function to perform the encoding
fn encode_internal(input: &[u8], mut output: Option<&mut [u8]>, out_written: &mut usize) -> Result<(), Base122Error> {
    fn emit_byte(
        output: &mut Option<&mut [u8]>,
        out_index: &mut usize,
        out_written: &mut usize,
        byte: u8,
    ) -> Result<(), Base122Error> {
        if let Some(buf) = output.as_deref_mut() {
            if *out_index == buf.len() {
                return Err(Base122Error::new("output does not have sufficient size"));
            }
            buf[*out_index] = byte;
            *out_index += 1;
        }

        *out_written += 1;
        Ok(())
    }

    let mut reader = BitReader::new(input);
    let mut out_index = 0usize;
    *out_written = 0;

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
            let (next_nbits, mut next_bits) = reader.read(7);
            next_bits <<= 7 - next_nbits;

            let mut b1 = 0xC2u8;
            let mut b2 = 0x80u8;
            if next_nbits == 0 {
                b1 |= 0x7 << 2;
                next_bits = bits;
            } else {
                b1 |= illegal_index << 2;
            }

            let first_bit = (next_bits >> 6) & 1;
            b1 |= first_bit;
            b2 |= next_bits & 0x3F;

            emit_byte(&mut output, &mut out_index, out_written, b1)?;
            emit_byte(&mut output, &mut out_index, out_written, b2)?;
        } else {
            emit_byte(&mut output, &mut out_index, out_written, bits)?;
        }
    }

    Ok(())
}
/// Write the last 7 bits of byteVal for decoding.
/// Returns an error if byteVal has 1 bits exceeding the last byte boundary.
fn write_last_7(writer: &mut BitWriter, byte_val: u8, error: &mut Base122Error) -> Result<(), Base122Error> {
    let nbits = 8 - (writer.cur_bit % 8);
    if nbits == 8 {
        *error = Base122Error::new("Decoded data is not a byte multiple");
        return Err(error.clone());
    }

    let mask = if 7 == nbits {
        0
    } else {
        ((1u16 << (7 - nbits)) - 1) as u8
    };
    if (byte_val & mask) > 0 {
        *error = Base122Error::new("Encoded data is malformed. Last byte has extra data.");
        return Err(error.clone());
    }

    let byte_val = byte_val >> (7 - nbits);
    if writer.write(nbits as u8, byte_val).is_err() {
        *error = Base122Error::new("Output does not have sufficient size");
        return Err(error.clone());
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
    let mut decoded_len = 0;
    decode_internal(input, &mut count_writer, &mut decoded_len)?;

    let mut output = vec![0; decoded_len];
    let len = output.len();
    let mut writer = BitWriter::new(Some(output.as_mut_slice()), len);
    let mut written = 0;
    decode_internal(input, &mut writer, &mut written)?;
    debug_assert_eq!(written, decoded_len);
    Ok(output)
}
/// Internal function to perform the decoding
fn decode_internal(input: &[u8], writer: &mut BitWriter, out_written: &mut usize) -> Result<(), Base122Error> {
    let mut cur_byte = 0usize;

    while cur_byte < input.len() {
        let write_7 = |writer: &mut BitWriter, pos: usize, val: u8| -> Result<(), Base122Error> {
            if pos + 1 == input.len() {
                let mut error = Base122Error::new("");
                write_last_7(writer, val, &mut error)
            } else if writer.write(7, val).is_err() {
                Err(Base122Error::new("Output does not have sufficient size"))
            } else {
                Ok(())
            }
        };

        if input[cur_byte] >> 7 == 0 {
            write_7(writer, cur_byte, input[cur_byte])?;
        } else {
            let cur_byte_val = input[cur_byte];
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

                let last_byte_val = cur_byte_val.wrapping_shl(6) | (next_byte_val & 0x3F);
                write_7(writer, cur_byte, last_byte_val)?;
            } else if usize::from(illegal_index) < ILLEGALS.len() {
                if writer
                    .write(7, ILLEGALS[usize::from(illegal_index)])
                    .is_err()
                {
                    return Err(Base122Error::new("Output does not have sufficient size"));
                }

                let second_byte_val = cur_byte_val.wrapping_shl(6) | (next_byte_val & 0x3F);
                write_7(writer, cur_byte, second_byte_val)?;
            } else {
                return Err(Base122Error::new("Got unrecognized illegal index"));
            }
        }

        cur_byte += 1;
    }

    *out_written = writer.cur_bit / 8;
    Ok(())
}
