use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::move_gen::{Black, Evasions, NonEvasions, White, gen_moves};
use chess_engine::nnue::{AccumulatorStack, evaluate};
use chess_engine::search::SearchStack;

fn legal_moves(board: &Board) -> Vec<Move> {
    let moves = match (board.to_play, board.checkers() != 0) {
        (Color::White, true) => gen_moves::<White, Evasions>(board),
        (Color::White, false) => gen_moves::<White, NonEvasions>(board),
        (Color::Black, true) => gen_moves::<Black, Evasions>(board),
        (Color::Black, false) => gen_moves::<Black, NonEvasions>(board),
    };
    moves
        .as_slice()
        .iter()
        .map(|m| m.mov)
        .filter(|&m| board.legal(m))
        .collect()
}

struct Walker {
    board: Board,
    stack: SearchStack,
    nnue: AccumulatorStack,
    evals: usize,
}

impl Walker {
    fn check(&mut self, ply: u16) {
        // Evaluate only some nodes so that multi-ply replays are exercised too.
        if self.board.hash().is_multiple_of(3) {
            let expected = evaluate(&self.board);
            let got = self.nnue.eval(&self.board, &mut self.stack, ply);
            assert_eq!(got, expected, "eval mismatch at ply {ply}");
            self.evals += 1;
        }
    }

    fn dfs(&mut self, ply: u16, depth: u8) {
        self.check(ply);
        if depth == 0 {
            return;
        }
        for mov in legal_moves(&self.board) {
            let moved_piece = self.board.piece_at(mov.from());
            let captured_piece = self.board.piece_at(mov.capture_square(self.board.to_play));
            self.board.make_move(mov);
            self.stack[ply + 1].set_move(mov, moved_piece, captured_piece);
            self.dfs(ply + 1, depth - 1);
            self.board.undo_move(mov);
        }
        if self.board.checkers() == 0 {
            self.board.make_null_move();
            self.stack[ply + 1].set_null_move();
            self.dfs(ply + 1, depth - 1);
            self.board.undo_null_move();
        }
    }
}

fn check_fen(fen: &str) {
    let board = Board::from_fen(fen).unwrap();
    let mut stack = SearchStack::default();
    stack[0].acc_computed = [true; Color::NB];
    let mut walker = Walker {
        nnue: AccumulatorStack::new(&board),
        board,
        stack,
        evals: 0,
    };
    walker.dfs(0, 3);
    assert!(walker.evals > 0);
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_stack_eval_kiwipete() {
    check_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1");
    check_fen("r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq - 0 1");
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_stack_eval_promotions() {
    check_fen("r3k2r/Pppp1ppp/1b3nbN/nP6/BBP1P3/q4N2/Pp1P2PP/R2Q1RK1 w kq - 0 1");
    check_fen("1r5k/P1P5/8/8/8/8/3K2p1/5R1R b - - 0 1");
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_stack_eval_en_passant() {
    check_fen("rnbqkbnr/ppp1p1pp/8/3pPp2/8/8/PPPP1PPP/RNBQKBNR w KQkq f6 0 3");
    check_fen("rnbqkbnr/pppp1ppp/8/8/3PpP2/8/PPP1P1PP/RNBQKBNR b KQkq d3 0 3");
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_stack_eval_king_buckets() {
    // Central kings cross the mirror line and bucket boundaries on almost every move.
    check_fen("8/3p4/4k3/8/8/3K4/4P3/R6r w - - 0 1");
    check_fen("8/8/2n1k3/3p4/4P3/3K1N2/8/8 b - - 0 1");
}
