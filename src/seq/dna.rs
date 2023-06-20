use lazy_static::lazy_static;

lazy_static! {
    static ref BYTE_LOOKUP: [u8; 256] = {
        let mut l = [0b11111111; 256];

        l[b'A' as usize] = 0b00;
        l[b'a' as usize] = 0b00;
        l[b'C' as usize] = 0b01;
        l[b'c' as usize] = 0b01;
        l[b'G' as usize] = 0b11;
        l[b'g' as usize] = 0b11;
        l[b'T' as usize] = 0b10;
        l[b't' as usize] = 0b10;

        l
    };

    static ref ASCII_LOOKUP: [&'static str; 4] = {
        let mut l = ["A"; 4];

        // Do not need:
        // l[0b00 as usize] = 'A';
        l[0b01] = "C";
        l[0b11] = "G";
        l[0b10] = "T";

        l
    };
}

#[inline(always)]
pub fn dna_to_u8(s: u8) -> u8 {
    BYTE_LOOKUP[s as usize]
}

#[inline(always)]
pub fn u8_to_dna(b: u8) -> &'static str {
    assert!(
        b <= 0b11,
        "Byte must be 2-bits to represent a valid alphabet"
    );
    ASCII_LOOKUP[b as usize]
}

pub fn dna_to_u32(
    v1: u8,
    v2: u8,
    v3: u8,
    v4: u8,
    v5: u8,
    v6: u8,
    v7: u8,
    v8: u8,
    v9: u8,
    v10: u8,
    v11: u8,
    v12: u8,
    v13: u8,
    v14: u8,
    v15: u8,
    v16: u8,
) -> u32 {
    let mut total = ((v1 & 0b00000110) as u32) >> 1;
    //total += dna_to_u32_macro!(v2, v3, v4, v5, v6, v7, v8);
    total += ((v2 & 0b00000110) as u32) << 1;
    total += ((v3 & 0b00000110) as u32) << 3;
    total += ((v4 & 0b00000110) as u32) << 5;
    total += ((v5 & 0b00000110) as u32) << 7;
    total += ((v6 & 0b00000110) as u32) << 9;
    total += ((v7 & 0b00000110) as u32) << 11;
    total += ((v8 & 0b00000110) as u32) << 13;
    total += ((v9 & 0b00000110) as u32) << 15;
    total += ((v10 & 0b00000110) as u32) << 17;
    total += ((v11 & 0b00000110) as u32) << 19;
    total += ((v12 & 0b00000110) as u32) << 21;
    total += ((v13 & 0b00000110) as u32) << 23;
    total += ((v14 & 0b00000110) as u32) << 25;
    total += ((v15 & 0b00000110) as u32) << 27;
    total += ((v16 & 0b00000110) as u32) << 29;
    total
}

#[cfg(test)]
mod tests {
    #[test]
    fn dna_to_u32_test() {
        let v = vec![
            'A', 'T', 'C', 'G', 'A', 'T', 'C', 'G', 'A', 'T', 'C', 'G', 'A', 'T', 'C', 'G',
        ];
        let bit_encoding = 0b11011000110110001101100011011000;

        assert_eq!(super::dna_to_u32(v), bit_encoding)
    }
}
