use chess_core::{bitboard::*, for_each_bit, prelude::*};

use crate::{
    attacks::{bishop_attacks, king_attacks, knight_attacks, queen_attacks, rook_attacks},
    board::Board,
    move_gen::MoveListPtr,
};

use crate::move_gen::traits::*;

pub fn generate_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    mut moves: MoveListPtr,
) -> MoveListPtr {
    let us = Us::COLOR;
    let them = !us;

    let king_bb = board.color_piece(Piece::King, us);
    let king_sq = unsafe { bb_lsb(king_bb) };

    if Type::EVASIONS && !board.in_check() {
        return moves;
    }

    moves = generate_king_moves::<Us, Type>(board, king_sq, moves);

    // Double check: Only king can move
    let checkers = board.checkers();
    if Type::EVASIONS && bb_several(checkers) {
        return moves;
    }

    let target_mask = if Type::EVASIONS {
        let checker_sq = unsafe { bb_lsb(checkers) };
        checkers | bb_between(king_sq, checker_sq)
    } else {
        let mut mask = 0;
        if Type::CAPTURES {
            mask |= board.colors(them);
        }
        if Type::QUIETS {
            mask |= !board.occupied();
        }
        mask
    };

    moves = generate_pieces_moves::<Us, Type>(board, target_mask, moves);
    moves = generate_pawn_moves::<Us, Type>(board, target_mask, moves);

    if Type::QUIETS && !Type::EVASIONS {
        moves = generate_castling_moves::<Us>(board, moves);
    }

    moves
}

const KNIGHT: u8 = Piece::Knight as u8;
const BISHOP: u8 = Piece::Bishop as u8;
const ROOK: u8 = Piece::Rook as u8;
const QUEEN: u8 = Piece::Queen as u8;

#[inline(always)]
pub fn generate_piece_moves<const PIECE: u8, Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    mut moves: MoveListPtr,
) -> MoveListPtr {
    let us = Us::COLOR;
    let enemy = board.colors(!us);
    let occupied = board.occupied();

    let piece = match PIECE {
        KNIGHT => Piece::Knight,
        BISHOP => Piece::Bishop,
        ROOK => Piece::Rook,
        QUEEN => Piece::Queen,
        _ => unsafe { std::hint::unreachable_unchecked() },
    };

    for_each_bit!(from in board.color_piece(piece, us) => {
        let attacks = match PIECE {
            KNIGHT => knight_attacks(from),
            BISHOP => bishop_attacks(from, occupied),
            ROOK => rook_attacks(from, occupied),
            QUEEN => queen_attacks(from, occupied),
            _ => 0,
        } & target_mask;

        if !Type::QUIETS {
            for_each_bit!(to in attacks => { moves.push(from, to, MoveFlags::Capture) });
        } else if !Type::CAPTURES {
            for_each_bit!(to in attacks => { moves.push(from, to, MoveFlags::Quiet) });
        } else {
            let captures = attacks & enemy;
            let quiets = attacks & !enemy;

            for_each_bit!(to in captures => { moves.push(from, to, MoveFlags::Capture) });
            for_each_bit!(to in quiets => { moves.push(from, to, MoveFlags::Quiet) });
        }
    });

    moves
}

#[inline(always)]
pub fn generate_knight_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    moves: MoveListPtr,
) -> MoveListPtr {
    generate_piece_moves::<KNIGHT, Us, Type>(board, target_mask, moves)
}

#[inline(always)]
pub fn generate_bishop_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    moves: MoveListPtr,
) -> MoveListPtr {
    generate_piece_moves::<BISHOP, Us, Type>(board, target_mask, moves)
}

#[inline(always)]
pub fn generate_rook_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    moves: MoveListPtr,
) -> MoveListPtr {
    generate_piece_moves::<ROOK, Us, Type>(board, target_mask, moves)
}

