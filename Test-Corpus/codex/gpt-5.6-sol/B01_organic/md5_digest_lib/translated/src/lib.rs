#[repr(C)]
pub struct TflacMd5 {
    pub a: u32,
    pub b: u32,
    pub c: u32,
    pub d: u32,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn md5_digest(m: *const TflacMd5, out: *mut u8) {
    unsafe {
        out.add(0).write(m.read().a as u8);
        out.add(1).write((m.read().a >> 8) as u8);
        out.add(2).write((m.read().a >> 16) as u8);
        out.add(3).write((m.read().a >> 24) as u8);
        out.add(4).write(m.read().b as u8);
        out.add(5).write((m.read().b >> 8) as u8);
        out.add(6).write((m.read().b >> 16) as u8);
        out.add(7).write((m.read().b >> 24) as u8);
        out.add(8).write(m.read().c as u8);
        out.add(9).write((m.read().c >> 8) as u8);
        out.add(10).write((m.read().c >> 16) as u8);
        out.add(11).write((m.read().c >> 24) as u8);
        out.add(12).write(m.read().d as u8);
        out.add(13).write((m.read().d >> 8) as u8);
        out.add(14).write((m.read().d >> 16) as u8);
        out.add(15).write((m.read().d >> 24) as u8);
    }
}
