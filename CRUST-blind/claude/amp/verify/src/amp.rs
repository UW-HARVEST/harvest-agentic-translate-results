/// Protocol version.
pub const AMP_VERSION: i16 = 1;
/// Message struct.
pub struct Amp {
    /// Protocol version.
    pub version: i16,
    /// Number of arguments.
    pub argc: i16,
    /// Encoded buffer.
    pub buf: String,
}
impl Amp {
    /// Decodes the given buffer into this message.
    ///
    /// # Arguments
    ///
    /// * `buf` - A string slice containing the encoded message.
    pub fn decode(&mut self, buf: &str) {
        let bytes = buf.as_bytes();
        // First byte: high nibble is version, low nibble is argc.
        let header = bytes[0];
        self.version = (header >> 4) as i16;
        self.argc = (header & 0x0F) as i16;
        // Remaining bytes are the encoded arguments.
        let rest: Vec<u8> = bytes[1..].to_vec();
        self.buf = String::from_utf8(rest).expect("amp: buffer is not valid UTF-8");
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        // Read the 4-byte big-endian length.
        let bytes = self.buf.as_bytes();
        let len = ((bytes[0] as u32) << 24
            | (bytes[1] as u32) << 16
            | (bytes[2] as u32) << 8
            | (bytes[3] as u32)) as usize;

        // Layout of self.buf is: <4-byte len><arg><rest>
        let arg_start = 4;
        let arg_end = arg_start + len;

        // Extract pieces.
        let arg_bytes: Vec<u8> = bytes[arg_start..arg_end].to_vec();
        let rest_bytes: Vec<u8> = bytes[arg_end..].to_vec();
        let len_be: [u8; 4] = [
            ((len >> 24) & 0xFF) as u8,
            ((len >> 16) & 0xFF) as u8,
            ((len >> 8) & 0xFF) as u8,
            (len & 0xFF) as u8,
        ];

        // Rotate: place <rest> first, followed by the length+arg we just consumed.
        // This lets the next call parse the next argument from the start of self.buf,
        // while still keeping the just-decoded arg accessible via a slice at the tail.
        let mut new_bytes: Vec<u8> = Vec::with_capacity(rest_bytes.len() + 4 + arg_bytes.len());
        new_bytes.extend_from_slice(&rest_bytes);
        new_bytes.extend_from_slice(&len_be);
        new_bytes.extend_from_slice(&arg_bytes);

        self.buf = String::from_utf8(new_bytes).expect("amp: buffer is not valid UTF-8");

        // The decoded arg now sits at the end of self.buf.
        let total = self.buf.len();
        &self.buf[total - len..]
    }
}
/// Encodes the given arguments into a message buffer.
///
/// # Arguments
///
/// * `argv` - A slice of string slices representing the arguments.
///
/// # Returns
///
/// A `String` containing the encoded message.
pub fn amp_encode(argv: &[&str]) -> String {
    let argc = argv.len();

    // Compute total length: 1 byte header + (4 bytes length + data) per arg.
    let mut total: usize = 1;
    for arg in argv {
        total += 4 + arg.as_bytes().len();
    }

    let mut bytes: Vec<u8> = Vec::with_capacity(total);

    // Header: high nibble version, low nibble argc.
    let header: u8 = ((AMP_VERSION as u8) << 4) | ((argc as u8) & 0x0F);
    bytes.push(header);

    // Each argument: 4-byte big-endian length followed by data.
    for arg in argv {
        let arg_bytes = arg.as_bytes();
        let len = arg_bytes.len() as u32;
        bytes.push(((len >> 24) & 0xFF) as u8);
        bytes.push(((len >> 16) & 0xFF) as u8);
        bytes.push(((len >> 8) & 0xFF) as u8);
        bytes.push((len & 0xFF) as u8);
        bytes.extend_from_slice(arg_bytes);
    }

    String::from_utf8(bytes).expect("amp: encoded buffer is not valid UTF-8")
}
