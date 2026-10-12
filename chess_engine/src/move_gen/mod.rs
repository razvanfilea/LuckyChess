use chess_core::prelude::*;

use crate::{
    board::Board,
    search::{ContHistKeys, ContinuationHistoryTable, HistoryTable, KillerMoves},
};

mod generate;
mod move_list;
pub mod scoring;
mod traits;

pub use generate::*;
pub use move_list::*;
pub use traits::*;

// Quiets are sorted lazily: most cut nodes use 1–2 quiets
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
enum Order {
    Sorted,
    Unsorted,
    MaxFirst,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug, Default)]
enum GenStage {
    #[default]
    Init,
    GenerateCaptures,
    YieldGoodCaptures,
    GenerateEvasions,
    // Final list of [bad captures][quiets], sorted lazily
    YieldRemaining(Order),
    Done,
}

pub struct MoveGenerator<const QUIESENCE: bool = false> {
    start_ptr: MoveListPtr,
    end_ptr: MoveListPtr,
    stage: GenStage,
    list_index: usize,
    bad_count: usize,

    tt_move: Move,
    killer_moves: KillerMoves,
    conthist_keys: ContHistKeys,
}

impl<const QUIESENCE: bool> MoveGenerator<QUIESENCE> {
    pub fn new(
        move_buffer: MoveListPtr,
        tt_move: Move,
        killer_moves: KillerMoves,
        conthist_keys: ContHistKeys,
    ) -> Self {
        Self {
            start_ptr: move_buffer,
            end_ptr: move_buffer,
            stage: GenStage::default(),
            list_index: 0,
            bad_count: 0,
            tt_move,
            killer_moves,
            conthist_keys,
        }
    }

    #[inline(always)]
    pub fn next(
        &mut self,
        board: &Board,
        history: &HistoryTable,
        cont_history: &ContinuationHistoryTable,
    ) -> Option<ScoredMove> {
        loop {
            if self.list_index < self.len() {
                let mut mov = self.pick_next();
                if mov == self.tt_move {
                    continue;
                }

                if self.stage == GenStage::YieldGoodCaptures {
                    mov.score = scoring::capture_tier(mov.mov, board, mov.score);
                    // Defer bad captures to the front: slots before list_index are consumed
                    if mov.is_bad_capture() {
                        let index = self.bad_count;
                        self.as_slice_mut()[index] = mov;
                        self.bad_count += 1;
                        continue;
                    }
                }

                return Some(mov);
            }

            if matches!(self.stage, GenStage::YieldRemaining(_) | GenStage::Done) {
                return None;
            }

            if let Some(mov) = self.advance_stage(board, history, cont_history) {
                return Some(mov);
            }
        }
    }

    #[inline(always)]
    pub const fn next_ptr(&self) -> MoveListPtr {
        self.end_ptr
    }

    #[inline(always)]
    const fn len(&self) -> usize {
        unsafe { self.end_ptr.0.offset_from(self.start_ptr.0) as usize }
    }

    #[inline(always)]
    const fn as_slice(&self) -> &[ScoredMove] {
        unsafe { core::slice::from_raw_parts(self.start_ptr.0, self.len()) }
    }

    #[inline(always)]
    const fn as_slice_mut(&mut self) -> &mut [ScoredMove] {
        unsafe { core::slice::from_raw_parts_mut(self.start_ptr.0, self.len()) }
    }

    #[inline(always)]
    fn pick_next(&mut self) -> ScoredMove {
        let next = self.list_index;
        if let GenStage::YieldRemaining(order) = self.stage {
            let order = match order {
                Order::Sorted => Order::Sorted,
                Order::Unsorted => {
                    select_first_max(&mut self.as_slice_mut()[next..]);
                    Order::MaxFirst
                }
                Order::MaxFirst => {
                    insertion_sort(&mut self.as_slice_mut()[next..]);
                    Order::Sorted
                }
            };
            self.stage = GenStage::YieldRemaining(order);
        }

        let mov = unsafe { *self.as_slice().get_unchecked(next) };
        self.list_index += 1;
        mov
    }

