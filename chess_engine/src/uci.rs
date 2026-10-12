use chess_core::prelude::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use crate::nnue::evaluate;
use crate::search::{INFINITY, MATE_THRESHOLD, Searcher, piece_value};
use crate::time::{SearchOptions, TimeManager};
use crate::transposition::TranspositionTable;
use crate::{board::Board, move_gen::gen_all_moves};

pub type OutputCallback = Arc<dyn Fn(String) + Send + Sync>;

pub struct UciState {
    board: Board,
    game_history: Vec<u64>,
    searcher: Option<Box<Searcher>>,
    #[cfg(not(target_family = "wasm"))]
    search_thread: Option<std::thread::JoinHandle<Box<Searcher>>>,
    stop_requested: Arc<AtomicBool>,
    tt: Arc<TranspositionTable>,
    output_cb: OutputCallback,
    move_overhead: u64,
}

impl Default for UciState {
    fn default() -> Self {
        Self::new(|line| println!("{line}"))
    }
}

impl UciState {
    pub fn new(output_cb: impl Fn(String) + Send + Sync + 'static) -> Self {
        let board = Board::start_pos();
        let game_history = vec![board.hash()];
        let stop_requested = Arc::<AtomicBool>::default();
        let tt = Arc::new(TranspositionTable::new(64));
        Self {
            board,
            game_history,
            searcher: Some(Searcher::new(tt.clone(), stop_requested.clone())),
            #[cfg(not(target_family = "wasm"))]
            search_thread: None,
            stop_requested,
            tt,
            output_cb: Arc::new(output_cb),
            move_overhead: crate::time::DEFAULT_MOVE_OVERHEAD_MS,
        }
    }

    #[inline(always)]
    pub fn output_line(&self, line: impl Into<String>) {
        (self.output_cb)(line.into());
    }

    pub fn move_overhead(&self) -> u64 {
        self.move_overhead
    }

    pub fn set_move_overhead(&mut self, ms: u64) {
        self.move_overhead = ms;
    }

    pub fn stop(&mut self) {
        self.stop_requested.store(true, Ordering::Relaxed);
    }

    #[cfg(not(target_family = "wasm"))]
    fn join_search(&mut self, stop: bool) {
        if let Some(thread) = self.search_thread.take() {
            if stop {
                self.stop_requested.store(true, Ordering::Relaxed);
            }
            self.searcher = thread.join().ok();
        }
    }

    fn take_searcher(&mut self) -> Box<Searcher> {
        self.searcher
            .take()
            .unwrap_or_else(|| Searcher::new(self.tt.clone(), self.stop_requested.clone()))
    }

    #[cfg(target_family = "wasm")]
    fn join_search(&mut self, _stop: bool) {}

    pub fn process_command(&mut self, input: &str) -> bool {
        let (command, args) = split_first_word(input.trim());

        match command.to_ascii_lowercase().as_str() {
            "" => {}

            // Custom developer commands
            "d" | "display" => self.display_board(),
            "eval" => {
                let eval = evaluate(&self.board);
                self.output_line(format!("score: {}", format_score(eval)));
            }
            "perft" => self.run_perft(parse_perft_depth(args)),
            "wait" => self.join_search(false),

            // Standard UCI commands
            "uci" => {
                self.output_line(
                    r#"id name Lucky Chess 2.0
id author Răzvan Filea
option name Hash type spin default 64 min 1 max 1024
option name ClearHash type button
option name Move Overhead type spin default 10 min 0 max 5000
option name Threads type spin default 1 min 1 max 1
uciok"#,
                );
            }
            "debug" | "ponderhit" => {}
            "isready" => {
                self.join_search(false);
                self.output_line("readyok");
            }
            "setoption" => self.set_option(args),
            "register" => self.output_line("registration ok"),
            "ucinewgame" => {
                self.join_search(true);
                self.stop_requested.store(false, Ordering::Relaxed);
                self.board = Board::start_pos();
                self.game_history = vec![self.board.hash()];
                self.tt.clear();
                if let Some(searcher) = &mut self.searcher {
                    searcher.clear_histories();
                }
            }
            "position" => self.set_position(args),
            "go" => match split_first_word(args) {
                (word, depth) if word.eq_ignore_ascii_case("perft") => {
                    self.run_perft(parse_perft_depth(depth))
                }
                _ => {
                    let opts = parse_go(args, &self.board);
                    let time_manager =
                        TimeManager::from_options(&opts, self.board.to_play, self.move_overhead);
                    self.start_search(time_manager);
                }
            },
            "stop" => self.join_search(true),
            "quit" => {
                self.join_search(true);
                return false;
            }
            _ => eprintln!("Unknown command: {command}"),
        }

        true
    }

