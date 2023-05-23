pub enum Dna {
    A = 0b00,
    C = 0b01,
    G = 0b10,
    T = 0b11,
}

pub fn a_to_b(s: u8) -> u8 {
    match s {
        b'A' | b'a' => 0b00,
        b'C' | b'c' => 0b01,
        b'G' | b'g' => 0b10,
        b'T' | b't' => 0b11,
        _ => panic!(
            "ASCII character {} not recognised as a valid DNA sequence",
            s
        ),
    }
}

pub fn b_to_a(b: u8) -> &'static str {
    match b {
        0b00 => "A",
        0b01 => "C",
        0b10 => "G",
        0b11 => "T",
        _ => panic!(
            "Byte {} not recognised as valid encoding for a DNA sequence",
            b
        ),
    }
}