    #[inline(never)]
    fn advance_stage(
        &mut self,
        board: &Board,
        history: &HistoryTable,
        cont_history: &ContinuationHistoryTable,
    ) -> Option<ScoredMove> {
        match self.stage {
            GenStage::Init => {
                self.stage = if board.in_check() {
                    GenStage::GenerateEvasions
                } else {
                    GenStage::GenerateCaptures
                };

                if board.pseudo_legal(self.tt_move) {
                    let mut mov = ScoredMove::new(self.tt_move);
                    mov.score = i16::MAX;
                    return Some(mov);
                }
            }
            GenStage::GenerateCaptures => {
                let ptr = if board.to_play == Color::White {
                    generate_moves::<White, Captures>(board, self.start_ptr)
                } else {
                    generate_moves::<Black, Captures>(board, self.start_ptr)
                };
                self.end_ptr = ptr;

                for scored_move in self.as_slice_mut() {
                    scored_move.score = scoring::score_capture(scored_move.mov, board);
                }

                insertion_sort(self.as_slice_mut());

                self.stage = GenStage::YieldGoodCaptures;
            }
            GenStage::YieldGoodCaptures => {
                // Good captures are exhausted: the list becomes [bad captures][quiets]
                self.list_index = 0;
                self.end_ptr = MoveListPtr(unsafe { self.start_ptr.0.add(self.bad_count) });

                // Bad captures were deferred in pick order, so they are already sorted
                if QUIESENCE {
                    self.stage = GenStage::Done;
                } else {
                    let ptr = if board.to_play == Color::White {
                        generate_moves::<White, Quiets>(board, self.end_ptr)
                    } else {
                        generate_moves::<Black, Quiets>(board, self.end_ptr)
                    };
                    self.end_ptr = ptr;

                    let (bad_count, killer_moves, conthist_keys) =
                        (self.bad_count, self.killer_moves, self.conthist_keys);
                    for scored_move in &mut self.as_slice_mut()[bad_count..] {
                        scored_move.score = scoring::score_quiet(
                            scored_move.mov,
                            board,
                            killer_moves,
                            history,
                            cont_history,
                            &conthist_keys,
                        );
                    }

                    self.stage = GenStage::YieldRemaining(Order::Unsorted);
                }
            }
            GenStage::GenerateEvasions => {
                let ptr = if board.to_play == Color::White {
                    generate_moves::<White, Evasions>(board, self.start_ptr)
                } else {
                    generate_moves::<Black, Evasions>(board, self.start_ptr)
                };
                self.end_ptr = ptr;

                let (killer_moves, conthist_keys) = (self.killer_moves, self.conthist_keys);
                for scored_move in self.as_slice_mut() {
                    let mov = scored_move.mov;
                    scored_move.score = if mov.is_tactical() {
                        scoring::capture_tier(mov, board, scoring::score_capture(mov, board))
                    } else {
                        scoring::score_quiet(
                            mov,
                            board,
                            killer_moves,
                            history,
                            cont_history,
                            &conthist_keys,
                        )
                    };
                }

                insertion_sort(self.as_slice_mut());

                self.stage = GenStage::Done;
            }
            GenStage::YieldRemaining(_) | GenStage::Done => {}
        }

        None
    }
}

fn select_first_max(moves: &mut [ScoredMove]) {
    let mut best = 0;
    for i in 1..moves.len() {
        if moves[i].score > moves[best].score {
            best = i;
        }
    }
    let mov = moves[best];
    moves.copy_within(0..best, 1);
    moves[0] = mov;
}

fn insertion_sort(moves: &mut [ScoredMove]) {
    for p in 1..moves.len() {
        let tmp = moves[p];
        let mut q = p;
        while q > 0 && moves[q - 1].score < tmp.score {
            moves[q] = moves[q - 1];
            q -= 1;
        }
        moves[q] = tmp;
    }
}

pub fn gen_all_moves(board: &Board) -> MoveList {
    let mut moves = MoveList::default();
    let ptr = match (board.to_play, board.in_check()) {
        (Color::White, true) => generate_moves::<White, Evasions>(board, moves.as_ptr()),
        (Color::White, false) => generate_moves::<White, NonEvasions>(board, moves.as_ptr()),
        (Color::Black, true) => generate_moves::<Black, Evasions>(board, moves.as_ptr()),
        (Color::Black, false) => generate_moves::<Black, NonEvasions>(board, moves.as_ptr()),
    };
    moves.update_size(ptr);
    moves
}

pub fn gen_moves<Us: Player, Type: MoveGenType>(board: &Board) -> MoveList {
    let mut moves = MoveList::default();
    let ptr = generate_moves::<Us, Type>(board, moves.as_ptr());
    moves.update_size(ptr);
    moves
}