    fn set_option(&mut self, args: &str) {
        let mut tokens = args.split_whitespace();
        if !tokens
            .next()
            .is_some_and(|t| t.eq_ignore_ascii_case("name"))
        {
            return;
        }

        // Names are matched without spaces or case, so "Move Overhead" is "moveoverhead"
        let name = tokens
            .by_ref()
            .take_while(|t| !t.eq_ignore_ascii_case("value"))
            .collect::<String>()
            .to_ascii_lowercase();
        let value = tokens.next();

        match name.as_str() {
            "hash" => {
                if let Some(mb) = value.and_then(|v| v.parse::<usize>().ok()) {
                    self.join_search(true);
                    self.tt = Arc::new(TranspositionTable::new(mb.clamp(1, 1024)));
                    if let Some(searcher) = &mut self.searcher {
                        searcher.set_tt(self.tt.clone());
                    }
                }
            }
            "clearhash" => self.tt.clear(),
            "moveoverhead" => {
                if let Some(ms) = value.and_then(|v| v.parse::<u64>().ok()) {
                    self.move_overhead = ms.min(5000);
                }
            }
            // Lucky Chess is currently single-threaded
            "threads" => {}
            _ => eprintln!("Unknown option: {name}"),
        }
    }

    fn set_position(&mut self, args: &str) {
        let mut tokens = args.split_whitespace();
        let kind = tokens.next();
        let fen = tokens
            .by_ref()
            .take_while(|t| !t.eq_ignore_ascii_case("moves"))
            .collect::<Vec<_>>()
            .join(" ");

        let board = match kind.map(str::to_ascii_lowercase).as_deref() {
            Some("startpos") => Some(Board::start_pos()),
            Some("fen") => Board::from_fen(&fen).filter(has_one_king_each),
            _ => None,
        };
        let Some(board) = board else {
            eprintln!("Invalid position: {args}");
            return;
        };

        self.board = board;
        self.game_history = vec![self.board.hash()];

        for uci_move in tokens {
            let Some(mov) = find_move(&self.board, uci_move) else {
                eprintln!("Illegal or unrecognized move in position command: {uci_move}");
                break;
            };
            self.board.play_move(mov);
            self.game_history.push(self.board.hash());
        }
    }

    fn display_board(&self) {
        let mut out = format!("{:?}\n", self.board);
        out.push_str(&format!("FEN: {}\n", self.board.to_fen()));
        out.push_str(&format!("Key: 0x{:016X}", self.board.hash()));
        self.output_line(out);
    }

    fn run_perft(&mut self, depth: u8) {
        self.join_search(true);

        let mut board = self.board.clone();
        let nodes = crate::perft::perft(&mut board, depth);
        self.output_line(format!("Nodes searched: {nodes}"));
    }

    fn start_search(&mut self, time_manager: TimeManager) {
        self.join_search(true);

        self.stop_requested.store(false, Ordering::Relaxed);

        self.tt.new_search();

        let board = self.board.clone();
        let tt = self.tt.clone();
        let output_cb = self.output_cb.clone();

        let mut searcher = self.take_searcher();
        searcher.prepare_for_search(board.clone(), &self.game_history, time_manager);

        let run_search = move || {
            let on_info = |line: String| {
                output_cb(line);
            };

            let best = searcher.search(on_info);
            let mut ponder = None;
            if best != Move::NONE {
                let mut next_board = board;
                next_board.make_move(best);
                if let Some(entry) = tt.probe(next_board.hash(), 1)
                    && entry.mov != Move::NONE
                    && next_board.legal(entry.mov)
                {
                    ponder = Some(entry.mov);
                }
            }

            let best_line = match ponder {
                Some(p) => format!("bestmove {} ponder {}", format_move(best), format_move(p)),
                None => format!("bestmove {}", format_move(best)),
            };
            output_cb(best_line);
            searcher
        };

        #[cfg(not(target_family = "wasm"))]
        {
            self.search_thread = Some(std::thread::spawn(run_search));
        }

        #[cfg(target_family = "wasm")]
        {
            self.searcher = Some(run_search());
        }
    }
}

