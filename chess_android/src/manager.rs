use chess_core::prelude::*;
use chess_engine::{board::Board, nnue::evaluate, search::piece_value};

use crate::{
    notation,
    rules::{self, GameState},
};

const NO_PIECE: u8 = 255;

#[derive(Clone, Debug)]
pub struct BoardSnapshot {
    pub game_state: i32,
    pub eval_score: i32,
    pub search_time_ms: i64,
    pub uci_info: String,
    pub is_white_turn: bool,
    pub current_move_index: i32,
    pub pieces: Vec<i32>,
    pub moves_history: Vec<i32>,
    pub moves_san: Vec<String>,
    pub legal_moves: Vec<i32>,
}

#[derive(Clone)]
struct Entry {
    mov: Move,
    san: String,
    board_before: Board,
}

pub struct ChessGame {
    start: Board,
    board: Board,
    history: Vec<Entry>,
    redo: Vec<Entry>,
    version: u64,
}

impl ChessGame {
    pub fn new() -> Self {
        Self::from_board(Board::start_pos())
    }

    fn from_board(board: Board) -> Self {
        Self {
            start: board.clone(),
            board,
            history: Vec::with_capacity(64),
            redo: Vec::new(),
            version: 0,
        }
    }

    pub fn load(fen: &str, uci_moves: &[&str]) -> Option<Self> {
        let board = Board::from_fen(fen).filter(rules::is_valid_position)?;
        let mut game = Self::from_board(board);
        for uci_move in uci_moves {
            let mov = notation::find_move(&game.board, uci_move)?;
            if game.game_state().is_game_over() || !game.try_apply(mov) {
                return None;
            }
        }
        Some(game)
    }

    pub fn replace(&mut self, other: ChessGame) {
        let version = self.version + 1;
        *self = other;
        self.version = version;
    }

    pub fn version(&self) -> u64 {
        self.version
    }

    pub fn load_save(save: &str) -> Option<Self> {
        let (fen, moves) = notation::parse_save(save);
        Self::load(fen, &moves)
    }

    pub fn save(&self) -> String {
        notation::format_save(&self.start, self.history.iter().map(|e| e.mov))
    }

    pub fn board(&self) -> &Board {
        &self.board
    }

    pub fn position_keys(&self) -> Vec<u64> {
        self.history
            .iter()
            .map(|entry| entry.board_before.hash())
            .chain([self.board.hash()])
            .collect()
    }

    pub fn game_state(&self) -> GameState {
        let history_hashes: Vec<u64> = self.history.iter().map(|e| e.board_before.hash()).collect();
        rules::calculate_game_state(&self.board, &history_hashes)
    }

    fn try_apply(&mut self, mov: Move) -> bool {
        if !self.board.pseudo_legal(mov) || !self.board.legal(mov) {
            return false;
        }

        let san = notation::san(&self.board, mov);
        let board_before = self.board.clone();
        self.board.play_move(mov);
        self.history.push(Entry {
            mov,
            san,
            board_before,
        });
        self.version += 1;
        true
    }

    pub fn make_move(&mut self, move_bits: i32) -> bool {
        let Some(mov) = u16::try_from(move_bits).ok().and_then(Move::from_bits) else {
            return false;
        };
        if self.game_state().is_game_over() || !self.try_apply(mov) {
            return false;
        }
        self.redo.clear();
        true
    }

    fn undo_ply(&mut self) -> bool {
        let Some(entry) = self.history.pop() else {
            return false;
        };
        self.board = entry.board_before.clone();
        self.redo.push(entry);
        self.version += 1;
        true
    }

    fn redo_ply(&mut self) -> bool {
        self.redo
            .pop()
            .is_some_and(|entry| self.try_apply(entry.mov))
    }

    pub fn undo(&mut self, player: Color) -> bool {
        if !self.undo_ply() {
            return false;
        }
        while self.board.to_play != player && self.undo_ply() {}
        true
    }

    pub fn redo(&mut self, player: Color) -> bool {
        if !self.redo_ply() {
            return false;
        }
        while self.board.to_play != player && self.redo_ply() {}
        true
    }

    fn piece_ids(&self) -> [u8; 64] {
        let mut ids = [NO_PIECE; 64];
        for (sq, piece) in self.start.mailbox.iter().enumerate() {
            if piece.is_some() {
                ids[sq] = sq as u8;
            }
        }

        let mut color = self.start.to_play;
        for entry in &self.history {
            let mov = entry.mov;
            let from = mov.from() as usize;
            let to = mov.to() as usize;

            if mov.flags() == MoveFlags::EnPassant {
                ids[mov.capture_square(color) as usize] = NO_PIECE;
            }
            ids[to] = ids[from];
            ids[from] = NO_PIECE;

            if mov.is_castle() {
                let (rook_from, rook_to) = mov.castling_rook_squares(color);
                ids[rook_to as usize] = ids[rook_from as usize];
                ids[rook_from as usize] = NO_PIECE;
            }
            color = !color;
        }
        ids
    }

    pub fn snapshot(&self, search_time_ms: u64, uci_info: String) -> BoardSnapshot {
        let piece_ids = self.piece_ids();
        let pieces = (0..64)
            .filter_map(|sq| {
                let cp = self.board.mailbox[sq]?;
                let id = piece_ids[sq] as i32;
                let piece_type = cp.piece() as i32;
                let is_white = (cp.color() == Color::White) as i32;
                Some(id | ((sq as i32) << 8) | (piece_type << 16) | (is_white << 24))
            })
            .collect();

        let game_state = self.game_state();
        let legal_moves = if game_state.is_game_over() {
            Vec::new()
        } else {
            rules::legal_moves(&self.board)
                .into_iter()
                .map(|m| m.bits() as i32)
                .collect()
        };

        let all_entries = || self.history.iter().chain(self.redo.iter().rev());
        BoardSnapshot {
            game_state: game_state as i32,
            eval_score: self.get_board_evaluation(),
            search_time_ms: search_time_ms as i64,
            uci_info,
            is_white_turn: self.board.to_play == Color::White,
            current_move_index: self.history.len() as i32 - 1,
            pieces,
            moves_history: all_entries().map(|e| e.mov.bits() as i32).collect(),
            moves_san: all_entries().map(|e| e.san.clone()).collect(),
            legal_moves,
        }
    }

    pub fn pgn(&self, date: &str, player_is_white: bool) -> String {
        let sans: Vec<&str> = self.history.iter().map(|e| e.san.as_str()).collect();
        notation::pgn(&self.start, &sans, self.game_state(), date, player_is_white)
    }

    pub fn get_current_fen(&self) -> String {
        self.board.to_fen()
    }

    pub fn get_board_evaluation(&self) -> i32 {
        let raw = evaluate(&self.board) as i32;
        let white_score = if self.board.to_play == Color::White {
            raw
        } else {
            -raw
        };
        white_score * 100 / (piece_value(Piece::Pawn) as i32)
    }
}
