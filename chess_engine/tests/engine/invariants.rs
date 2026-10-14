//! Walks the game tree and checks that every incremental update (make/undo, hash,
//! checkers, NNUE accumulators) matches the same position built from scratch

use crate::common::{PERFT_SUITE, board, legal_moves};
use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::move_gen::{
    Black, Captures, MoveGenType, NonEvasions, Player, Quiets, White, gen_moves,
};
use chess_engine::nnue::{AccumulatorStack, FinnyTable, evaluate};
use chess_engine::search::SearchStack;

fn assert_matches_fen(board: &Board) {
    let fen = board.to_fen();
    let fresh = Board::from_fen(&fen).unwrap();
    assert_eq!(board.hash(), fresh.hash(), "hash of {fen}");
    assert_eq!(board.checkers(), fresh.checkers(), "checkers of {fen}");
    assert_eq!(board.in_check(), board.checkers() != 0, "{fen}");
    for color in [Color::White, Color::Black] {
        assert_eq!(board.king_sq(color), fresh.king_sq(color), "{fen}");
        for piece in Piece::ALL {
            assert_eq!(
                board.color_piece(piece, color),
                fresh.color_piece(piece, color),
                "{color:?} {piece:?} bitboard of {fen}"
            );
        }
    }
    assert_eq!(
        board.occupied(),
        board.colors(Color::White) | board.colors(Color::Black),
        "{fen}"
    );
}

fn moves<Us: Player, Type: MoveGenType>(board: &Board) -> Vec<Move> {
    let mut moves: Vec<Move> = gen_moves::<Us, Type>(board)
        .as_slice()
        .iter()
        .map(|m| m.mov)
        .collect();
    moves.sort_by_key(|m| m.bits());
    moves
}

/// Captures and quiets split the non-evasions, and every generated move is pseudo-legal
fn assert_gen_types<Us: Player>(board: &Board) {
    if board.in_check() {
        return;
    }
    let fen = board.to_fen();
    let captures = moves::<Us, Captures>(board);
    let quiets = moves::<Us, Quiets>(board);
    let all = moves::<Us, NonEvasions>(board);

    assert!(captures.iter().all(|m| m.is_capture()), "{fen}");
    assert!(quiets.iter().all(|m| !m.is_capture()), "{fen}");
    let mut split = [captures, quiets].concat();
    split.sort_by_key(|m| m.bits());
    assert_eq!(split, all, "{fen}");

    for &mov in &all {
        assert!(board.pseudo_legal(mov), "{mov:?} in {fen}");
    }
}

struct Walker {
    board: Board,
    stack: SearchStack,
    nnue: AccumulatorStack,
    finny: FinnyTable,
    evals: usize,
}

impl Walker {
    fn new(fen: &str) -> Self {
        let board = board(fen);
        let mut stack = SearchStack::default();
        stack[0].acc_computed = [true; Color::NB];
        Self {
            nnue: AccumulatorStack::new(&board),
            board,
            stack,
            finny: FinnyTable::default(),
            evals: 0,
        }
    }

    fn check_eval(&mut self, ply: u16) {
        // Evaluate only some nodes so that multi-ply accumulator updates are exercised too
        let sampled = if cfg!(miri) {
            self.evals == 0 && ply > 0
        } else {
            self.board.hash().is_multiple_of(3)
        };
        if !sampled {
            return;
        }
        let fen = self.board.to_fen();
        let expected = evaluate(&self.board);
        let stacked = self.nnue.eval(&self.board, &mut self.stack, ply);
        assert_eq!(stacked, expected, "accumulator stack eval of {fen}");
        assert_eq!(
            self.finny.eval(&self.board),
            expected,
            "Finny eval of {fen}"
        );
        self.evals += 1;
    }

    fn visit(&mut self, ply: u16, depth: u8) {
        assert_matches_fen(&self.board);
        match self.board.to_play {
            Color::White => assert_gen_types::<White>(&self.board),
            Color::Black => assert_gen_types::<Black>(&self.board),
        }
        self.check_eval(ply);
        if depth == 0 {
            return;
        }

        let before = (
            self.board.to_fen(),
            self.board.hash(),
            self.board.checkers(),
        );
        for mov in legal_moves(&self.board) {
            let moved_piece = self.board.piece_at(mov.from());
            let captured_piece = self.board.piece_at(mov.capture_square(self.board.to_play));
            self.board.make_move(mov);
            self.stack[ply + 1].set_move(mov, moved_piece, captured_piece);
            self.visit(ply + 1, depth - 1);
            self.board.undo_move(mov);

            let after = (
                self.board.to_fen(),
                self.board.hash(),
                self.board.checkers(),
            );
            assert_eq!(after, before, "undo of {mov:?}");
        }

        if !self.board.in_check() {
            self.board.make_null_move();
            self.stack[ply + 1].set_null_move();
            self.visit(ply + 1, depth - 1);
            self.board.undo_null_move();

            let after = (
                self.board.to_fen(),
                self.board.hash(),
                self.board.checkers(),
            );
            assert_eq!(after, before, "undo of null move");
        }
    }
}

fn walk(fen: &str, depth: u8) {
    let mut walker = Walker::new(fen);
    walker.visit(0, depth);
    assert!(walker.evals > 0, "no evaluated nodes for {fen}");
}

#[test]
fn test_perft_suite_invariants() {
    let (count, depth) = if cfg!(miri) {
        (2, 1)
    } else {
        (PERFT_SUITE.len(), 3)
    };
    for &(fen, _) in &PERFT_SUITE[..count] {
        walk(fen, depth);
    }
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_special_position_invariants() {
    for fen in [
        "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq - 0 1",
        // Promotions
        "1r5k/P1P5/8/8/8/8/3K2p1/5R1R b - - 0 1",
        // En passant
        "rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3",
        "rnbqkbnr/pppp1ppp/8/8/3PpP2/8/PPP1P1PP/RNBQKBNR b KQkq d3 0 3",
        "8/8/8/KPp4r/8/8/8/7k w - c6 0 1",
        // Central kings cross the mirror line and bucket boundaries on almost every move
        "8/3p4/4k3/8/8/3K4/4P3/R6r w - - 0 1",
        "8/8/2n1k3/3p4/4P3/3K1N2/8/8 b - - 0 1",
    ] {
        walk(fen, 3);
    }
}
