use std::hint::assert_unchecked;

use super::constants::*;
use crate::board::Board;
use chess_core::prelude::*;

#[repr(C, align(64))]
pub struct Network {
    pub feature_weights: [[[i8; HIDDEN_SIZE]; 768]; INPUT_BUCKETS],
    pub feature_biases: [i16; HIDDEN_SIZE],
    pub output_weights: [[[i16; HIDDEN_SIZE]; 2]; OUTPUT_BUCKETS],
    pub output_bias: [i16; OUTPUT_BUCKETS],
}

include!(concat!(env!("OUT_DIR"), "/nnue_data.rs"));

impl Network {
    #[inline(always)]
    pub(super) fn feature_index(
        perspective: Color,
        piece: ColoredPiece,
        sq: Sq,
        flip: u8,
    ) -> usize {
        let piece_color = piece.color() as usize;
        let piece_type = piece.piece() as usize;
        let pers = perspective as usize;
        let color_offset = (piece_color ^ pers) * 384;
        let v_flip = (pers ^ 1) * 56;
        let oriented_sq = (sq as usize) ^ v_flip ^ (flip as usize);
        color_offset + piece_type * 64 + oriented_sq
    }

    #[inline]
    pub fn king_bucket_and_flip(perspective: Color, king_sq: Sq) -> (usize, u8) {
        let king_sq = if perspective == Color::White {
            king_sq
        } else {
            king_sq.flip_vertical()
        };
        let file = king_sq.file();
        let rank = king_sq.rank();

        let flip = if file > 3 { 7 } else { 0 };
        let index = (rank * 4) + (file ^ flip);

        (BUCKET_LAYOUT[index as usize] as usize, flip)
    }

    #[inline(always)]
    pub fn feature_weights(&self, piece_idx: usize, bucket: usize) -> &[i8; HIDDEN_SIZE] {
        debug_assert!(piece_idx < 768);
        debug_assert!(bucket < INPUT_BUCKETS);
        unsafe {
            self.feature_weights
                .get_unchecked(bucket)
                .get_unchecked(piece_idx)
        }
    }

    #[inline(always)]
    pub fn bucket_index(piece_count: u8) -> usize {
        let bucket = (piece_count - 2) / 32u8.div_ceil(OUTPUT_BUCKETS as u8);
        unsafe {
            assert_unchecked(bucket < OUTPUT_BUCKETS as u8);
        }
        bucket as usize
    }

    /// Output bucket and the (white, black) perspective output weights for `board`
    #[inline(always)]
    pub fn output_layer(&'static self, board: &Board) -> (usize, OutputWeights, OutputWeights) {
        let bucket = Self::bucket_index(board.occupied().count_ones() as u8);
        let [us, them] = &self.output_weights[bucket];
        match board.to_play {
            Color::White => (bucket, us, them),
            Color::Black => (bucket, them, us),
        }
    }

    #[inline(always)]
    pub fn finalize(&self, out_bucket: usize, sum: i32) -> i16 {
        let mut out = sum / QA;
        out += self.output_bias[out_bucket] as i32;
        out *= SCALE;
        out /= QA * QB;
        out as i16
    }
}

pub fn advise_huge_pages() {
    #[cfg(target_os = "linux")]
    unsafe {
        const HUGE_PAGE: usize = 2 * 1024 * 1024;
        let len = size_of::<Network>().next_multiple_of(HUGE_PAGE);
        libc::madvise(&raw const *NNUE as *mut libc::c_void, len, libc::MADV_HUGEPAGE);
    }
}

pub type SideAccumulator = [i16; HIDDEN_SIZE];
pub type OutputWeights = &'static [i16; HIDDEN_SIZE];
