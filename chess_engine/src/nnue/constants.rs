pub const INPUT_BUCKETS: usize = 8;
pub const HIDDEN_SIZE: usize = 1536;
pub const QA: i32 = 127;
pub const QB: i32 = 64;
pub const SCALE: i32 = 400;
pub const OUTPUT_BUCKETS: usize = 8;

#[rustfmt::skip]
pub const BUCKET_LAYOUT: [u8; 32] = [
    0, 0, 1, 2, // rank 1: a1, b1 | c1 | d1 (center Ke1)
    3, 3, 4, 4, // rank 2: a2, b2 (g2 flank) | c2, d2 (center Ke2)
    5, 5, 5, 5, // rank 3: midfield
    6, 6, 6, 6, // rank 4: midfield
    7, 7, 7, 7, // rank 5: endgame
    7, 7, 7, 7, // rank 6: endgame
    7, 7, 7, 7, // rank 7: endgame
    7, 7, 7, 7, // rank 8: endgame
];
