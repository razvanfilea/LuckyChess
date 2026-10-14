// One test binary: each `tests/*.rs` file would otherwise link the engine separately

mod common;

mod board;
mod invariants;
mod movegen;
mod primitives;
mod search;
mod see;
mod time;
mod uci;
