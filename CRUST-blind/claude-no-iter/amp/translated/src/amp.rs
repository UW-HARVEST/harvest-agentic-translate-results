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
        let header = bytes[0];
        self.version = ((header >> 4) & 0x0f) as i16;
        self.argc = (header & 0x0f) as i16;
        // Internal buffer layout invariant for sequential `decode_arg` calls:
        //
        //   <unconsumed bytes> <previously returned arg bytes> <4-byte BE length of previously returned arg>
        //
        // Initially there is no previously returned arg, so we append a
        // four-byte zero suffix indicating "previous arg length = 0".
        let mut new_buf: Vec<u8> = Vec::with_capacity(bytes.len() - 1 + 4);
        new_buf.extend_from_slice(&bytes[1..]);
        new_buf.extend_from_slice(&[0u8; 4]);
        self.buf = String::from_utf8(new_buf)
            .expect("amp::decode: encoded buffer is not valid UTF-8");
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let (remaining_start, remaining_end, current_len) = {
            let bytes = self.buf.as_bytes();
            let total_len = bytes.len();

            // Read the previously returned arg's length from the last 4 bytes
            // (a trailing length suffix that we use as a tiny cursor).
            let prev_arg_len = u32::from_be_bytes([
                bytes[total_len - 4],
                bytes[total_len - 3],
                bytes[total_len - 2],
                bytes[total_len - 1],
            ]) as usize;

            // The unconsumed portion ends right before the previous-arg suffix.
            let unparsed_end = total_len - 4 - prev_arg_len;

            // Read the next arg's length from the start of the unconsumed portion.
            let current_len = u32::from_be_bytes([
                bytes[0],
                bytes[1],
                bytes[2],
                bytes[3],
            ]) as usize;

            // Current arg data lives at bytes[4 .. 4 + current_len].
            // What remains unparsed afterwards is bytes[4 + current_len .. unparsed_end].
            (4 + current_len, unparsed_end, current_len)
        };

        // Build the new buffer:
        //   <remaining_unparsed_bytes>
        //   <current_arg_bytes>
        //   <current_len as 4-byte big endian>
        let bytes = self.buf.as_bytes();
        let remaining = &bytes[remaining_start..remaining_end];
        let current_arg = &bytes[4..4 + current_len];

        let mut new_buf: Vec<u8> =
            Vec::with_capacity(remaining.len() + current_arg.len() + 4);
        new_buf.extend_from_slice(remaining);
        new_buf.extend_from_slice(current_arg);
        new_buf.extend_from_slice(&(current_len as u32).to_be_bytes());

        self.buf = String::from_utf8(new_buf)
            .expect("amp::decode_arg: rebuilt buffer is not valid UTF-8");

        // The current arg now sits between the remaining unparsed bytes and
        // the 4-byte length suffix.
        let total = self.buf.len();
        let start = total - 4 - current_len;
        let end = total - 4;
        &self.buf[start..end]
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
    // Compute total length: 1 byte header + 4 bytes length + payload per arg.
    let argc = argv.len();
    let mut total = 1usize;
    let mut lens: Vec<usize> = Vec::with_capacity(argc);
    for arg in argv {
        let l = arg.as_bytes().len();
        lens.push(l);
        total += 4 + l;
    }

    let mut buf: Vec<u8> = Vec::with_capacity(total);

    // Header byte: version in the high nibble, argc in the low nibble.
    let header: u8 = ((AMP_VERSION as u8) << 4) | ((argc as u8) & 0x0f);
    buf.push(header);

    // Encode each argument as a 4-byte big-endian length followed by data.
    for (i, arg) in argv.iter().enumerate() {
        let l = lens[i] as u32;
        buf.extend_from_slice(&l.to_be_bytes());
        buf.extend_from_slice(arg.as_bytes());
    }

    String::from_utf8(buf)
        .expect("amp_encode: encoded buffer is not valid UTF-8")
}
