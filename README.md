# DeepPawn — 5×5 Mini Chess in Rust

A terminal-first Rust implementation of 5×5 mini chess. The board uses files `a`–`e` and ranks `1`–`5`; pieces have standard chess movement except that pawn double-steps, en passant, and castling are not allowed.

The starting position gives each side a rook, knight, bishop, queen, king, and five pawns. Pawns promote on rank 5 for White and rank 1 for Black.

## Requirements

Rust 1.85 or newer (Rust 2024 edition).

## Run

```bash
cargo run
```

## Test

```bash
cargo test
```

## Terminal commands

```text
show                         Display the board and game status
moves                        List legal moves for the current player
move e2 e3 [Q|R|B|N]         Play a move; promotion defaults to queen
fen                          Display the current board as 5x5 FEN
loadfen <fen> [white|black] Replace the game with a 5x5 FEN position
solve [fen] [white|black] [threads]  Exhaustively solve with CPU workers
solve5 <fen> [white|black] [threads] Solve a legal K+K+three-piece position
pgn                          Print the current game as PGN
savepgn [file]               Save the game as a PGN file
text                         Print the text-only game state
savetext [file]              Save the text state to a file
debug <function> [square]    Call an engine function directly
reset                        Reset to the starting position
help                         Display command help
quit                         Exit
```

Examples:

```text
move e2 e3
debug pawnMoves e2 white
debug allLegalMoves - white
debug isAttacked e4 white
solve 4k/3Q1/2K2/5/5 black
loadfen 4k/5/2K2/3Q1/5 white
```

`solve` evaluates every legal continuation, caches transpositions, and prints
the forced outcome, optimal move(s), result for every legal root move, total
positions analyzed, elapsed time, and positions per second.
It applies alpha-beta pruning and reports the number of branches cut off by
that pruning, cache hits/misses, legal moves generated, and maximum depth.
Run the solver optimized with `cargo run --release`.
Pass a worker count to split legal root moves across CPU threads, for example:

```text
solve 4k/5/2K2/3Q1/5 white 16
```

## Five-piece cluster solving

`solve5` accepts exactly five pieces: one white king, one black king, and any
three queens, rooks, bishops, knights, or non-promoted pawns of either colour.
It rejects malformed positions, duplicate kings, pawns on a promotion rank,
and positions where the non-moving side is in check.

```text
solve5 4k/5/2K2/1QRN1/5 white 32
```

On Iridis, submit a position directly with the parameterized Slurm job:

```bash
sbatch scripts/iridis_five_piece_solve.sbatch '4k/5/2K2/1QRN1/5' white
```

The job uses all allocated CPUs by default; pass a third argument to set the
number of solver workers explicitly. It writes the full result to `logs/`.

For multi-worker solves, work is split beneath the root and completed entries
are shared through a striped transposition table. For multi-node HPC runs,
submit separate positions or root-move sets as scheduler jobs; ordinary Rust
threads cannot coordinate separate nodes. Without an explicit worker count,
the solver runs serially.
The optional FEN uses five ranks from Black's side down, with uppercase White
pieces and lowercase Black pieces. For example,
`4k/3Q1/2K2/5/5` is a checkmated Black king on `e5`.
Use `fen` to emit the live board and `loadfen` to set a new board. The optional
side-to-move argument defaults to `white` when loading a position.

An exhaustive search from material-rich positions can be very large. The
solver is intended primarily for solving endgames and other reduced positions.

## Library API

The public engine API is in [`src/lib.rs`](./src/lib.rs). Key functions include:

```rust
use deep_pawn::{Board, Color, legal_moves, solve_position, is_in_check, is_checkmate};

let board = Board::default();
let moves = legal_moves(&board, Color::White);
assert!(!is_in_check(&board, Color::White));
assert!(!is_checkmate(&board, Color::White));
let result = solve_position(&board, Color::White, 0);
```

## Project structure

```text
.
├── Cargo.toml
├── src/
│   ├── lib.rs       # Engine, move generation, state/PGN text exports, tests
│   └── main.rs      # Interactive terminal application
└── target/          # Cargo build output (generated)
```

This repository now uses Rust as its sole implementation and supported entry point.
