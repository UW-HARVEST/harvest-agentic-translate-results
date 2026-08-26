const fn generate_crc16_tables() -> [[u16; 256]; 8] {
    let mut tables = [[0u16; 256]; 8];
    let mut i = 0usize;
    while i < 256 {
        let mut crc = (i as u16) << 8;
        let mut bit = 0;
        while bit < 8 {
            crc = if (crc & 0x8000) != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
            bit += 1;
        }
        tables[0][i] = crc;
        i += 1;
    }

    let mut row = 1usize;
    while row < 8 {
        let mut col = 0usize;
        while col < 256 {
            let crc = tables[row - 1][col];
            tables[row][col] = (crc << 8) ^ tables[0][(crc >> 8) as usize];
            col += 1;
        }
        row += 1;
    }

    tables
}

const TFLAC_CRC16_TABLES: [[u16; 256]; 8] = generate_crc16_tables();

#[unsafe(no_mangle)]
pub unsafe extern "C" fn crc16(mut d: *const u8, mut len: u32, mut crc16: u16) -> u16 {
    while len >= 8 {
        // Preserve the C implementation's raw-pointer semantics and arithmetic.
        crc16 ^= (u16::from(unsafe { *d }) << 8) | u16::from(unsafe { *d.add(1) });
        crc16 = TFLAC_CRC16_TABLES[7][(crc16 >> 8) as usize]
            ^ TFLAC_CRC16_TABLES[6][(crc16 & 0x00ff) as usize]
            ^ TFLAC_CRC16_TABLES[5][unsafe { *d.add(2) } as usize]
            ^ TFLAC_CRC16_TABLES[4][unsafe { *d.add(3) } as usize]
            ^ TFLAC_CRC16_TABLES[3][unsafe { *d.add(4) } as usize]
            ^ TFLAC_CRC16_TABLES[2][unsafe { *d.add(5) } as usize]
            ^ TFLAC_CRC16_TABLES[1][unsafe { *d.add(6) } as usize]
            ^ TFLAC_CRC16_TABLES[0][unsafe { *d.add(7) } as usize];
        d = unsafe { d.add(8) };
        len -= 8;
    }

    while len != 0 {
        crc16 = (crc16 << 8) ^ TFLAC_CRC16_TABLES[0][((crc16 >> 8) ^ u16::from(unsafe { *d })) as usize];
        d = unsafe { d.add(1) };
        len -= 1;
    }

    crc16
}
