use deep_pawn::*;
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};

struct Game {
    board: Board,
    turn: Color,
    halfmove_clock: u32,
    history: Vec<String>,
    san_moves: Vec<String>,
}

impl Game {
    fn new() -> Self {
        let board = Board::default();
        Self {
            history: vec![board.position_key(Color::White)],
            board,
            turn: Color::White,
            halfmove_clock: 0,
            san_moves: Vec::new(),
        }
    }
    fn result(&self) -> &'static str {
        if is_checkmate(&self.board, self.turn) {
            if self.turn == Color::White {
                "0-1"
            } else {
                "1-0"
            }
        } else if is_stalemate(&self.board, self.turn)
            || is_insufficient_material(&self.board)
            || is_threefold_repetition(&self.history)
            || is_fifty_move_draw(self.halfmove_clock)
        {
            "1/2-1/2"
        } else {
            "*"
        }
    }
    fn apply(&mut self, move_: Move, legal: &[Move]) {
        let piece = self
            .board
            .get(move_.from)
            .expect("legal move source exists");
        let capture = self.board.get(move_.to).is_some();
        let notation = move_to_san(&self.board, move_, self.turn, legal);
        self.board.apply(move_).expect("legal move applies");
        self.halfmove_clock = if piece.kind == PieceKind::Pawn || capture {
            0
        } else {
            self.halfmove_clock + 1
        };
        self.turn = self.turn.other();
        self.history.push(self.board.position_key(self.turn));
        self.san_moves.push(format!(
            "{notation}{}",
            check_suffix(&self.board, self.turn)
        ));
    }
}

fn print_board(board: &Board) {
    println!("\n    a b c d e\n   -----------");
    let symbols: HashMap<String, char> = [
        ("WK", '♔'),
        ("WQ", '♕'),
        ("WR", '♖'),
        ("WB", '♗'),
        ("WN", '♘'),
        ("WP", '♙'),
        ("BK", '♚'),
        ("BQ", '♛'),
        ("BR", '♜'),
        ("BB", '♝'),
        ("BN", '♞'),
        ("BP", '♟'),
    ]
    .into_iter()
    .map(|(code, symbol)| (code.to_owned(), symbol))
    .collect();
    for rank in (1..=BOARD_SIZE).rev() {
        let row = (0..BOARD_SIZE)
            .map(|file| {
                symbols
                    .get(&board.piece_code(Square { file, rank }))
                    .copied()
                    .unwrap_or(' ')
            })
            .map(|symbol| symbol.to_string())
            .collect::<Vec<_>>()
            .join(" ");
        println!("{rank}  {row}  {rank}");
    }
    println!("   -----------\n    a b c d e\n");
}

fn status(game: &Game) {
    println!(
        "Turn: {}{}",
        game.turn.name(),
        if is_in_check(&game.board, game.turn) {
            " — CHECK"
        } else {
            ""
        }
    );
    if is_checkmate(&game.board, game.turn) {
        println!("CHECKMATE — {} wins.", game.turn.other().name());
    }
    if is_stalemate(&game.board, game.turn) {
        println!("STALEMATE — draw.");
    }
    if is_insufficient_material(&game.board) {
        println!("DRAW — insufficient material.");
    }
    if is_threefold_repetition(&game.history) {
        println!("DRAW CLAIM — threefold repetition.");
    }
    if is_fifty_move_draw(game.halfmove_clock) {
        println!("DRAW CLAIM — fifty-move rule.");
    }
}

fn help() {
    println!(
        "Commands:\n  show                         Display the board and game status\n  moves                        List legal moves for the current player\n  move e2 e3 [Q|R|B|N]         Play a move; promotion defaults to queen\n  fen                          Display the current board as 5x5 FEN\n  loadfen <fen> [white|black] Replace the game with a 5x5 FEN position\n  solve [fen] [white|black] [threads]  Exhaustively solve with CPU workers\n  solve5 <fen> [white|black] [threads] Solve legal K+K+three-piece FEN\n  tablebase kqk [file]         Build an exact KQvK retrograde WDL table\n  pgn                          Print the current game as PGN\n  savepgn [file]               Save the game as a PGN file\n  text                         Print a text-only game state\n  savetext [file]              Save the text state to a file\n  debug <function> [square]    Call an engine function directly\n  reset                        Reset to the starting position\n  help                         Show this help\n  quit                         Exit"
    );
}