#[inline(always)]
pub fn generate_queen_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    moves: MoveListPtr,
) -> MoveListPtr {
    generate_piece_moves::<QUEEN, Us, Type>(board, target_mask, moves)
}

#[inline(always)]
pub fn generate_pieces_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    mut moves: MoveListPtr,
) -> MoveListPtr {
    moves = generate_knight_moves::<Us, Type>(board, target_mask, moves);
    moves = generate_bishop_moves::<Us, Type>(board, target_mask, moves);
    moves = generate_rook_moves::<Us, Type>(board, target_mask, moves);
    moves = generate_queen_moves::<Us, Type>(board, target_mask, moves);
    moves
}

pub fn generate_pawn_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    target_mask: u64,
    mut moves: MoveListPtr,
) -> MoveListPtr {
    let us = Us::COLOR;
    let them = !us;
    let forward = if us == Color::White {
        Dir::North
    } else {
        Dir::South
    };
    let backward = forward.opposite();
    let last_rank = if us == Color::White { RANK_8 } else { RANK_1 };
    let third_rank = if us == Color::White { RANK_3 } else { RANK_6 };
    let backward_left = if us == Color::White {
        Dir::SouthEast
    } else {
        Dir::NorthWest
    };
    let backward_right = if us == Color::White {
        Dir::SouthWest
    } else {
        Dir::NorthEast
    };

    let empty_squares = !board.occupied();
    let enemies = board.colors(them) & target_mask;
    let pawns = board.color_piece(Piece::Pawn, us);

    if Type::CAPTURES {
        let (left, right) = if Us::COLOR == Color::White {
            (sh_north_west(pawns), sh_north_east(pawns))
        } else {
            (sh_south_east(pawns), sh_south_west(pawns))
        };

        if let Some(en_passant_sq) = board.en_passant_target_sq() {
            let captured_pawn_sq = unsafe { en_passant_sq.shift(backward) };
            let ep_mask = en_passant_sq.bitboard() | captured_pawn_sq.bitboard();
            let en_passant_bb = if ep_mask & target_mask != 0 {
                en_passant_sq.bitboard()
            } else {
                0
            };

            if left & en_passant_bb != 0 {
                let from = unsafe { en_passant_sq.shift(backward_left) };
                moves.push(from, en_passant_sq, MoveFlags::EnPassant);
            }

            if right & en_passant_bb != 0 {
                let from = unsafe { en_passant_sq.shift(backward_right) };
                moves.push(from, en_passant_sq, MoveFlags::EnPassant);
            }
        }

        let left = left & enemies;
        let right = right & enemies;

        if Type::EVASIONS {
            let left_non_promo = left & !last_rank;
            if left_non_promo != 0 {
                let to = unsafe { Sq::from_raw_unchecked(left_non_promo.trailing_zeros() as u8) };
                let from = unsafe { to.shift(backward_left) };
                moves.push(from, to, MoveFlags::Capture);
            }

            let right_non_promo = right & !last_rank;
            if right_non_promo != 0 {
                let to = unsafe { Sq::from_raw_unchecked(right_non_promo.trailing_zeros() as u8) };
                let from = unsafe { to.shift(backward_right) };
                moves.push(from, to, MoveFlags::Capture);
            }

            let left_promo = left & last_rank;
            if left_promo != 0 {
                let to = unsafe { Sq::from_raw_unchecked(left_promo.trailing_zeros() as u8) };
                let from = unsafe { to.shift(backward_left) };
                moves.push_promotions(from, to, true);
            }

            let right_promo = right & last_rank;
            if right_promo != 0 {
                let to = unsafe { Sq::from_raw_unchecked(right_promo.trailing_zeros() as u8) };
                let from = unsafe { to.shift(backward_right) };
                moves.push_promotions(from, to, true);
            }
        } else {
            // Non-promotions
            for_each_bit!(to in left & !last_rank => {
                let from = unsafe { to.shift(backward_left) };
                moves.push(from, to, MoveFlags::Capture);
            });

            for_each_bit!(to in right & !last_rank => {
                let from = unsafe { to.shift(backward_right) };
                moves.push(from, to, MoveFlags::Capture);
            });

            // Promotions
            for_each_bit!(to in left & last_rank => {
                let from = unsafe { to.shift(backward_left) };
                moves.push_promotions(from, to, true);
            });

            for_each_bit!(to in right & last_rank => {
                let from = unsafe { to.shift(backward_right) };
                moves.push_promotions(from, to, true);
            });
        }
    }

    if Type::QUIETS {
        let forward_pushed_pawn = sh_dir(forward, pawns) & empty_squares;
        let promotion_pushes = forward_pushed_pawn & last_rank & target_mask;
        let pushes = forward_pushed_pawn & !last_rank & target_mask;
        let double_pushes =
            sh_dir(forward, forward_pushed_pawn & third_rank) & empty_squares & target_mask;

        for_each_bit!(to in promotion_pushes => {
            let from = unsafe { to.shift(backward) };
            moves.push_promotions(from, to, false);
        });

        for_each_bit!(to in pushes => {
            let from = unsafe { to.shift(backward) };
            moves.push(from, to, MoveFlags::Quiet);
        });

        for_each_bit!(to in double_pushes => {
            let from = unsafe { to.shift(backward).shift(backward) };
            moves.push(from, to, MoveFlags::DoublePawn);
        });
    }

    moves
}

