# LuckyChess

[![Rust](https://img.shields.io/badge/Rust-2024_Edition-orange.svg)](https://www.rust-lang.org/)
[![License: GPL-3.0](https://img.shields.io/badge/License-GPLv3-blue.svg)](LICENSE.txt)
[![CI](https://github.com/razvanfilea/LuckyChess/actions/workflows/ci.yml/badge.svg)](https://github.com/razvanfilea/LuckyChess/actions/workflows/ci.yml)

A UCI chess engine written in Rust with an embedded NNUE network. Runs natively, in the browser (WASM) and on Android.

## Downloads

Prebuilt binaries are on the [releases page](https://github.com/razvanfilea/LuckyChess/releases):

- `avx512` / `avx2` — fastest on modern CPUs
- `sse42` — older CPUs, and AMD Zen 1/2 (the AVX2 builds use PEXT, which is slow on those)

## Building

Requires Rust 1.98+ and [Git LFS](https://git-lfs.com/) for the network weights (`git lfs pull` after cloning).

```bash
RUSTFLAGS="-C target-cpu=native" cargo build --release --bin lucky_chess
```

The binary is written to `target/release/lucky_chess`.

**Web:** `docker build -t lucky-chess . && docker run -p 8080:8080 lucky-chess`, then open `http://localhost:8080`.
To build only the WASM module: `cargo build-wasm`.

## UCI

| Option | Default | Range |
|---|---|---|
| `Hash` | 64 | 1 – 1024 MB |
| `Move Overhead` | 10 | 0 – 5000 ms |
| `Threads` | 1 | 1 (single-threaded) |
| `ClearHash` | button | |

Extra commands: `eval`, `d`, `perft <depth>`, and `bench [depth]` / `perft-suite [depth]` (also as CLI arguments,
e.g. `lucky_chess bench 12`).

## Features

- **Move generation:** bitboards with PEXT (BMI2) or magic slider attacks, staged move picker, lazy SEE
- **Search:** PVS with iterative deepening and aspiration windows, transposition table, IIR, singular extensions,
  quiescence search
- **Pruning & reductions:** LMR, LMP, NMP, RFP, futility, SEE and history pruning, delta pruning in qsearch
- **Move ordering:** MVV-LVA + SEE, killers, history and 1/2-ply continuation history
- **Evaluation:** NNUE `(768×8 → 1536)x2 → 1×8`, SCReLU, 8 mirrored king buckets, 8 output buckets,
  incremental accumulators with Finny tables, SIMD via `fearless_simd`. Trained with
  [`bullet`](https://github.com/jw1912/bullet) on 20B positions of
  [Leela-derived data](https://huggingface.co/datasets/linrock/bullet-training-data)

## Testing

- `cargo test --all` — perft, move generation, make/undo, FEN, SEE, Zobrist, draws, time management, NNUE parity
- `benchmarks/sprt_self_play.sh [elo0] [elo1] [tc]` — SPRT against a saved baseline (`benchmarks/save_baseline.sh`)
  with [`fastchess`](https://github.com/Disservin/fastchess)
- `benchmarks/gauntlet_ccrl_blitz.sh [tc] [rounds] [concurrency]` — CCRL Blitz-style gauntlet

Latest gauntlet (v5 net, 60+0.6, 128 MB hash, 200 games per opponent):

| Engine | Elo | Score |
|---|---|---|
| Prune 4.0.1 (3645) | +54.3 ± 22.5 | 57.8% |
| **LuckyChess** | **+7.0 ± 11.0** | **51.0%** |
| Akimbo 1.0.0 (3614) | −15.7 ± 25.2 | 47.8% |
| Oxide 3.0 (3530) | −31.4 ± 14.1 | 45.5% |
| Pawn 4.0 (3557) | −34.9 ± 20.7 | 45.0% |

That puts it around 3,590 on CCRL Blitz. This is an unofficial estimate, since 60+0.6 is not calibrated to CCRL hardware.

## Workspace

```text
chess_core/      core types: bitboards, squares, pieces, moves
chess_engine/    board, move generation, search, NNUE, UCI
chess_cli/       native binary (lucky_chess)
chess_web/       WASM build and browser frontend
chess_android/   JNI bindings and Android / Wear OS app
nnue_trainer/    NNUE training pipeline
benchmarks/      SPRT and gauntlet scripts, opening books
```

## Credits

The code is original. It draws on ideas from [Stockfish](https://github.com/official-stockfish/Stockfish),
[Alexandria](https://github.com/PGG106/Alexandria) and the [Chess Programming Wiki](https://www.chessprogramming.org/).
Thanks to [jw1912](https://github.com/jw1912) for `bullet` and [linrock](https://huggingface.co/linrock) for the training data.

**AI usage:** LLMs were used for tests and fixtures, profiling and benchmark analysis, and code review. The architecture,
design decisions and training runs are the author's.