fn promotion(value: Option<&str>) -> Option<PieceKind> {
    match value?.to_ascii_uppercase().as_str() {
        "Q" => Some(PieceKind::Queen),
        "R" => Some(PieceKind::Rook),
        "B" => Some(PieceKind::Bishop),
        "N" => Some(PieceKind::Knight),
        _ => None,
    }
}

fn debug(game: &Game, args: &[&str]) {
    let player = match args.get(2).copied() {
        Some("black") => Color::Black,
        _ => game.turn,
    };
    let square = args.get(1).and_then(|value| Square::parse(value));
    let moves = |squares: Vec<Square>| {
        squares
            .into_iter()
            .map(|square| square.to_string())
            .collect::<Vec<_>>()
            .join(",")
    };
    match args.first().copied() {
        Some("allLegalMoves") => println!(
            "{}",
            legal_moves(&game.board, player)
                .into_iter()
                .map(|move_| move_.label())
                .collect::<Vec<_>>()
                .join(",")
        ),
        Some("isInCheck") => println!("{}", is_in_check(&game.board, player)),
        Some("isCheckmate") => println!("{}", is_checkmate(&game.board, player)),
        Some("isStalemate") => println!("{}", is_stalemate(&game.board, player)),
        Some("isInsufficientMaterial") => println!("{}", is_insufficient_material(&game.board)),
        Some("isFiftyMoveDraw") => println!("{}", is_fifty_move_draw(game.halfmove_clock)),
        Some("isThreefoldRepetition") => println!("{}", is_threefold_repetition(&game.history)),
        Some("validSquare") => println!("{}", square.is_some()),
        Some("hasPiece") => println!(
            "{}",
            square.is_some_and(|square| game.board.get(square).is_some())
        ),
        Some("selfPiece") => println!(
            "{}",
            square.is_some_and(|square| game
                .board
                .get(square)
                .is_some_and(|piece| piece.color == player))
        ),
        Some("isAttacked") => println!(
            "{}",
            square.is_some_and(|square| is_attacked(&game.board, square, player))
        ),
        Some("kingMoves") | Some("queenMoves") | Some("rookMoves") | Some("bishopMoves")
        | Some("knightMoves") | Some("pawnMoves") => println!(
            "{}",
            square
                .map(|square| moves(pseudo_moves(&game.board, square)))
                .unwrap_or_default()
        ),
        _ => println!(
            "Unknown function. Use allLegalMoves, isInCheck, isCheckmate, isStalemate, isAttacked, hasPiece, selfPiece, validSquare, or <piece>Moves."
        ),
    }
}

fn solve(board: &Board, turn: Color, halfmove_clock: u32, workers: usize) {
    println!(
        "Solving {} to move from FEN: {}",
        turn.name(),
        board.to_fen()
    );
    report_solve_result(solve_position_parallel(
        board,
        turn,
        halfmove_clock,
        workers,
    ));
}

fn solve5(board: &Board, turn: Color, workers: usize) {
    println!(
        "Solving legal five-piece position (K+K+three pieces), {} to move from FEN: {}",
        turn.name(),
        board.to_fen()
    );
    match solve_five_piece_position(board, turn, 0, workers) {
        Ok(result) => report_solve_result(result),
        Err(error) => println!("Cannot solve five-piece position: {error}"),
    }
}

fn report_solve_result(result: SolveResult) {
    match result.outcome {
        deep_pawn::Outcome::Draw => {
            println!("Result: draw (WDL; draw distance is not assigned by search)")
        }
        _ => println!(
            "Result: {} in {} plies",
            result.outcome, result.plies_to_result
        ),
    }
    println!(
        "Best move(s): {}",
        result
            .best_moves
            .iter()
            .map(|move_| move_.label())
            .collect::<Vec<_>>()
            .join(", ")
    );
    println!("Positions analyzed: {}", result.positions_analyzed);
    println!(
        "Search time: {:.3} seconds ({:.0} positions/second)",
        result.elapsed.as_secs_f64(),
        result.positions_per_second
    );
    println!("Alpha-beta cutoffs: {}", result.alpha_beta_cutoffs);
    println!(
        "Cache: {} hits, {} misses; {} legal moves; max depth {}",
        result.cache_hits, result.cache_misses, result.legal_moves_generated, result.maximum_depth
    );
    if !result.moves.is_empty() {
        println!("All legal root moves:");
        for evaluation in result.moves {
            if evaluation.outcome == deep_pawn::Outcome::Draw {
                println!("  {} -> draw", evaluation.move_.label());
            } else {
                println!(
                    "  {} -> {} in {} plies",
                    evaluation.move_.label(),
                    evaluation.outcome,
                    evaluation.plies_to_result
                );
            }
        }
    }
}