fn find_move(board: &Board, uci_move: &str) -> Option<Move> {
    let uci_move = uci_move.to_ascii_lowercase();
    gen_all_moves(board)
        .as_slice()
        .iter()
        .map(|scored_move| scored_move.mov)
        .find(|&mov| format_move(mov) == uci_move && board.legal(mov))
}

fn split_first_word(s: &str) -> (&str, &str) {
    match s.split_once(char::is_whitespace) {
        Some((word, rest)) => (word, rest.trim_start()),
        None => (s, ""),
    }
}

fn has_one_king_each(board: &Board) -> bool {
    [Color::White, Color::Black]
        .into_iter()
        .all(|color| board.color_piece(Piece::King, color).count_ones() == 1)
}

fn parse_perft_depth(s: &str) -> u8 {
    s.trim().parse::<u8>().unwrap_or(1).max(1)
}

// GUIs can send negative clock times
fn parse_clamped(s: &str) -> Option<u64> {
    s.parse::<i128>()
        .ok()
        .map(|n| n.clamp(0, u64::MAX.into()) as u64)
}

pub fn parse_go(args: &str, board: &Board) -> SearchOptions {
    let mut opts = SearchOptions::default();

    let mut tokens = args.split_whitespace().peekable();
    while let Some(key) = tokens.next() {
        let mut value = || tokens.next().and_then(parse_clamped);
        match key.to_ascii_lowercase().as_str() {
            "wtime" => opts.wtime = value().map(Duration::from_millis),
            "btime" => opts.btime = value().map(Duration::from_millis),
            "winc" => opts.winc = value().map(Duration::from_millis),
            "binc" => opts.binc = value().map(Duration::from_millis),
            "movetime" => opts.movetime = value().map(Duration::from_millis),
            "movestogo" => opts.movestogo = value(),
            "depth" => opts.depth = value(),
            "nodes" => opts.nodes = value(),
            "infinite" => opts.infinite = true,
            // The list ends at the first token that is not a legal move, e.g. the next keyword
            "searchmoves" => {
                while let Some(mov) = tokens.peek().and_then(|t| find_move(board, t)) {
                    opts.searchmoves.push(mov);
                    tokens.next();
                }
            }
            _ => {}
        }
    }

    // Without any limit, search until `stop`
    opts.infinite |= opts.depth.is_none()
        && opts.nodes.is_none()
        && opts.movetime.is_none()
        && opts.wtime.is_none()
        && opts.btime.is_none();

    opts
}

pub fn format_move(mov: Move) -> String {
    let promo = match mov.promotion_piece() {
        Some(Piece::Queen) => "q",
        Some(Piece::Rook) => "r",
        Some(Piece::Bishop) => "b",
        Some(Piece::Knight) => "n",
        _ => "",
    };
    format!("{}{}{promo}", mov.from(), mov.to())
}

pub fn format_score(score: i16) -> String {
    if score > MATE_THRESHOLD {
        // We are mating the opponent
        let plies_to_mate = INFINITY - score;
        let moves_to_mate = (plies_to_mate + 1) / 2;
        format!("mate {moves_to_mate}")
    } else if score < -MATE_THRESHOLD {
        // Opponent is mating us
        let plies_to_mate = INFINITY + score;
        let moves_to_mate = (plies_to_mate + 1) / 2;
        format!("mate -{moves_to_mate}")
    } else {
        format!(
            "cp {}",
            100 * score as i32 / (piece_value(Piece::Pawn) as i32)
        )
    }
}
