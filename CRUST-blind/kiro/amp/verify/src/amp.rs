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
        let b = buf.as_bytes()[0];
        self.version = (b >> 4) as i16;
        self.argc = (b & 0xf) as i16;
        self.buf = buf[1..].to_string();
    }
    /// Decodes and returns the next argument from the message.
    ///
    /// # Returns
    ///
    /// A string slice representing the next decoded argument.
    pub fn decode_arg(&mut self) -> &str {
        let bytes = self.buf.as_bytes();
        let len = ((bytes[0] as u32) << 24
            | (bytes[1] as u32) << 16
            | (bytes[2] as u32) << 8
            | (bytes[3] as u32)) as usize;
        let arg_end = 4 + len;
        let new_buf = format!("{}{}", &self.buf[arg_end..], &self.buf[4..arg_end]);
        self.buf = new_buf;
        &self.buf[self.buf.len() - len..]
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
    let mut out = Vec::with_capacity(1 + argc * 4);
    out.push(((AMP_VERSION as u8) << 4) | (argc as u8));
    for arg in argv {
        let len = arg.len() as u32;
        out.push((len >> 24) as u8);
        out.push((len >> 16) as u8);
        out.push((len >> 8) as u8);
        out.push(len as u8);
        out.extend_from_slice(arg.as_bytes());
    }
    out.iter().map(|&b| b as char).collect()
}