fn main() {
    let mut game = Game::new();
    println!("5x5 mini chess terminal debugger");
    help();
    print_board(&game.board);
    status(&game);
    loop {
        print!("chess> ");
        io::stdout().flush().expect("prompt flushes");
        let mut line = String::new();
        if io::stdin().read_line(&mut line).is_err() {
            break;
        }
        let args: Vec<_> = line.split_whitespace().collect();
        let Some(command) = args.first().copied() else {
            continue;
        };
        match command {
            "show" => {
                print_board(&game.board);
                status(&game);
            }
            "moves" => {
                let moves = legal_moves(&game.board, game.turn);
                if moves.is_empty() {
                    println!("No legal moves.");
                } else {
                    println!(
                        "\n{} legal moves ({}):\n{}",
                        game.turn.name(),
                        moves.len(),
                        moves
                            .iter()
                            .map(|move_| move_.label())
                            .collect::<Vec<_>>()
                            .join("  ")
                    );
                }
            }
            "move" => {
                if args.len() < 3 {
                    println!("Usage: move e2 e3 [Q|R|B|N]");
                    continue;
                }
                let (Some(from), Some(to)) = (Square::parse(args[1]), Square::parse(args[2]))
                else {
                    println!("Invalid square. Use files a-e and ranks 1-5.");
                    continue;
                };
                let legal = legal_moves(&game.board, game.turn);
                let promotion_move = legal
                    .iter()
                    .any(|move_| move_.from == from && move_.to == to && move_.promotion.is_some());
                let chosen_promotion = if promotion_move {
                    promotion(args.get(3).copied()).or(Some(PieceKind::Queen))
                } else {
                    None
                };
                let Some(move_) = legal.iter().copied().find(|move_| {
                    move_.from == from && move_.to == to && move_.promotion == chosen_promotion
                }) else {
                    println!("Illegal move. Use `moves` to list legal moves.");
                    continue;
                };
                game.apply(move_, &legal);
                print_board(&game.board);
                status(&game);
            }
            "fen" => println!("{} {}", game.board.to_fen(), game.turn.name()),
            "loadfen" => {
                let Some(fen) = args.get(1) else {
                    println!("Usage: loadfen <fen> [white|black]");
                    continue;
                };
                let turn = match args.get(2).copied() {
                    Some("black") => Color::Black,
                    Some("white") | None => Color::White,
                    Some(_) => {
                        println!("Colour must be `white` or `black`.");
                        continue;
                    }
                };
                match Board::from_fen(fen) {
                    Ok(board) => {
                        game = Game {
                            history: vec![board.position_key(turn)],
                            board,
                            turn,
                            halfmove_clock: 0,
                            san_moves: Vec::new(),
                        };
                        print_board(&game.board);
                        status(&game);
                    }
                    Err(error) => println!("Invalid FEN: {error}"),
                }
            }
            "solve" => {
                let turn = match args.get(2).copied() {
                    Some("white") => Color::White,
                    Some("black") => Color::Black,
                    Some(_) => {
                        println!("Colour must be `white` or `black`.");
                        continue;
                    }
                    None => game.turn,
                };
                let workers = args
                    .get(3)
                    .and_then(|value| value.parse().ok())
                    .unwrap_or(1);
                if let Some(fen) = args.get(1) {
                    match Board::from_fen(fen) {
                        Ok(board) => solve(&board, turn, 0, workers),
                        Err(error) => println!("Invalid FEN: {error}"),
                    }
                } else {
                    solve(&game.board, turn, game.halfmove_clock, workers);
                }
            }
            "solve5" => {
                let Some(fen) = args.get(1) else {
                    println!("Usage: solve5 <fen> [white|black] [threads]");
                    continue;
                };
                let turn = match args.get(2).copied() {
                    Some("black") => Color::Black,
                    Some("white") | None => Color::White,
                    Some(_) => {
                        println!("Colour must be `white` or `black`.");
                        continue;
                    }
                };
                let workers = args
                    .get(3)
                    .and_then(|value| value.parse().ok())
                    .filter(|&count| count > 0)
                    .unwrap_or(1);
                match Board::from_fen(fen) {
                    Ok(board) => solve5(&board, turn, workers),
                    Err(error) => println!("Invalid FEN: {error}"),
                }
            }
            "tablebase" => match args.get(1).copied() {
                Some("one-piece") => {
                    let path = args.get(2).copied().unwrap_or("one-piece.dptb");
                    println!("Building promotion-aware combined one-piece WDL tablebase: {path}");
                    match build_one_piece_wdl(path) {
                        Ok(stats) => println!(
                            "Done: {} positions; {} wins, {} losses, {} draws.",
                            stats.positions, stats.wins, stats.losses, stats.draws
                        ),
                        Err(error) => println!("Could not build tablebase: {error}"),
                    }
                }
                Some(kind @ ("kqk" | "krk" | "kbk" | "knk")) => {
                    let default_path = format!("{kind}.dptb");
                    let path = args.get(2).copied().unwrap_or(&default_path);
                    println!("Building exact clock-independent {kind} WDL tablebase: {path}");
                    let result = match kind {
                        "kqk" => build_kqk_wdl(path),
                        "krk" => build_krk_wdl(path),
                        "kbk" => build_kbk_wdl(path),
                        "knk" => build_knk_wdl(path),
                        _ => unreachable!(),
                    };
                    match result {
                        Ok(stats) => println!(
                            "Done: {} positions; {} wins, {} losses, {} draws.",
                            stats.positions, stats.wins, stats.losses, stats.draws
                        ),
                        Err(error) => println!("Could not build tablebase: {error}"),
                    }
                }
                _ => println!("Usage: tablebase <kqk|krk|kbk|knk> [file]"),
            },
            "probe" => {
                let Some(fen) = args.get(1) else {
                    println!("Usage: probe <fen> [white|black]");
                    continue;
                };
                let turn = if args.get(2).copied() == Some("black") {
                    Color::Black
                } else {
                    Color::White
                };
                match Board::from_fen(fen) {
                    Ok(board) => match probe_tablebase(&board, turn) {
                        TablebaseProbe::Win { best_move } => println!(
                            "WDL: win; move: {}",
                            best_move
                                .map(|move_| move_.label())
                                .unwrap_or_else(|| "none".to_owned())
                        ),
                        TablebaseProbe::Draw { best_move } => println!(
                            "WDL: draw; move: {}",
                            best_move
                                .map(|move_| move_.label())
                                .unwrap_or_else(|| "none".to_owned())
                        ),
                        TablebaseProbe::Loss { best_move } => println!(
                            "WDL: loss; move: {}",
                            best_move
                                .map(|move_| move_.label())
                                .unwrap_or_else(|| "none".to_owned())
                        ),
                        TablebaseProbe::Unknown => {
                            println!("WDL: unknown (no tablebase for this material signature).")
                        }
                    },
                    Err(error) => println!("Invalid FEN: {error}"),
                }
            }
            "pgn" => println!("{}", export_pgn(&game.san_moves, game.result())),
            "savepgn" => match fs::write(
                args.get(1).copied().unwrap_or("game.pgn"),
                format!("{}\n", export_pgn(&game.san_moves, game.result())),
            ) {
                Ok(()) => println!("PGN saved."),
                Err(error) => println!("Could not save PGN: {error}"),
            },
            "text" => println!(
                "{}",
                state_text(
                    &game.board,
                    game.turn,
                    game.halfmove_clock,
                    &game.history,
                    &game.san_moves
                )
            ),
            "savetext" => match fs::write(
                args.get(1).copied().unwrap_or("game.txt"),
                format!(
                    "{}\n",
                    state_text(
                        &game.board,
                        game.turn,
                        game.halfmove_clock,
                        &game.history,
                        &game.san_moves
                    )
                ),
            ) {
                Ok(()) => println!("Text state saved."),
                Err(error) => println!("Could not save text state: {error}"),
            },
            "debug" => debug(&game, &args[1..]),
            "reset" => {
                game = Game::new();
                print_board(&game.board);
                status(&game);
            }
            "help" => help(),
            "quit" | "exit" => break,
            _ => println!("Unknown command. Type `help`."),
        }
    }
    println!("Goodbye.");
}
