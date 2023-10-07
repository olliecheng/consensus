use lazy_static::lazy_static;

pub const A: u8 = 0b00;
pub const T: u8 = 0b10;
pub const C: u8 = 0b01;
pub const G: u8 = 0b11;

lazy_static! {
    pub static ref ASCII_LOOKUP: [char; 4] = {
        let mut l = ['A'; 4];

        // Do not need:
        // l[0b00 as usize] = 'A';
        l[0b01] = 'C';
        l[0b11] = 'G';
        l[0b10] = 'T';

        l
    };
}

#[inline(always)]
pub fn dna_to_u8(s: u8) -> u8 {
    return (s & 0b00000110) >> 1;
}

#[inline(always)]
pub fn u8_to_dna(b: u8) -> char {
    assert!(
        b <= 0b11,
        "Byte must be 2-bits to represent a valid alphabet"
    );
    ASCII_LOOKUP[b as usize]
}

#[inline(always)]
pub fn dna_to_u32(vars: &[u8; 16]) -> u32 {
    let mut total = u32::from_le_bytes([vars[0], vars[4], vars[8], vars[12]]);
    total &= 0b00000110000001100000011000000110;
    total = total >> 1;

    for offset in 1..4 {
        let mut v = u32::from_le_bytes([
            vars[0 + offset],
            vars[4 + offset],
            vars[8 + offset],
            vars[12 + offset],
        ]);
        v &= 0b00000110000001100000011000000110;

        total += v << ((offset * 2) - 1);
    }

    total
}

#[cfg(test)]
mod tests {
    #[test]
    fn dna_to_u32_test1() {
        let v = vec![
            b'A', b'T', b'C', b'G', b'A', b'T', b'C', b'G', b'A', b'T', b'C', b'G', b'A', b'T',
            b'C', b'G',
        ];
        let bit_encoding = 0b11011000110110001101100011011000;

        assert_eq!(
            super::dna_to_u32(v.as_slice().try_into().unwrap()),
            bit_encoding
        )
    }

    #[test]
    fn dna_to_u32_test2() {
        let mut v = [0u8; 16];
        v[0] = b'T';

        assert_eq!(super::dna_to_u32(&v), 0b00000000000000000000000000000010);
    }
}