#[inline]
pub fn generate_king_moves<Us: Player, Type: MoveGenType>(
    board: &Board,
    king_sq: Sq,
    mut moves: MoveListPtr,
) -> MoveListPtr {
    let us = Us::COLOR;
    let them = !us;

    let enemy = board.colors(them);

    let mut target_mask = !board.colors(us);
    if !Type::EVASIONS {
        let mut mask = 0;
        if Type::CAPTURES {
            mask |= board.colors(them);
        }
        if Type::QUIETS {
            mask |= !board.occupied();
        }
        target_mask &= mask;
    }
    let attacks = king_attacks(king_sq) & target_mask;

    let captures = attacks & enemy;
    let quiets = attacks & !enemy;

    if Type::CAPTURES {
        for_each_bit!(to in captures => {
            moves.push(king_sq, to, MoveFlags::Capture);
        });
    }

    if Type::QUIETS {
        for_each_bit!(to in quiets => {
            moves.push(king_sq, to, MoveFlags::Quiet);
        });
    }

    moves
}

pub fn generate_castling_moves<Us: Player>(board: &Board, mut moves: MoveListPtr) -> MoveListPtr {
    let us = Us::COLOR;
    let occupied = board.occupied();

    let (king_from, ks_to, qs_to, ks_path, qs_path, ks_right, qs_right) = if us == Color::White {
        (
            Sq::E1,
            Sq::G1,
            Sq::C1,
            Sq::F1.bitboard() | Sq::G1.bitboard(),
            Sq::B1.bitboard() | Sq::C1.bitboard() | Sq::D1.bitboard(),
            CastlingRights::WHITE_00,
            CastlingRights::WHITE_000,
        )
    } else {
        (
            Sq::E8,
            Sq::G8,
            Sq::C8,
            Sq::F8.bitboard() | Sq::G8.bitboard(),
            Sq::B8.bitboard() | Sq::C8.bitboard() | Sq::D8.bitboard(),
            CastlingRights::BLACK_00,
            CastlingRights::BLACK_000,
        )
    };

    let rights = board.castling_rights();
    if rights.contains(ks_right) && (occupied & ks_path) == 0 {
        moves.push(king_from, ks_to, MoveFlags::CastleKing);
    }

    if rights.contains(qs_right) && (occupied & qs_path) == 0 {
        moves.push(king_from, qs_to, MoveFlags::CastleQueen);
    }

    moves
}
