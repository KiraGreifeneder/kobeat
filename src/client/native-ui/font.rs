/// Tiny 3x5 bitmap font, digits + a few symbols only, enough for the
/// position/duration readout. Each row's 3 bits are left-to-right (MSB first).
pub fn glyph_rows(ch: char) -> [u8; 5] {
    match ch {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 7, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 1, 1, 1],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        ':' => [0, 2, 0, 2, 0],
        '/' => [1, 1, 2, 4, 4],
        '-' => [0, 0, 7, 0, 0],
        _ => [0, 0, 0, 0, 0],
    }
}
