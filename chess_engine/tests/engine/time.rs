use chess_core::prelude::*;
use chess_engine::board::Board;
use chess_engine::search::search;
use chess_engine::time::{DEFAULT_MOVE_OVERHEAD_MS, Instant, SearchOptions, TimeManager};
use chess_engine::transposition::TranspositionTable;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Duration;

fn ms(millis: u64) -> Option<Duration> {
    Some(Duration::from_millis(millis))
}

fn white(opts: &SearchOptions) -> TimeManager {
    TimeManager::from_options(opts, Color::White, DEFAULT_MOVE_OVERHEAD_MS)
}

#[test]
fn test_time_manager_from_depth() {
    let tm = TimeManager::from_depth(5);
    assert_eq!(tm.limits.max_depth, 5);
    assert!(tm.limits.optimum_time.is_none());
    assert!(tm.limits.max_time.is_none());
    assert!(!tm.limits.infinite);
}

#[test]
fn test_time_manager_clock() {
    let opts = SearchOptions {
        wtime: ms(60_000),
        winc: ms(1_000),
        btime: ms(30_000),
        binc: ms(500),
        ..Default::default()
    };
    let w = white(&opts);
    let b = TimeManager::from_options(&opts, Color::Black, DEFAULT_MOVE_OVERHEAD_MS);
    assert!(w.limits.optimum_time > b.limits.optimum_time);
    assert!(w.limits.max_time > w.limits.optimum_time);

    // 60s over 10 moves is about 6s per move
    let opts = SearchOptions {
        wtime: ms(60_000),
        movestogo: Some(10),
        ..Default::default()
    };
    let optimum = white(&opts).limits.optimum_time.unwrap().as_millis();
    assert!((5_000..=6_500).contains(&optimum), "{optimum}");

    // Low on time: never plan beyond what is left
    let opts = SearchOptions {
        wtime: ms(50),
        winc: ms(0),
        ..Default::default()
    };
    let tm = white(&opts);
    assert!(tm.limits.optimum_time.unwrap() <= Duration::from_millis(50));
    assert!(tm.limits.max_time.unwrap() <= Duration::from_millis(50));
}

#[test]
fn test_time_manager_move_overhead() {
    let opts = SearchOptions {
        wtime: ms(10_000),
        movestogo: Some(20),
        ..Default::default()
    };
    let low = TimeManager::from_options(&opts, Color::White, 10);
    let high = TimeManager::from_options(&opts, Color::White, 500);
    assert!(low.limits.optimum_time > high.limits.optimum_time);
    assert!(low.limits.max_time > high.limits.max_time);

    let opts = SearchOptions {
        movetime: ms(100),
        ..Default::default()
    };
    let tm = TimeManager::from_options(&opts, Color::White, 30);
    assert_eq!(tm.limits.optimum_time, ms(70));
    assert_eq!(tm.limits.max_time, ms(70));
}

#[test]
#[cfg_attr(miri, ignore)]
fn test_search_limits() {
    let board = Board::start_pos();
    let tt = Arc::new(TranspositionTable::with_buckets(16));
    let run = |tm| {
        search(
            board.clone(),
            &[],
            tm,
            Arc::new(AtomicBool::new(false)),
            &tt,
            |_| {},
        )
    };

    let start = Instant::now();
    let best = run(TimeManager::from_movetime(Duration::from_millis(50)));
    assert!(start.elapsed() <= Duration::from_millis(250));
    assert!(board.legal(best));

    let best = run(TimeManager::from_nodes(5000));
    assert!(board.legal(best));
}
