use chess_core::prelude::*;
use chess_engine::{board::Board, uci::format_move};

use crate::rules::{GameState, legal_moves};

const SITE: &str = "https://github.com/razvanfilea/ChessEngine";
const PLAYER: &str = "Player";
const ENGINE: &str = "LuckyChess";
const MAX_LINE_LEN: usize = 80;

pub fn san(board: &Board, mov: Move) -> String {
    let mut san = san_without_check(board, mov);
    let mut after = board.clone();
    after.make_move(mov);
    if after.in_check() {
        san.push(if legal_moves(&after).is_empty() {
            '#'
        } else {
            '+'
        });
    }
    san
}

/// Standard Algebraic Notation of `mov`, which must be legal in `board`, without the check suffix
fn san_without_check(board: &Board, mov: Move) -> String {
    match mov.flags() {
        MoveFlags::CastleKing => return "O-O".to_string(),
        MoveFlags::CastleQueen => return "O-O-O".to_string(),
        _ => {}
    }

    let from = mov.from();
    let piece = board.piece_at(from).map_or(Piece::Pawn, |cp| cp.piece());
    let mut san = String::with_capacity(8);

    if piece == Piece::Pawn {
        if mov.is_capture() {
            san.push((b'a' + from.file()) as char);
        }
    } else {
        san.push(piece_letter(piece));

        let rivals: Vec<Sq> = legal_moves(board)
            .into_iter()
            .filter(|m| m.to() == mov.to() && m.from() != from)
            .map(|m| m.from())
            .filter(|&sq| board.piece_at(sq).is_some_and(|cp| cp.piece() == piece))
            .collect();
        if !rivals.is_empty() {
            let same_file = rivals.iter().any(|sq| sq.file() == from.file());
            let same_rank = rivals.iter().any(|sq| sq.rank() == from.rank());
            if !same_file || same_rank {
                san.push((b'a' + from.file()) as char);
            }
            if same_file {
                san.push((b'1' + from.rank()) as char);
            }
        }
    }

    if mov.is_capture() {
        san.push('x');
    }
    san.push_str(&mov.to().to_string());

    if let Some(promo) = mov.promotion_piece() {
        san.push('=');
        san.push(piece_letter(promo));
    }
    san
}

fn piece_letter(piece: Piece) -> char {
    match piece {
        Piece::Pawn => 'P',
        Piece::Knight => 'N',
        Piece::Bishop => 'B',
        Piece::Rook => 'R',
        Piece::Queen => 'Q',
        Piece::King => 'K',
    }
}

pub fn find_move(board: &Board, uci_move: &str) -> Option<Move> {
    let uci_move = uci_move.to_ascii_lowercase();
    legal_moves(board)
        .into_iter()
        .find(|&mov| format_move(mov) == uci_move)
}

pub fn format_save(start: &Board, moves: impl IntoIterator<Item = Move>) -> String {
    let moves: Vec<String> = moves.into_iter().map(format_move).collect();
    format!("{}\n{}", start.to_fen(), moves.join(" "))
}

pub fn parse_save(save: &str) -> (&str, Vec<&str>) {
    let (fen, moves) = save.split_once('\n').unwrap_or((save, ""));
    (fen.trim(), moves.split_whitespace().collect())
}

fn result_of(game_state: GameState) -> &'static str {
    match game_state {
        GameState::WinnerWhite => "1-0",
        GameState::WinnerBlack => "0-1",
        GameState::Draw => "1/2-1/2",
        _ => "*",
    }
}

pub fn pgn(
    start: &Board,
    sans: &[&str],
    game_state: GameState,
    date: &str,
    player_is_white: bool,
) -> String {
    let result = result_of(game_state);
    let (white, black) = if player_is_white {
        (PLAYER, ENGINE)
    } else {
        (ENGINE, PLAYER)
    };

    let mut pgn = String::new();
    let tags = [
        ("Event", "Casual Game".to_string()),
        ("Site", SITE.to_string()),
        ("Date", date.to_string()),
        ("Round", "-".to_string()),
        ("White", white.to_string()),
        ("Black", black.to_string()),
        ("Result", result.to_string()),
        ("SetUp", "1".to_string()),
        ("FEN", start.to_fen()),
        ("PlyCount", sans.len().to_string()),
    ];
    for (name, value) in tags {
        pgn.push_str(&format!("[{name} \"{value}\"]\n"));
    }
    pgn.push('\n');

    let mut tokens = Vec::with_capacity(sans.len() * 3 / 2 + 1);
    for (i, san) in sans.iter().enumerate() {
        let ply = start.ply as usize + i;
        let move_number = ply / 2 + 1;
        if ply.is_multiple_of(2) {
            tokens.push(format!("{move_number}. {san}"));
        } else if i == 0 {
            tokens.push(format!("{move_number}... {san}"));
        } else {
            tokens.push(san.to_string());
        }
    }
    tokens.push(result.to_string());

    let mut line_len = 0;
    for token in tokens {
        if line_len > 0 && line_len + 1 + token.len() > MAX_LINE_LEN {
            pgn.push('\n');
            line_len = 0;
        } else if line_len > 0 {
            pgn.push(' ');
            line_len += 1;
        }
        line_len += token.len();
        pgn.push_str(&token);
    }
    pgn.push('\n');
    pgn
}
