use crate::{board::Board, move_gen::gen_all_moves};

pub fn perft(board: &mut Board, depth: u8) -> u64 {
    if depth == 0 {
        return 1;
    }

    let moves = gen_all_moves(board);
    let mut nodes = 0;
    for scored_move in moves.as_slice() {
        let mov = scored_move.mov;
        if !board.legal(mov) {
            continue;
        }

        if depth == 1 {
            nodes += 1;
            continue;
        }

        board.make_move(mov);
        nodes += perft(board, depth - 1);
        board.undo_move(mov);
    }

    nodes
}
