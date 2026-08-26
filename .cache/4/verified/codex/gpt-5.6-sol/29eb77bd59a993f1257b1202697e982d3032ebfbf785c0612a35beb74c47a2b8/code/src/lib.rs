const CRC16_TABLES: [[u16; 256]; 8] = make_crc16_tables();

const fn make_crc16_tables() -> [[u16; 256]; 8] {
    let mut tables = [[0; 256]; 8];
    let mut value = 0;

    while value < 256 {
        let mut crc = (value as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
            bit += 1;
        }
        tables[0][value] = crc;
        value += 1;
    }

    let mut table = 1;
    while table < 8 {
        value = 0;
        while value < 256 {
            let previous = tables[table - 1][value];
            tables[table][value] = (previous << 8) ^ tables[0][(previous >> 8) as usize];
            value += 1;
        }
        table += 1;
    }

    tables
}

/// Calculates the FLAC CRC-16 using the caller-provided initial value.
///
/// # Safety
///
/// When `len` is nonzero, `data` must point to at least `len` readable bytes.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(mut data: *const u8, mut len: u32, mut crc: u16) -> u16 {
    while len >= 8 {
        // SAFETY: The function's C contract requires `len` readable bytes.
        unsafe {
            crc ^= ((*data.add(0) as u16) << 8) | (*data.add(1) as u16);
            crc = CRC16_TABLES[7][(crc >> 8) as usize]
                ^ CRC16_TABLES[6][(crc & 0xff) as usize]
                ^ CRC16_TABLES[5][*data.add(2) as usize]
                ^ CRC16_TABLES[4][*data.add(3) as usize]
                ^ CRC16_TABLES[3][*data.add(4) as usize]
                ^ CRC16_TABLES[2][*data.add(5) as usize]
                ^ CRC16_TABLES[1][*data.add(6) as usize]
                ^ CRC16_TABLES[0][*data.add(7) as usize];
            data = data.add(8);
        }
        len -= 8;
    }

    while len != 0 {
        // SAFETY: The function's C contract requires `len` readable bytes.
        unsafe {
            crc = (crc << 8) ^ CRC16_TABLES[0][((crc >> 8) as u8 ^ *data) as usize];
            data = data.add(1);
        }
        len -= 1;
    }

    crc
}
