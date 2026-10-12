use chess_core::{
    bitboard::{RANK_1, RANK_8},
    prelude::*,
};
use chess_engine::{board::Board, move_gen::gen_all_moves};

/// Order must match `GameState.kt`, which decodes the ordinal
#[repr(i32)]
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum GameState {
    None,
    WinnerWhite,
    WinnerBlack,
    Draw,
    WhiteInCheck,
    BlackInCheck,
}

impl GameState {
    pub fn is_game_over(self) -> bool {
        matches!(self, Self::WinnerWhite | Self::WinnerBlack | Self::Draw)
    }
}

/// One king per side, and the side that just moved must not be left in check
pub fn is_valid_position(board: &Board) -> bool {
    let one_king = |color| board.color_piece(Piece::King, color).count_ones() == 1;
    if !one_king(Color::White) || !one_king(Color::Black) {
        return false;
    }

    if board.pieces(Piece::Pawn) & (RANK_1 | RANK_8) != 0 || !is_valid_en_passant(board) {
        return false;
    }

    let us = board.to_play;
    board.generate_attackers(board.king_sq(!us), us, board.occupied()) == 0
}

fn is_valid_en_passant(board: &Board) -> bool {
    let Some(ep) = board.en_passant_target_sq() else {
        return true;
    };
    let (ep_rank, pawn_rank, origin_rank) = match board.to_play {
        Color::White => (5, 4, 6),
        Color::Black => (2, 3, 1),
    };
    if ep.rank() != ep_rank {
        return false;
    }

    let is_empty = |rank| Sq::new(ep.file(), rank).is_some_and(|sq| board.piece_at(sq).is_none());
    let has_enemy_pawn = Sq::new(ep.file(), pawn_rank)
        .and_then(|sq| board.piece_at(sq))
        .is_some_and(|cp| cp.piece() == Piece::Pawn && cp.color() != board.to_play);
    is_empty(ep_rank) && is_empty(origin_rank) && has_enemy_pawn
}

pub fn legal_moves(board: &Board) -> Vec<Move> {
    gen_all_moves(board)
        .as_slice()
        .iter()
        .map(|sm| sm.mov)
        .filter(|&m| board.legal(m))
        .collect()
}

pub fn calculate_game_state(board: &Board, history_hashes: &[u64]) -> GameState {
    let has_moves = !legal_moves(board).is_empty();
    let is_mate = !has_moves && board.in_check();
    let repetitions = history_hashes
        .iter()
        .filter(|&&h| h == board.hash())
        .count();
    if !is_mate && (board.is_draw() || repetitions >= 2) {
        return GameState::Draw;
    }

    match (has_moves, board.in_check(), board.to_play) {
        (false, false, _) => GameState::Draw,
        (false, true, Color::White) => GameState::WinnerBlack,
        (false, true, Color::Black) => GameState::WinnerWhite,
        (true, true, Color::White) => GameState::WhiteInCheck,
        (true, true, Color::Black) => GameState::BlackInCheck,
        (true, false, _) => GameState::None,
    }
}
