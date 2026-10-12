use std::str::FromStr;

use chess_core::prelude::*;

use super::{Board, BoardState};
use crate::zobrist::ZOBRIST_KEYS;

pub const START_POS_FEN: &str = "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1";

pub fn parse_fen(fen: &str) -> Option<Board> {
    let mut board = Board {
        mailbox: [None; Sq::NB],
        bit_colors: [0; Color::NB],
        bit_pieces: [0; Piece::NB],
        to_play: Color::White,
        ply: 0,
        state_index: 0,
        state_array: std::array::from_fn(|_| BoardState::default()),
    };

    let mut tokens = fen.split_ascii_whitespace();
    let piece_placement = tokens.next()?;
    let side_to_move = tokens.next();
    let castling = tokens.next();
    let en_passant = tokens.next();
    let half_move = tokens.next();
    let full_move = tokens.next();
    if tokens.next().is_some() {
        return None;
    }

    let mut ranks = piece_placement.split('/');
    for rank in (0..8u8).rev() {
        let mut file = 0;
        let mut prev_digit = false;
        for ch in ranks.next()?.chars() {
            if let Some(empty) = ch.to_digit(10).filter(|d| (1..=8).contains(d)) {
                if prev_digit {
                    return None;
                }
                prev_digit = true;
                file += empty as u8;
            } else {
                prev_digit = false;
                board.add_piece::<true>(Sq::new(file, rank)?, ColoredPiece::parse(ch)?);
                file += 1;
            }
            if file > 8 {
                return None;
            }
        }
        if file != 8 {
            return None;
        }
    }
    if ranks.next().is_some() {
        return None;
    }

    board.to_play = match side_to_move {
        None | Some("w") => Color::White,
        Some("b") => Color::Black,
        _ => return None,
    };

    let mut castling_rights = CastlingRights::empty();
    if let Some(castling_str) = castling.filter(|&c| c != "-") {
        for ch in castling_str.chars() {
            castling_rights |= match ch {
                'K' => CastlingRights::WHITE_00,
                'Q' => CastlingRights::WHITE_000,
                'k' => CastlingRights::BLACK_00,
                'q' => CastlingRights::BLACK_000,
                _ => return None,
            };
        }
    }

    // Drop rights whose king or rook has left its home square
    for (rights, color, king, rook) in [
        (CastlingRights::WHITE_00, Color::White, Sq::E1, Sq::H1),
        (CastlingRights::WHITE_000, Color::White, Sq::E1, Sq::A1),
        (CastlingRights::BLACK_00, Color::Black, Sq::E8, Sq::H8),
        (CastlingRights::BLACK_000, Color::Black, Sq::E8, Sq::A8),
    ] {
        if board.piece_at(king) != Some(ColoredPiece::new(Piece::King, color))
            || board.piece_at(rook) != Some(ColoredPiece::new(Piece::Rook, color))
        {
            castling_rights.remove(rights);
        }
    }

    let mut en_passant_target_sq = None;
    if let Some(en_passant_sq) = en_passant.filter(|&e| e != "-") {
        if en_passant_sq.len() != 2 {
            return None;
        }
        let sq = Sq::parse(en_passant_sq)?;
        let valid_rank = if board.to_play == Color::White { 5 } else { 2 };
        if sq.rank() == valid_rank {
            let attackers = crate::attacks::pawn_attacks(sq, !board.to_play)
                & board.color_piece(Piece::Pawn, board.to_play);
            if attackers != 0 {
                en_passant_target_sq = Some(sq);
            }
        }
    }

    let half_move_clock = match half_move {
        Some(val) => val.parse::<u32>().ok()?.min(u8::MAX.into()) as u8,
        None => 0,
    };

    let counter = match full_move {
        Some(val) => val.parse::<u16>().ok()?,
        None => 1,
    };
    board.ply = (counter.saturating_sub(1)) * 2 + if board.to_play == Color::Black { 1 } else { 0 };

    let to_play = board.to_play;
    let state = board.state_mut();
    state.castling_rights = castling_rights;
    state.en_passant_target_sq = en_passant_target_sq;
    state.half_move_clock = half_move_clock;

    // Piece keys were already added by add_piece
    if to_play == Color::White {
        state.hash ^= ZOBRIST_KEYS.side();
    }
    state.hash ^= ZOBRIST_KEYS.castling(castling_rights);
    if let Some(ep_sq) = en_passant_target_sq {
        state.hash ^= ZOBRIST_KEYS.en_passant(ep_sq);
    }

    if board.color_piece(Piece::King, to_play) != 0 {
        board.set_checkers();
        board.set_pinned();
    }

    Some(board)
}

pub fn format_fen(board: &Board) -> String {
    let mut fen = String::with_capacity(90);

    // 1. Piece placement
    for rank in (0..8u8).rev() {
        let mut empty_count = 0;
        for file in 0..8u8 {
            let sq = Sq::new(file, rank).unwrap();
            if let Some(cp) = board.piece_at(sq) {
                if empty_count > 0 {
                    fen.push((b'0' + empty_count) as char);
                    empty_count = 0;
                }
                fen.push(cp.to_char());
            } else {
                empty_count += 1;
            }
        }
        if empty_count > 0 {
            fen.push((b'0' + empty_count) as char);
        }
        if rank > 0 {
            fen.push('/');
        }
    }

    // 2. Active color
    fen.push(' ');
    fen.push(if board.to_play == Color::White {
        'w'
    } else {
        'b'
    });

    // 3. Castling availability
    fen.push(' ');
    let state = board.state();
    let mut any_castling = false;
    if state.castling_rights.contains(CastlingRights::WHITE_00) {
        fen.push('K');
        any_castling = true;
    }
    if state.castling_rights.contains(CastlingRights::WHITE_000) {
        fen.push('Q');
        any_castling = true;
    }
    if state.castling_rights.contains(CastlingRights::BLACK_00) {
        fen.push('k');
        any_castling = true;
    }
    if state.castling_rights.contains(CastlingRights::BLACK_000) {
        fen.push('q');
        any_castling = true;
    }
    if !any_castling {
        fen.push('-');
    }

    // 4. En passant target square
    fen.push(' ');
    if let Some(sq) = state.en_passant_target_sq {
        fen.push_str(&sq.to_string());
    } else {
        fen.push('-');
    }

    // 5. Halfmove clock
    fen.push(' ');
    fen.push_str(&state.half_move_clock.to_string());

    // 6. Fullmove counter
    fen.push(' ');
    let full_move = (board.ply / 2) + 1;
    fen.push_str(&full_move.to_string());

    fen
}

impl FromStr for Board {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        parse_fen(s).ok_or(())
    }
}
