use chess_engine::{board::Board, nnue::evaluate};
use nnue_trainer::{SCALE, build_trainer};

const CHECKPOINT: &str = "checkpoints/lucky-v5-480";
const TOLERANCE: f32 = 40.0;

const FENS: &[&str] = &[
    "rnbqkbnr/pppppppp/8/8/8/8/PPPPPPPP/RNBQKBNR w KQkq - 0 1",
    "rnbqkbnr/pp1ppppp/8/2p5/4P3/8/PPPP1PPP/RNBQKBNR w KQkq c6 0 2",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R w KQkq - 0 1",
    "r3k2r/p1ppqpb1/bn2pnp1/3PN3/1p2P3/2N2Q1p/PPPBBPPP/R3K2R b KQkq - 0 1",
    "rnbqkbnr/p1p2ppp/8/8/8/8/P1P2PPP/RNBQKBNR w KQkq - 0 1",
    "rnbqkbnr/pp2pp2/8/8/8/8/PP2PP2/RNBQKBNR b KQkq - 0 1",
    "r1bq1rk1/pp2bppp/2n2n2/2pp4/3P4/2N1PN2/PP2BPPP/R1BQ1RK1 w - - 0 1",
    "2r2rk1/1bqnbppp/p2ppn2/1p6/3NPP2/1BN1B3/PPPQ2PP/2KR3R b - - 0 1",
    "r4rk1/1pp1qppp/p1np1n2/2b1p1B1/2B1P3/2NP1N2/PPP2PPP/R2Q1RK1 w - - 0 1",
    "3r2k1/p4ppp/1p2p3/2b5/2P5/1P3N2/P4PPP/3R2K1 b - - 0 1",
    "8/2p2pkp/2p3p1/8/2P5/1P3NP1/P4P1P/6K1 w - - 0 1",
    "6k1/5ppp/8/8/8/8/1R3PPP/6K1 b - - 0 1",
    "r5k1/5ppp/8/8/8/8/5PPP/R5K1 w - - 0 1",
    "8/8/8/4k3/8/8/4Q3/4K3 w - - 0 1",
    "8/8/8/4k3/8/8/8/R3K3 w - - 0 1",
    "8/8/8/8/4k3/8/4P3/4K3 w - - 0 1",
    "8/8/8/4k3/8/8/8/1NB1K3 w - - 0 1",
    "8/8/4k3/8/8/4K3/8/R6r w - - 0 1",
    "8/8/4k3/8/8/4K3/8/R6r b - - 0 1",
    "4k3/8/4K3/4P3/8/8/8/8 b - - 0 1",
    "8/6k1/8/8/3B4/8/6K1/8 w - - 0 1",
    "8/5k2/8/8/8/8/3q4/4K3 b - - 0 1",
];

fn main() {
    let mut trainer = build_trainer();
    trainer.load_from_checkpoint(CHECKPOINT);
    println!("[parity] Loaded checkpoint '{CHECKPOINT}'");

    println!("\n{:>9} {:>9} {:>7}  fen", "engine_cp", "bullet_cp", "diff");
    println!("{}", "-".repeat(90));

    let mut fails = 0;
    let mut worst: f32 = 0.0;

    for &fen in FENS {
        let board = Board::from_fen(fen).expect("Invalid FEN");
        let engine_cp = evaluate(&board) as f32;

        let bullet_out = trainer.eval(fen);
        let bullet_cp = bullet_out * SCALE as f32;

        let diff = engine_cp - bullet_cp;
        worst = worst.max(diff.abs());

        let mut flag = "";
        if diff.abs() > TOLERANCE {
            flag = "  <== MISMATCH";
            fails += 1;
        } else if engine_cp != 0.0
            && engine_cp.signum() != bullet_cp.signum()
            && bullet_cp.abs() > 5.0
        {
            flag = "  <== SIGN";
            fails += 1;
        }

        println!(
            "{:>9.0} {:>9.1} {:>7.1}  {}{}",
            engine_cp, bullet_cp, diff, fen, flag
        );
    }

    println!("{}", "-".repeat(90));
    println!(
        "max |diff| = {:.1} cp, tolerance = {:.0} cp, failures = {}/{}",
        worst,
        TOLERANCE,
        fails,
        FENS.len()
    );

    if fails > 0 {
        eprintln!("PARITY FAILED — layout / quantisation / bucket bug likely.");
        std::process::exit(1);
    }

    println!("PARITY OK — engine eval matches the bullet checkpoint.");
}
