use lazy_static::lazy_static;

lazy_static! {
    static ref BYTE_LOOKUP: [u8; 256] = {
        let mut l = [0b11111111; 256];

        l[b'A' as usize] = 0b00;
        l[b'a' as usize] = 0b00;
        l[b'C' as usize] = 0b01;
        l[b'c' as usize] = 0b01;
        l[b'G' as usize] = 0b10;
        l[b'g' as usize] = 0b10;
        l[b'T' as usize] = 0b11;
        l[b't' as usize] = 0b11;

        l
    };

    static ref ASCII_LOOKUP: [&'static str; 4] = {
        let mut l = ["A"; 4];

        // Do not need:
        // l[0b00 as usize] = 'A';
        l[0b01] = "C";
        l[0b10] = "G";
        l[0b11] = "T";

        l
    };
}

#[inline(always)]
pub fn a_to_b(s: u8) -> u8 {
    BYTE_LOOKUP[s as usize]
}

#[inline(always)]
pub fn b_to_a(b: u8) -> &'static str {
    assert!(
        b <= 0b11,
        "Byte must be 2-bits to represent a valid alphabet"
    );
    ASCII_LOOKUP[b as usize]
}
