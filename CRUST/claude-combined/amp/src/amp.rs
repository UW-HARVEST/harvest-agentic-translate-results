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

fn read_u32_be(buf: &[u8]) -> u32 {
    ((buf[0] as u32) << 24)
        | ((buf[1] as u32) << 16)
        | ((buf[2] as u32) << 8)
        | (buf[3] as u32)
}

fn write_u32_be(buf: &mut [u8], n: u32) {
    buf[0] = ((n >> 24) & 0xff) as u8;
    buf[1] = ((n >> 16) & 0xff) as u8;
    buf[2] = ((n >> 8) & 0xff) as u8;
    buf[3] = (n & 0xff) as u8;
}

impl Amp {
    /// Decodes the given buffer into this message.
    ///
    /// # Arguments
    ///
    /// * `buf` - A string slice containing the encoded message.
    pub fn decode(&mut self, buf: &str) {
        let bytes = buf.as_bytes();
        // First byte: high nibble = version, low nibble = argc.
        self.version = (bytes[0] >> 4) as i16;
        self.argc = (bytes[0] & 0xf) as i16;
        // Internal layout for incremental decoding:
        //   [4-byte previous_arg_length][previous_arg_bytes][remaining_encoded_data]
        // Initialize with a zero-length "previous arg" so the first call to
        // decode_arg simply skips 4 bytes and starts reading the first arg.
        let mut new_buf: Vec<u8> = Vec::with_capacity(4 + bytes.len() - 1);
        new_buf.extend_from_slice(&[0u8, 0, 0, 0]);
        new_buf.extend_from_slice(&bytes[1..]);
        self.buf = String::from_utf8(new_buf)
            .expect("amp::decode: input contained invalid UTF-8");
    }

    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        // Skip past the previously-returned arg block.
        let len_prev = read_u32_be(&bytes[0..4]) as usize;
        let cursor = 4 + len_prev;
        // Read the next arg's length and content.
        let len_next = read_u32_be(&bytes[cursor..cursor + 4]) as usize;
        let arg_start = cursor + 4;
        let arg_end = arg_start + len_next;
        let arg_bytes = bytes[arg_start..arg_end].to_vec();
        let rest_bytes = bytes[arg_end..].to_vec();

        // Rebuild self.buf in the canonical layout:
        //   [4-byte len_next][arg bytes][remaining encoded data]
        let mut new_buf: Vec<u8> = Vec::with_capacity(4 + len_next + rest_bytes.len());
        let mut prefix = [0u8; 4];
        write_u32_be(&mut prefix, len_next as u32);
        new_buf.extend_from_slice(&prefix);
        new_buf.extend_from_slice(&arg_bytes);
        new_buf.extend_from_slice(&rest_bytes);

        self.buf = String::from_utf8(new_buf)
            .expect("amp::decode_arg: arg bytes were not valid UTF-8");
        &self.buf[4..4 + len_next]
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
    let mut bytes: Vec<u8> = Vec::new();
    // Header byte: high nibble = version, low nibble = argc.
    bytes.push(((AMP_VERSION as u8) << 4) | (argc as u8 & 0x0f));
    for arg in argv {
        let len = arg.len() as u32;
        let mut prefix = [0u8; 4];
        write_u32_be(&mut prefix, len);
        bytes.extend_from_slice(&prefix);
        bytes.extend_from_slice(arg.as_bytes());
    }
    String::from_utf8(bytes).expect("amp_encode: produced invalid UTF-8")
}
