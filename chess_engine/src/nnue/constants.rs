pub const INPUT_BUCKETS: usize = 16;
pub const HIDDEN_SIZE: usize = 1536;
pub const QA: i32 = 127;
pub const QB: i32 = 64;
pub const SCALE: i32 = 400;
pub const OUTPUT_BUCKETS: usize = 8;

#[rustfmt::skip]
pub const BUCKET_LAYOUT: [u8; 32] = [
     0,  1,  2,  3, // rank 1: one bucket per square
     4,  5,  6,  7, // rank 2: one bucket per square
     8,  9, 10, 11, // rank 3
     8,  9, 10, 11, // rank 4
    12, 12, 13, 13, // rank 5
    12, 12, 13, 13, // rank 6
    14, 14, 15, 15, // rank 7
    14, 14, 15, 15, // rank 8
];
