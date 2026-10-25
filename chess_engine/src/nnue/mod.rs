use crate::{
    board::Board,
    search::{MAX_PLY, SearchStack, StackMove},
};
use chess_core::prelude::*;
use constants::*;
use fearless_simd::{Level, Simd, dispatch, i32x16, prelude::*};
use kernels::{Delta, apply_delta_copy, apply_delta_screlu_dot, screlu_dot};
use network::{NNUE, Network, SideAccumulator};

mod constants;
mod finny;
mod kernels;
mod network;

pub use finny::FinnyTable;
pub use network::advise_huge_pages;

pub fn evaluate(board: &Board) -> i16 {
    FinnyTable::default().eval(board)
}

#[repr(align(64))]
#[derive(Clone)]
struct PlyAccumulator([SideAccumulator; Color::NB]);

impl PlyAccumulator {
    #[inline(always)]
    fn side(&self, p: Color) -> &SideAccumulator {
        &self.0[p as usize]
    }

    #[inline(always)]
    fn side_mut(&mut self, p: Color) -> &mut SideAccumulator {
        &mut self.0[p as usize]
    }
}

/// Lazily updated per-ply accumulators. `stack[ply].stack_move` is the change record and
/// `stack[ply].acc_computed` tells which sides of `accs[ply]` are valid.
pub struct AccumulatorStack {
    accs: Box<[PlyAccumulator]>,
    finny: FinnyTable,
}

impl AccumulatorStack {
    pub fn new(board: &Board) -> Self {
        let mut this = Self {
            accs: vec![PlyAccumulator([[0; HIDDEN_SIZE]; Color::NB]); MAX_PLY as usize]
                .into_boxed_slice(),
            finny: FinnyTable::default(),
        };
        this.reset(board);
        this
    }

    pub fn reset(&mut self, board: &Board) {
        self.finny.eval(board);
        // Sets `board` as the root (ply 0)
        for p in [Color::White, Color::Black] {
            let (bucket, flip) = Network::king_bucket_and_flip(p, board.king_sq(p));
            *self.accs[0].side_mut(p) = self.finny.entry(p, bucket, flip).accum;
        }
    }

    pub fn eval(&mut self, board: &Board, stack: &mut SearchStack, ply: u16) -> i16 {
        let (out_bucket, white_w, black_w) = NNUE.output_layer(board);
        let sum = dispatch!(Level::baseline(), simd => {
            let mut sum = self.eval_side(simd, board, stack, ply, Color::White, white_w);
            sum += self.eval_side(simd, board, stack, ply, Color::Black, black_w);
            sum.reduce_sum()
        });

        let eval = NNUE.finalize(out_bucket, sum);
        debug_assert_eq!(eval, evaluate(board));
        eval
    }

    #[inline(always)]
    fn eval_side<S: Simd>(
        &mut self,
        simd: S,
        board: &Board,
        stack: &mut SearchStack,
        ply: u16,
        perspective: Color,
        out_w: &[i16; HIDDEN_SIZE],
    ) -> i32x16<S> {
        let pi = perspective as usize;
        let (bucket, flip) = Network::king_bucket_and_flip(perspective, board.king_sq(perspective));

        // Walk back to the last computed ply, or refresh if our king changed bucket
        let mut base = ply;
        while !stack[base].acc_computed[pi] {
            if changes_king_bucket(stack[base].stack_move, perspective) {
                stack[ply].acc_computed[pi] = true;
                let dst = self.accs[ply as usize].side_mut(perspective);
                let sum = self.finny.refresh(simd, board, perspective, dst, out_w);
                self.backward_fill(simd, stack, base, ply, perspective, bucket, flip);
                return sum;
            }
            base -= 1;
        }

        let mut src = base as usize;
        for i in base + 1..=ply {
            let m = stack[i].stack_move;
            let Some(piece) = m.moved_piece else {
                continue; // Skip null moves
            };
            let (lo, hi) = self.accs.split_at_mut(i as usize);
            let delta = move_delta(m, piece, perspective, flip);
            let sum = apply_delta_screlu_dot(
                simd,
                lo[src].side(perspective),
                hi[0].side_mut(perspective),
                &delta,
                bucket,
                out_w,
            );
            stack[i].acc_computed[pi] = true;
            src = i as usize;
            if i == ply {
                return sum;
            }
        }

        screlu_dot(simd, self.accs[src].side(perspective), out_w)
    }

    #[inline(never)]
    #[allow(clippy::too_many_arguments)]
    fn backward_fill<S: Simd>(
        &mut self,
        simd: S,
        stack: &mut SearchStack,
        king_ply: u16,
        ply: u16,
        perspective: Color,
        bucket: usize,
        flip: u8,
    ) {
        let pi = perspective as usize;
        let mut src = ply as usize;
        for i in (king_ply + 1..=ply).rev() {
            let m = stack[i].stack_move;
            let Some(piece) = m.moved_piece else {
                continue; // Same position as the parent: `src` still holds it
            };
            let (lo, hi) = self.accs.split_at_mut(src);
            let delta = move_delta(m, piece, perspective, flip).inverted();
            let dst = lo[i as usize - 1].side_mut(perspective);
            apply_delta_copy(simd, hi[0].side(perspective), dst, &delta, bucket);
            stack[i - 1].acc_computed[pi] = true;
            src = i as usize - 1;
        }
    }
}

#[inline(always)]
fn changes_king_bucket(m: StackMove, perspective: Color) -> bool {
    m.moved_piece == Some(ColoredPiece::new(Piece::King, perspective))
        && Network::king_bucket_and_flip(perspective, m.mov.from())
            != Network::king_bucket_and_flip(perspective, m.mov.to())
}

/// Feature changes of a non-null move
#[inline(always)]
fn move_delta(m: StackMove, piece: ColoredPiece, perspective: Color, flip: u8) -> Delta<2> {
    let (mov, us) = (m.mov, piece.color());
    let index = |piece, sq| Network::feature_index(perspective, piece, sq, flip);
    let to_piece = mov
        .promotion_piece()
        .map_or(piece, |promo| ColoredPiece::new(promo, us));

    let mut delta = Delta::new();
    delta.sub(index(piece, mov.from()));
    delta.add(index(to_piece, mov.to()));
    if let Some(captured) = m.captured {
        delta.sub(index(captured, mov.capture_square(us)));
    } else if mov.is_castle() {
        let (rook_from, rook_to) = mov.castling_rook_squares(us);
        let rook = ColoredPiece::new(Piece::Rook, us);
        delta.sub(index(rook, rook_from));
        delta.add(index(rook, rook_to));
    }
    delta
}
