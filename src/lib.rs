use std::collections::{HashMap, VecDeque};
use std::fmt;
use std::fs::{self, File};
use std::hash::{BuildHasherDefault, Hasher};
use std::io::{BufWriter, Write};
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant};

pub const FILES: [char; 5] = ['a', 'b', 'c', 'd', 'e'];
pub const BOARD_SIZE: usize = 5;

/// A fast non-cryptographic hasher for the solver's already well-distributed
/// packed `u128` position keys. General-purpose SipHash is unnecessary here.
#[derive(Default)]
struct PositionHasher(u64);

impl Hasher for PositionHasher {
    fn finish(&self) -> u64 {
        self.0
    }

    fn write(&mut self, bytes: &[u8]) {
        let mut hash = 0_u64;
        for &byte in bytes {
            hash = hash.rotate_left(5) ^ u64::from(byte);
        }
        self.0 = hash;
    }

    fn write_u128(&mut self, value: u128) {
        let low = value as u64;
        let high = (value >> 64) as u64;
        self.0 = low ^ high.rotate_left(29);
    }
}

type PositionMap<T> = HashMap<u128, T, BuildHasherDefault<PositionHasher>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Color {
    White,
    Black,
}

impl Color {
    pub fn other(self) -> Self {
        match self {
            Self::White => Self::Black,
            Self::Black => Self::White,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::White => "white",
            Self::Black => "black",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceKind {
    King,
    Queen,
    Rook,
    Bishop,
    Knight,
    Pawn,
}

impl PieceKind {
    pub fn code(self) -> char {
        match self {
            Self::King => 'K',
            Self::Queen => 'Q',
            Self::Rook => 'R',
            Self::Bishop => 'B',
            Self::Knight => 'N',
            Self::Pawn => 'P',
        }
    }
    pub fn promotion_choices() -> [Self; 4] {
        [Self::Queen, Self::Rook, Self::Bishop, Self::Knight]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Piece {
    pub color: Color,
    pub kind: PieceKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Square {
    pub file: usize,
    pub rank: usize,
}

impl Square {
    pub fn new(file: usize, rank: usize) -> Option<Self> {
        (file < BOARD_SIZE && (1..=BOARD_SIZE).contains(&rank)).then_some(Self { file, rank })
    }
    pub fn parse(value: &str) -> Option<Self> {
        let mut chars = value.chars();
        let file = chars.next()?;
        let rank = chars.next()?.to_digit(10)? as usize;
        (chars.next().is_none()).then_some(())?;
        Self::new(FILES.iter().position(|&candidate| candidate == file)?, rank)
    }
}

impl fmt::Display for Square {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", FILES[self.file], self.rank)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Move {
    pub from: Square,
    pub to: Square,
    pub promotion: Option<PieceKind>,
}

impl Move {
    pub fn label(self) -> String {
        let suffix = self
            .promotion
            .map(|kind| format!("={}", kind.code()))
            .unwrap_or_default();
        format!("{}-{}{}", self.from, self.to, suffix)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Board {
    squares: [[Option<Piece>; BOARD_SIZE]; BOARD_SIZE],
}

impl Default for Board {
    fn default() -> Self {
        let mut board = Self::empty();
        let back_rank = [
            PieceKind::Rook,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Queen,
            PieceKind::King,
        ];
        for (file, kind) in back_rank.into_iter().enumerate() {
            board.set(
                Square { file, rank: 1 },
                Some(Piece {
                    color: Color::White,
                    kind,
                }),
            );
            board.set(
                Square { file, rank: 2 },
                Some(Piece {
                    color: Color::White,
                    kind: PieceKind::Pawn,
                }),
            );
            board.set(
                Square { file, rank: 4 },
                Some(Piece {
                    color: Color::Black,
                    kind: PieceKind::Pawn,
                }),
            );
            board.set(
                Square { file, rank: 5 },
                Some(Piece {
                    color: Color::Black,
                    kind,
                }),
            );
        }
        board
    }
}

impl Board {
    pub fn empty() -> Self {
        Self {
            squares: [[None; BOARD_SIZE]; BOARD_SIZE],
        }
    }
    pub fn get(&self, square: Square) -> Option<Piece> {
        self.squares[square.rank - 1][square.file]
    }
    pub fn set(&mut self, square: Square, piece: Option<Piece>) {
        self.squares[square.rank - 1][square.file] = piece;
    }
    pub fn piece_code(&self, square: Square) -> String {
        self.get(square)
            .map(|piece| {
                format!(
                    "{}{}",
                    if piece.color == Color::White {
                        'W'
                    } else {
                        'B'
                    },
                    piece.kind.code()
                )
            })
            .unwrap_or_else(|| "  ".to_owned())
    }
    pub fn apply(&mut self, move_: Move) -> Result<Piece, String> {
        let piece = self
            .get(move_.from)
            .ok_or_else(|| "No piece on the starting square.".to_owned())?;
        self.set(move_.from, None);
        self.set(
            move_.to,
            Some(Piece {
                kind: move_.promotion.unwrap_or(piece.kind),
                ..piece
            }),
        );
        Ok(piece)
    }
    pub fn king_square(&self, color: Color) -> Option<Square> {
        self.each_square().find(|&square| {
            self.get(square)
                == Some(Piece {
                    color,
                    kind: PieceKind::King,
                })
        })
    }
    pub fn piece_count(&self) -> usize {
        self.each_square()
            .filter(|&square| self.get(square).is_some())
            .count()
    }
    fn king_count(&self, color: Color) -> usize {
        self.each_square()
            .filter(|&square| {
                self.get(square)
                    == Some(Piece {
                        color,
                        kind: PieceKind::King,
                    })
            })
            .count()
    }
    pub fn each_square(&self) -> impl Iterator<Item = Square> {
        (1..=BOARD_SIZE).flat_map(|rank| (0..BOARD_SIZE).map(move |file| Square { file, rank }))
    }
    pub fn position_key(&self, turn: Color) -> String {
        let mut key = turn.name().to_owned();
        for square in self.each_square() {
            key.push_str(&self.piece_code(square));
        }
        key
    }

    /// Compact 108-bit state key for solver transposition tables.
    /// Five bits are not enough for the 13 square states, so each of the 25
    /// squares receives four bits; turn and halfmove clock occupy the rest.
    pub fn transposition_key(&self, turn: Color, halfmove_clock: u32) -> u128 {
        let mut key = 0_u128;
        for (index, square) in self.each_square().enumerate() {
            let code = match self.get(square) {
                None => 0,
                Some(Piece {
                    color: Color::White,
                    kind,
                }) => match kind {
                    PieceKind::King => 1,
                    PieceKind::Queen => 2,
                    PieceKind::Rook => 3,
                    PieceKind::Bishop => 4,
                    PieceKind::Knight => 5,
                    PieceKind::Pawn => 6,
                },
                Some(Piece {
                    color: Color::Black,
                    kind,
                }) => match kind {
                    PieceKind::King => 7,
                    PieceKind::Queen => 8,
                    PieceKind::Rook => 9,
                    PieceKind::Bishop => 10,
                    PieceKind::Knight => 11,
                    PieceKind::Pawn => 12,
                },
            };
            key |= (code as u128) << (index * 4);
        }
        key | ((turn == Color::Black) as u128) << 100 | (halfmove_clock.min(100) as u128) << 101
    }

    /// Canonical tablebase key under left-right board reflection.  File
    /// reflection preserves all 5x5 chess rules (including pawn direction),
    /// so the smaller packed form represents both symmetric positions.  This
    /// is intentionally *not* used for the active variation: a mirrored
    /// position is not a literal repetition in a played game.
    pub fn canonical_transposition_key(&self, turn: Color, halfmove_clock: u32) -> u128 {
        let normal = self.transposition_key(turn, halfmove_clock);
        let mut reflected = 0_u128;
        for (index, square) in self.each_square().enumerate() {
            let reflected_square = Square {
                file: BOARD_SIZE - 1 - square.file,
                rank: square.rank,
            };
            let code = match self.get(reflected_square) {
                None => 0,
                Some(Piece {
                    color: Color::White,
                    kind,
                }) => match kind {
                    PieceKind::King => 1,
                    PieceKind::Queen => 2,
                    PieceKind::Rook => 3,
                    PieceKind::Bishop => 4,
                    PieceKind::Knight => 5,
                    PieceKind::Pawn => 6,
                },
                Some(Piece {
                    color: Color::Black,
                    kind,
                }) => match kind {
                    PieceKind::King => 7,
                    PieceKind::Queen => 8,
                    PieceKind::Rook => 9,
                    PieceKind::Bishop => 10,
                    PieceKind::Knight => 11,
                    PieceKind::Pawn => 12,
                },
            };
            reflected |= (code as u128) << (index * 4);
        }
        reflected |= ((turn == Color::Black) as u128) << 100;
        reflected |= (halfmove_clock.min(100) as u128) << 101;
        normal.min(reflected)
    }

    /// Parses a five-rank FEN board field, such as `rnbqk/ppppp/5/PPPPP/RNBQK`.
    /// Uppercase pieces are White and lowercase pieces are Black.
    pub fn from_fen(fen: &str) -> Result<Self, String> {
        let ranks: Vec<_> = fen.split('/').collect();
        if ranks.len() != BOARD_SIZE {
            return Err("A 5x5 FEN position must contain exactly five ranks.".to_owned());
        }

        let mut board = Self::empty();
        for (rank_index, encoded_rank) in ranks.iter().enumerate() {
            let rank = BOARD_SIZE - rank_index;
            let mut file = 0;
            for symbol in encoded_rank.chars() {
                if let Some(empty_squares) = symbol.to_digit(10) {
                    file += empty_squares as usize;
                    continue;
                }
                let color = if symbol.is_ascii_uppercase() {
                    Color::White
                } else {
                    Color::Black
                };
                let kind = match symbol.to_ascii_uppercase() {
                    'K' => PieceKind::King,
                    'Q' => PieceKind::Queen,
                    'R' => PieceKind::Rook,
                    'B' => PieceKind::Bishop,
                    'N' => PieceKind::Knight,
                    'P' => PieceKind::Pawn,
                    _ => return Err(format!("Invalid FEN piece: {symbol}.")),
                };
                let square = Square::new(file, rank)
                    .ok_or_else(|| "A FEN rank has more than five squares.".to_owned())?;
                board.set(square, Some(Piece { color, kind }));
                file += 1;
            }
            if file != BOARD_SIZE {
                return Err("Every FEN rank must contain exactly five squares.".to_owned());
            }
        }
        if board.king_count(Color::White) != 1 || board.king_count(Color::Black) != 1 {
            return Err("A position must contain exactly one king of each colour.".to_owned());
        }
        Ok(board)
    }

    pub fn to_fen(&self) -> String {
        (1..=BOARD_SIZE)
            .rev()
            .map(|rank| {
                let mut encoded = String::new();
                let mut empty = 0;
                for file in 0..BOARD_SIZE {
                    match self.get(Square { file, rank }) {
                        None => empty += 1,
                        Some(piece) => {
                            if empty > 0 {
                                encoded.push(
                                    char::from_digit(empty, 10)
                                        .expect("board has at most five files"),
                                );
                                empty = 0;
                            }
                            let code = piece.kind.code();
                            encoded.push(if piece.color == Color::White {
                                code
                            } else {
                                code.to_ascii_lowercase()
                            });
                        }
                    }
                }
                if empty > 0 {
                    encoded
                        .push(char::from_digit(empty, 10).expect("board has at most five files"));
                }
                encoded
            })
            .collect::<Vec<_>>()
            .join("/")
    }
}

/// Verifies the input class handled by the five-piece cluster solver:
/// precisely both kings plus three other pieces, with a legal side to move.
///
/// This is intentionally a position-legality check, not a game-history
/// reconstruction. Promoted pieces are therefore allowed, while pawns on a
/// promotion rank and a side that has just left its own king in check are not.
pub fn validate_five_piece_position(board: &Board, turn: Color) -> Result<(), String> {
    if board.king_count(Color::White) != 1 || board.king_count(Color::Black) != 1 {
        return Err("position must contain exactly one white king and one black king".to_owned());
    }
    if board.piece_count() != 5 {
        return Err(format!(
            "position must contain exactly five pieces (two kings plus three others), found {}",
            board.piece_count()
        ));
    }
    if board.each_square().any(|square| {
        board.get(square).is_some_and(|piece| {
            piece.kind == PieceKind::Pawn && (square.rank == 1 || square.rank == BOARD_SIZE)
        })
    }) {
        return Err(
            "pawns may not be placed on rank 1 or rank 5; promote them in the FEN instead"
                .to_owned(),
        );
    }
    if is_in_check(board, turn.other()) {
        return Err(format!(
            "illegal position: {} is in check even though {} is to move",
            turn.other().name(),
            turn.name()
        ));
    }
    Ok(())
}

fn offset(square: Square, file_delta: isize, rank_delta: isize) -> Option<Square> {
    Square::new(
        square.file.checked_add_signed(file_delta)?,
        square.rank.checked_add_signed(rank_delta)?,
    )
}

fn step_moves_into(
    board: &Board,
    from: Square,
    color: Color,
    deltas: &[(isize, isize)],
    moves: &mut Vec<Square>,
) {
    for &(file, rank) in deltas {
        if let Some(to) = offset(from, file, rank)
            && board.get(to).is_none_or(|piece| piece.color != color)
        {
            moves.push(to);
        }
    }
}

fn sliding_moves_into(
    board: &Board,
    from: Square,
    color: Color,
    directions: &[(isize, isize)],
    moves: &mut Vec<Square>,
) {
    for &(file_delta, rank_delta) in directions {
        let mut current = from;
        while let Some(next) = offset(current, file_delta, rank_delta) {
            current = next;
            match board.get(current) {
                Some(piece) if piece.color == color => break,
                Some(_) => {
                    moves.push(current);
                    break;
                }
                None => moves.push(current),
            }
        }
    }
}

fn pseudo_moves_into(board: &Board, from: Square, moves: &mut Vec<Square>) {
    moves.clear();
    let Some(piece) = board.get(from) else {
        return;
    };
    match piece.kind {
        PieceKind::King => step_moves_into(
            board,
            from,
            piece.color,
            &[
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ],
            moves,
        ),
        PieceKind::Queen => sliding_moves_into(
            board,
            from,
            piece.color,
            &[
                (0, 1),
                (0, -1),
                (1, 0),
                (-1, 0),
                (1, 1),
                (1, -1),
                (-1, 1),
                (-1, -1),
            ],
            moves,
        ),
        PieceKind::Rook => sliding_moves_into(
            board,
            from,
            piece.color,
            &[(0, 1), (0, -1), (1, 0), (-1, 0)],
            moves,
        ),
        PieceKind::Bishop => sliding_moves_into(
            board,
            from,
            piece.color,
            &[(1, 1), (1, -1), (-1, 1), (-1, -1)],
            moves,
        ),
        PieceKind::Knight => step_moves_into(
            board,
            from,
            piece.color,
            &[
                (1, 2),
                (1, -2),
                (-1, 2),
                (-1, -2),
                (2, 1),
                (2, -1),
                (-2, 1),
                (-2, -1),
            ],
            moves,
        ),
        PieceKind::Pawn => pawn_moves_into(board, from, piece.color, moves),
    }
}

pub fn pseudo_moves(board: &Board, from: Square) -> Vec<Square> {
    let mut moves = Vec::with_capacity(16);
    pseudo_moves_into(board, from, &mut moves);
    moves
}

fn pawn_moves_into(board: &Board, from: Square, color: Color, moves: &mut Vec<Square>) {
    let direction = if color == Color::White { 1 } else { -1 };
    if let Some(to) = offset(from, 0, direction)
        && board.get(to).is_none()
    {
        moves.push(to);
    }
    for file_delta in [-1, 1] {
        if let Some(to) = offset(from, file_delta, direction)
            && board.get(to).is_some_and(|piece| piece.color != color)
        {
            moves.push(to);
        }
    }
}

pub fn pawn_moves(board: &Board, from: Square, color: Color) -> Vec<Square> {
    let mut moves = Vec::with_capacity(3);
    pawn_moves_into(board, from, color, &mut moves);
    moves
}

fn piece_attacks(board: &Board, from: Square, piece: Piece, target: Square) -> bool {
    let file_delta = target.file as isize - from.file as isize;
    let rank_delta = target.rank as isize - from.rank as isize;
    let absolute_file = file_delta.unsigned_abs();
    let absolute_rank = rank_delta.unsigned_abs();
    match piece.kind {
        PieceKind::Pawn => {
            rank_delta == if piece.color == Color::White { 1 } else { -1 } && absolute_file == 1
        }
        PieceKind::Knight => {
            (absolute_file == 1 && absolute_rank == 2) || (absolute_file == 2 && absolute_rank == 1)
        }
        PieceKind::King => {
            (absolute_file != 0 || absolute_rank != 0) && absolute_file <= 1 && absolute_rank <= 1
        }
        PieceKind::Rook | PieceKind::Bishop | PieceKind::Queen => {
            let straight = (file_delta == 0) != (rank_delta == 0);
            let diagonal = absolute_file == absolute_rank && absolute_file != 0;
            let aligned = match piece.kind {
                PieceKind::Rook => straight,
                PieceKind::Bishop => diagonal,
                PieceKind::Queen => straight || diagonal,
                _ => false,
            };
            if !aligned {
                return false;
            }
            let file_step = file_delta.signum();
            let rank_step = rank_delta.signum();
            let mut current = from;
            while let Some(next) = offset(current, file_step, rank_step) {
                if next == target {
                    return true;
                }
                if board.get(next).is_some() {
                    return false;
                }
                current = next;
            }
            false
        }
    }
}

pub fn is_attacked(board: &Board, target: Square, defending: Color) -> bool {
    let attacker = defending.other();
    board.each_square().any(|from| {
        let Some(piece) = board.get(from) else {
            return false;
        };
        piece.color == attacker && piece_attacks(board, from, piece, target)
    })
}

pub fn is_in_check(board: &Board, color: Color) -> bool {
    board
        .king_square(color)
        .is_some_and(|square| is_attacked(board, square, color))
}

pub fn legal_moves(board: &Board, color: Color) -> Vec<Move> {
    let mut moves = Vec::new();
    let mut targets = Vec::with_capacity(16);
    for from in board.each_square() {
        let Some(piece) = board.get(from) else {
            continue;
        };
        if piece.color != color {
            continue;
        }
        pseudo_moves_into(board, from, &mut targets);
        for &to in &targets {
            if board
                .get(to)
                .is_some_and(|target| target.kind == PieceKind::King)
            {
                continue;
            }
            if piece.kind == PieceKind::Pawn && (to.rank == 1 || to.rank == BOARD_SIZE) {
                for promotion in PieceKind::promotion_choices() {
                    let move_ = Move {
                        from,
                        to,
                        promotion: Some(promotion),
                    };
                    let mut next = *board;
                    next.apply(move_).expect("pseudo move has a source piece");
                    if !is_in_check(&next, color) {
                        moves.push(move_);
                    }
                }
            } else {
                let move_ = Move {
                    from,
                    to,
                    promotion: None,
                };
                let mut next = *board;
                next.apply(move_).expect("pseudo move has a source piece");
                if !is_in_check(&next, color) {
                    moves.push(move_);
                }
            }
        }
    }
    moves
}

pub fn is_checkmate(board: &Board, color: Color) -> bool {
    is_in_check(board, color) && legal_moves(board, color).is_empty()
}
pub fn is_stalemate(board: &Board, color: Color) -> bool {
    board.king_square(color).is_some()
        && !is_in_check(board, color)
        && legal_moves(board, color).is_empty()
}
pub fn is_threefold_repetition(history: &[String]) -> bool {
    history
        .iter()
        .any(|item| history.iter().filter(|other| *other == item).count() >= 3)
}
pub fn is_fifty_move_draw(halfmove_clock: u32) -> bool {
    halfmove_clock >= 100
}

pub fn is_insufficient_material(board: &Board) -> bool {
    let non_kings: Vec<(Square, Piece)> = board
        .each_square()
        .filter_map(|square| board.get(square).map(|piece| (square, piece)))
        .filter(|(_, piece)| piece.kind != PieceKind::King)
        .collect();
    if non_kings.is_empty() {
        return true;
    }
    if non_kings.iter().any(|(_, piece)| {
        matches!(
            piece.kind,
            PieceKind::Pawn | PieceKind::Rook | PieceKind::Queen
        )
    }) {
        return false;
    }
    if non_kings.len() == 1 {
        return matches!(non_kings[0].1.kind, PieceKind::Bishop | PieceKind::Knight);
    }
    if non_kings.len() == 2
        && non_kings
            .iter()
            .all(|(_, piece)| piece.kind == PieceKind::Bishop)
    {
        return (non_kings[0].0.file + non_kings[0].0.rank) % 2
            == (non_kings[1].0.file + non_kings[1].0.rank) % 2;
    }
    false
}

pub fn move_to_san(board: &Board, move_: Move, color: Color, legal: &[Move]) -> String {
    let piece = board.get(move_.from).expect("legal move has source");
    let capture = board.get(move_.to).is_some();
    let mut notation = if piece.kind == PieceKind::Pawn {
        String::new()
    } else {
        piece.kind.code().to_string()
    };
    if piece.kind != PieceKind::Pawn {
        let ambiguous: Vec<_> = legal
            .iter()
            .filter(|candidate| {
                candidate.from != move_.from
                    && candidate.to == move_.to
                    && board.get(candidate.from) == Some(piece)
            })
            .collect();
        if !ambiguous.is_empty() {
            let same_file = ambiguous
                .iter()
                .any(|candidate| candidate.from.file == move_.from.file);
            notation.push(if same_file {
                char::from_digit(move_.from.rank as u32, 10).unwrap()
            } else {
                FILES[move_.from.file]
            });
        }
    } else if capture {
        notation.push(FILES[move_.from.file]);
    }
    if capture {
        notation.push('x');
    }
    notation.push_str(&move_.to.to_string());
    if let Some(promotion) = move_.promotion {
        notation.push_str(&format!("={}", promotion.code()));
    }
    let _ = color;
    notation
}

pub fn check_suffix(board: &Board, player_to_move: Color) -> &'static str {
    if is_checkmate(board, player_to_move) {
        "#"
    } else if is_in_check(board, player_to_move) {
        "+"
    } else {
        ""
    }
}
pub fn export_pgn(moves: &[String], result: &str) -> String {
    let mut output = Vec::new();
    for (index, pair) in moves.chunks(2).enumerate() {
        output.push(format!(
            "{}. {}{}",
            index + 1,
            pair[0],
            pair.get(1)
                .map(|black| format!(" {black}"))
                .unwrap_or_default()
        ));
    }
    format!(
        "{}{}",
        output.join(" "),
        if output.is_empty() {
            result.to_owned()
        } else {
            format!(" {result}")
        }
    )
}

pub fn board_text(board: &Board) -> String {
    let mut lines = vec!["board:".to_owned()];
    for rank in (1..=BOARD_SIZE).rev() {
        lines.push(format!(
            "{rank}: {}",
            (0..BOARD_SIZE)
                .map(|file| board.piece_code(Square { file, rank }))
                .collect::<Vec<_>>()
                .join(" ")
        ));
    }
    lines.join("\n")
}

pub fn state_text(
    board: &Board,
    turn: Color,
    halfmove_clock: u32,
    history: &[String],
    san_moves: &[String],
) -> String {
    let moves = legal_moves(board, turn)
        .iter()
        .map(|move_| move_.label())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "turn: {}\nin_check: {}\ncheckmate: {}\nstalemate: {}\ninsufficient_material: {}\nfifty_move_draw: {}\nthreefold_repetition: {}\nhalfmove_clock: {}\nmove_count: {}\nlegal_moves: {}\npgn_moves: {}\n{}",
        turn.name(),
        is_in_check(board, turn),
        is_checkmate(board, turn),
        is_stalemate(board, turn),
        is_insufficient_material(board),
        is_fifty_move_draw(halfmove_clock),
        is_threefold_repetition(history),
        halfmove_clock,
        san_moves.len(),
        moves,
        san_moves.join(" "),
        board_text(board)
    )
}

/// The result from the point of view of the side to move.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Win,
    Draw,
    Loss,
}

impl fmt::Display for Outcome {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Win => "forced win",
            Self::Draw => "draw",
            Self::Loss => "forced loss",
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct SearchValue {
    outcome: Outcome,
    plies: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MoveEvaluation {
    pub move_: Move,
    pub outcome: Outcome,
    pub plies_to_result: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SolveResult {
    pub outcome: Outcome,
    pub plies_to_result: u32,
    /// All equally optimal root moves, not merely the first one encountered.
    pub best_moves: Vec<Move>,
    /// Values for every legal root move, in the engine's deterministic move order.
    pub moves: Vec<MoveEvaluation>,
    /// Search nodes expanded after transposition-table lookup.
    pub positions_analyzed: usize,
    /// Wall-clock time spent searching.
    pub elapsed: Duration,
    /// Search nodes expanded per second. Concurrent work can include a small
    /// number of duplicate in-flight expansions before a shared entry wins.
    pub positions_per_second: f64,
    /// Branches discarded because alpha-beta proved they cannot affect the result.
    pub alpha_beta_cutoffs: usize,
    pub cache_hits: usize,
    pub cache_misses: usize,
    pub legal_moves_generated: usize,
    pub maximum_depth: u32,
}

fn value_is_better(candidate: SearchValue, current: SearchValue) -> bool {
    candidate.score() > current.score()
        || (candidate.score() == current.score() && candidate.plies < current.plies)
}

fn values_are_equal(left: SearchValue, right: SearchValue) -> bool {
    left.outcome == right.outcome && left.plies == right.plies
}

#[inline]
fn published_plies(value: SearchValue) -> u32 {
    if value.outcome == Outcome::Draw {
        0
    } else {
        value.plies
    }
}

// The interactive API deliberately uses the clear `Board` representation
// above.  The exhaustive solver has radically different needs: it visits
// millions of ephemeral boards, so it uses the same four-bit square encoding
// as the transposition key directly.  This removes all per-node allocations
// and makes making/unmaking a move two masked integer operations.
#[derive(Clone, Copy)]
struct FastBoard {
    key: u128,
    mirror_key: u128,
    pieces: [u8; 25],
}

#[derive(Clone, Copy, PartialEq, Eq)]
struct FastMove {
    from: u8,
    to: u8,
    promotion: u8, // zero means retain the moving piece code
}

struct MoveList {
    moves: [FastMove; 64],
    len: usize,
}

impl MoveList {
    const EMPTY_MOVE: FastMove = FastMove {
        from: 0,
        to: 0,
        promotion: 0,
    };
    #[inline]
    fn new() -> Self {
        Self {
            moves: [Self::EMPTY_MOVE; 64],
            len: 0,
        }
    }
    #[inline]
    fn push(&mut self, from: u8, to: u8, promotion: u8) {
        debug_assert!(self.len < self.moves.len());
        self.moves[self.len] = FastMove {
            from,
            to,
            promotion,
        };
        self.len += 1;
    }

    /// Captures and promotions first makes alpha-beta see forcing replies
    /// before quiet moves.  This is deliberately allocation-free.
    fn order(&mut self, board: FastBoard, preferred: Option<FastMove>) {
        let mut scores = [0_i32; 64];
        for index in 0..self.len {
            let move_ = self.moves[index];
            let captured = board.piece(move_.to);
            scores[index] = if Some(move_) == preferred {
                1_000_000
            } else {
                0
            } + if captured == 0 {
                0
            } else {
                100_000 + i32::from(captured % 6)
            } + if move_.promotion == 0 {
                0
            } else {
                90_000 + i32::from(move_.promotion % 6)
            };
        }
        for index in 1..self.len {
            let move_ = self.moves[index];
            let score = scores[index];
            let mut slot = index;
            while slot > 0 && scores[slot - 1] < score {
                self.moves[slot] = self.moves[slot - 1];
                scores[slot] = scores[slot - 1];
                slot -= 1;
            }
            self.moves[slot] = move_;
            scores[slot] = score;
        }
    }
}

#[inline]
fn fast_piece_color(code: u8) -> Color {
    if code <= 6 {
        Color::White
    } else {
        Color::Black
    }
}

impl FastBoard {
    #[inline]
    fn from_board(board: &Board) -> Self {
        let key = board.transposition_key(Color::White, 0);
        let mut pieces = [0; 25];
        for square in 0..25 {
            pieces[square] = ((key >> (square * 4)) & 15) as u8;
        }
        let mut mirror_key = 0_u128;
        for square in 0..25_u8 {
            let mirror = square / 5 * 5 + (4 - square % 5);
            mirror_key |= u128::from(pieces[square as usize]) << (u32::from(mirror) * 4);
        }
        Self {
            key,
            mirror_key,
            pieces,
        }
    }
    #[inline]
    fn piece(self, square: u8) -> u8 {
        self.pieces[square as usize]
    }
    #[inline]
    fn key(self, turn: Color) -> u128 {
        self.key | ((turn == Color::Black) as u128) << 100
    }
    /// Left-right reflection preserves this variant's rules.  The boolean
    /// says whether the reflected position supplied the canonical key; a TT
    /// move is only reusable directly when it is false.
    #[inline]
    fn canonical_key(self, turn: Color) -> (u128, bool) {
        let turn_bit = ((turn == Color::Black) as u128) << 100;
        let normal = self.key | turn_bit;
        let reflected = self.mirror_key | turn_bit;
        if reflected < normal {
            (reflected, true)
        } else {
            (normal, false)
        }
    }
    #[inline]
    fn apply(self, move_: FastMove) -> Self {
        let moving = self.piece(move_.from);
        let piece = if move_.promotion == 0 {
            moving
        } else {
            move_.promotion
        };
        let from_shift = u32::from(move_.from) * 4;
        let to_shift = u32::from(move_.to) * 4;
        let mirror_from = move_.from / 5 * 5 + (4 - move_.from % 5);
        let mirror_to = move_.to / 5 * 5 + (4 - move_.to % 5);
        let mirror_from_shift = u32::from(mirror_from) * 4;
        let mirror_to_shift = u32::from(mirror_to) * 4;
        let mut next = self;
        next.key = (next.key & !(15_u128 << from_shift) & !(15_u128 << to_shift))
            | (u128::from(piece) << to_shift);
        next.mirror_key =
            (next.mirror_key & !(15_u128 << mirror_from_shift) & !(15_u128 << mirror_to_shift))
                | (u128::from(piece) << mirror_to_shift);
        next.pieces[move_.from as usize] = 0;
        next.pieces[move_.to as usize] = piece;
        next
    }
}

#[inline]
fn fast_square(file: i8, rank: i8) -> Option<u8> {
    if (0..5).contains(&file) && (0..5).contains(&rank) {
        Some((rank * 5 + file) as u8)
    } else {
        None
    }
}

fn fast_attacked(board: FastBoard, target: u8, defending: Color) -> bool {
    let target_file = (target % 5) as i8;
    let target_rank = (target / 5) as i8;
    let attacker = defending.other();
    for from in 0..25_u8 {
        let piece = board.piece(from);
        if piece == 0 || fast_piece_color(piece) != attacker {
            continue;
        }
        let kind = if piece <= 6 { piece } else { piece - 6 };
        let file = (from % 5) as i8;
        let rank = (from / 5) as i8;
        let df = target_file - file;
        let dr = target_rank - rank;
        let af = df.unsigned_abs();
        let ar = dr.unsigned_abs();
        match kind {
            1 if af <= 1 && ar <= 1 && (af != 0 || ar != 0) => return true,
            5 if (af == 1 && ar == 2) || (af == 2 && ar == 1) => return true,
            6 if af == 1 && dr == if attacker == Color::White { 1 } else { -1 } => return true,
            2 | 3 | 4 => {
                let straight = (df == 0) != (dr == 0);
                let diagonal = af == ar && af != 0;
                if !((kind == 2 && (straight || diagonal))
                    || (kind == 3 && straight)
                    || (kind == 4 && diagonal))
                {
                    continue;
                }
                let sf = df.signum();
                let sr = dr.signum();
                let mut f = file + sf;
                let mut r = rank + sr;
                loop {
                    let square = fast_square(f, r).expect("aligned ray remains on board");
                    if square == target {
                        return true;
                    }
                    if board.piece(square) != 0 {
                        break;
                    }
                    f += sf;
                    r += sr;
                }
            }
            _ => {}
        }
    }
    false
}

#[inline]
fn fast_in_check(board: FastBoard, color: Color) -> bool {
    let king = if color == Color::White { 1 } else { 7 };
    for square in 0..25_u8 {
        if board.piece(square) == king {
            return fast_attacked(board, square, color);
        }
    }
    false
}

fn fast_legal_moves(board: FastBoard, color: Color, moves: &mut MoveList) {
    moves.len = 0;
    for from in 0..25_u8 {
        let piece = board.piece(from);
        if piece == 0 || fast_piece_color(piece) != color {
            continue;
        }
        let kind = if piece <= 6 { piece } else { piece - 6 };
        let file = (from % 5) as i8;
        let rank = (from / 5) as i8;
        let add = |to: u8, promotion: u8, moves: &mut MoveList| {
            let target = board.piece(to);
            if target != 0 && fast_piece_color(target) == color {
                return;
            }
            if target == if color == Color::White { 7 } else { 1 } {
                return;
            }
            let move_ = FastMove {
                from,
                to,
                promotion,
            };
            if !fast_in_check(board.apply(move_), color) {
                moves.push(from, to, promotion);
            }
        };
        match kind {
            1 | 5 => {
                const KING: &[(i8, i8)] = &[
                    (0, 1),
                    (0, -1),
                    (1, 0),
                    (-1, 0),
                    (1, 1),
                    (1, -1),
                    (-1, 1),
                    (-1, -1),
                ];
                const KNIGHT: &[(i8, i8)] = &[
                    (1, 2),
                    (1, -2),
                    (-1, 2),
                    (-1, -2),
                    (2, 1),
                    (2, -1),
                    (-2, 1),
                    (-2, -1),
                ];
                for &(df, dr) in if kind == 1 { KING } else { KNIGHT } {
                    if let Some(to) = fast_square(file + df, rank + dr) {
                        add(to, 0, moves);
                    }
                }
            }
            2 | 3 | 4 => {
                const QUEEN: &[(i8, i8)] = &[
                    (0, 1),
                    (0, -1),
                    (1, 0),
                    (-1, 0),
                    (1, 1),
                    (1, -1),
                    (-1, 1),
                    (-1, -1),
                ];
                const ROOK: &[(i8, i8)] = &[(0, 1), (0, -1), (1, 0), (-1, 0)];
                const BISHOP: &[(i8, i8)] = &[(1, 1), (1, -1), (-1, 1), (-1, -1)];
                let dirs = if kind == 2 {
                    QUEEN
                } else if kind == 3 {
                    ROOK
                } else {
                    BISHOP
                };
                for &(df, dr) in dirs {
                    let mut f = file + df;
                    let mut r = rank + dr;
                    while let Some(to) = fast_square(f, r) {
                        let target = board.piece(to);
                        if target != 0 && fast_piece_color(target) == color {
                            break;
                        }
                        add(to, 0, moves);
                        if target != 0 {
                            break;
                        }
                        f += df;
                        r += dr;
                    }
                }
            }
            6 => {
                let direction = if color == Color::White { 1 } else { -1 };
                if let Some(to) = fast_square(file, rank + direction) {
                    if board.piece(to) == 0 {
                        if to / 5 == 0 || to / 5 == 4 {
                            for promotion in [2, 3, 4, 5] {
                                add(
                                    to,
                                    promotion + if color == Color::Black { 6 } else { 0 },
                                    moves,
                                );
                            }
                        } else {
                            add(to, 0, moves);
                        }
                    }
                }
                for df in [-1, 1] {
                    if let Some(to) = fast_square(file + df, rank + direction) {
                        let target = board.piece(to);
                        if target != 0 && fast_piece_color(target) != color {
                            if to / 5 == 0 || to / 5 == 4 {
                                for promotion in [2, 3, 4, 5] {
                                    add(
                                        to,
                                        promotion + if color == Color::Black { 6 } else { 0 },
                                        moves,
                                    );
                                }
                            } else {
                                add(to, 0, moves);
                            }
                        }
                    }
                }
            }
            _ => unreachable!(),
        }
    }
}

#[inline]
fn fast_insufficient_material(board: FastBoard) -> bool {
    let mut count = 0;
    let mut bishops = [0_u8; 2];
    let mut bishop_count = 0;
    for square in 0..25_u8 {
        let piece = board.piece(square);
        let kind = if piece <= 6 { piece } else { piece - 6 };
        if piece == 0 || kind == 1 {
            continue;
        }
        if matches!(kind, 6 | 3 | 2) {
            return false;
        }
        count += 1;
        if kind == 4 {
            bishops[bishop_count] = square;
            bishop_count += 1;
        }
    }
    count == 0
        || count == 1
        || (count == 2
            && bishop_count == 2
            && ((bishops[0] / 5 + bishops[0] % 5) & 1) == ((bishops[1] / 5 + bishops[1] % 5) & 1))
}

#[inline]
fn public_move(move_: FastMove) -> Move {
    let promotion = match move_.promotion % 6 {
        2 => Some(PieceKind::Queen),
        3 => Some(PieceKind::Rook),
        4 => Some(PieceKind::Bishop),
        5 => Some(PieceKind::Knight),
        _ => None,
    };
    Move {
        from: Square {
            file: (move_.from % 5) as usize,
            rank: (move_.from / 5 + 1) as usize,
        },
        to: Square {
            file: (move_.to % 5) as usize,
            rank: (move_.to / 5 + 1) as usize,
        },
        promotion,
    }
}

struct Solver {
    root: Color,
    table: TranspositionTable,
    active_path: PositionMap<u8>,
    positions_analyzed: usize,
    alpha_beta_cutoffs: usize,
    cache_hits: usize,
    cache_misses: usize,
    legal_moves_generated: usize,
    maximum_depth: u32,
}

#[derive(Clone, Copy)]
enum Bound {
    Exact,
    Lower,
    Upper,
}

#[derive(Clone, Copy)]
struct TableEntry {
    value: SearchValue,
    bound: Bound,
    best_move: Option<FastMove>,
}

#[inline]
fn merge_table_entry(existing: Option<TableEntry>, candidate: TableEntry) -> TableEntry {
    let Some(existing) = existing else {
        return candidate;
    };
    // An exact value is stronger than either alpha-beta bound.  In particular,
    // do not let a concurrently-completed cutoff overwrite a proven value.
    match (existing.bound, candidate.bound) {
        (Bound::Exact, _) => existing,
        (_, Bound::Exact) => candidate,
        (Bound::Lower, Bound::Lower) if existing.value.score() >= candidate.value.score() => {
            existing
        }
        (Bound::Upper, Bound::Upper) if existing.value.score() <= candidate.value.score() => {
            existing
        }
        _ => candidate,
    }
}

/// A striped table lets independent work items share proven values without a
/// global lock in the search hot path.  The key is the full packed position;
/// hashing only selects a stripe and can therefore never affect correctness.
struct SharedTranspositionTable {
    shards: Box<[Mutex<PositionMap<TableEntry>>]>,
}

enum TranspositionTable {
    Local(PositionMap<TableEntry>),
    Shared(Arc<SharedTranspositionTable>),
}

impl SharedTranspositionTable {
    const SHARDS: usize = 128;

    fn new() -> Arc<Self> {
        let mut shards = Vec::with_capacity(Self::SHARDS);
        for _ in 0..Self::SHARDS {
            shards.push(Mutex::new(PositionMap::default()));
        }
        Arc::new(Self {
            shards: shards.into_boxed_slice(),
        })
    }

    #[inline]
    fn shard(&self, key: u128) -> &Mutex<PositionMap<TableEntry>> {
        let mixed = (key as u64) ^ ((key >> 64) as u64).rotate_left(29);
        // SHARDS is a power of two, avoiding an integer division per probe.
        &self.shards[(mixed as usize) & (Self::SHARDS - 1)]
    }

    #[inline]
    fn get(&self, key: u128) -> Option<TableEntry> {
        self.shard(key)
            .lock()
            .expect("transposition-table mutex poisoned")
            .get(&key)
            .copied()
    }

    #[inline]
    fn insert(&self, key: u128, entry: TableEntry) {
        let mut shard = self
            .shard(key)
            .lock()
            .expect("transposition-table mutex poisoned");
        let merged = merge_table_entry(shard.get(&key).copied(), entry);
        shard.insert(key, merged);
    }
}

impl TranspositionTable {
    #[inline]
    fn get(&self, key: u128) -> Option<TableEntry> {
        match self {
            Self::Local(table) => table.get(&key).copied(),
            Self::Shared(table) => table.get(key),
        }
    }

    #[inline]
    fn insert(&mut self, key: u128, entry: TableEntry) {
        match self {
            Self::Local(table) => {
                // A single DFS owns its local table, so a direct insert keeps
                // the one-hash-probe hot path used by the original solver.
                table.insert(key, entry);
            }
            Self::Shared(table) => table.insert(key, entry),
        }
    }
}

struct WorkerResult {
    evaluations: Vec<(usize, usize, SearchValue)>,
    solver: Solver,
}

#[derive(Clone, Copy)]
struct SearchTask {
    root_index: usize,
    reply_index: usize,
    board: Board,
    turn: Color,
    halfmove_clock: u32,
}

impl SearchValue {
    const MATE_SCORE: i32 = 1_000_000;

    fn score(self) -> i32 {
        match self.outcome {
            Outcome::Win => Self::MATE_SCORE - self.plies as i32,
            Outcome::Draw => 0,
            Outcome::Loss => -Self::MATE_SCORE + self.plies as i32,
        }
    }

    /// Advance a search ply. Draw-distance normalization happens only when a
    /// result is published, keeping this hot path branch-free.
    #[inline]
    fn advance(self) -> Self {
        Self {
            outcome: self.outcome,
            plies: self.plies + 1,
        }
    }
}

impl Solver {
    fn new(root: Color) -> Self {
        Self::with_table(root, TranspositionTable::Local(PositionMap::default()))
    }

    fn with_shared_table(root: Color, table: Arc<SharedTranspositionTable>) -> Self {
        Self::with_table(root, TranspositionTable::Shared(table))
    }

    fn with_table(root: Color, table: TranspositionTable) -> Self {
        Self {
            root,
            table,
            active_path: PositionMap::default(),
            positions_analyzed: 0,
            alpha_beta_cutoffs: 0,
            cache_hits: 0,
            cache_misses: 0,
            legal_moves_generated: 0,
            maximum_depth: 0,
        }
    }

    fn solve(
        &mut self,
        board: FastBoard,
        turn: Color,
        halfmove_clock: u32,
        mut alpha: i32,
        mut beta: i32,
        depth: u32,
    ) -> SearchValue {
        let original_alpha = alpha;
        let original_beta = beta;
        self.maximum_depth = self.maximum_depth.max(depth);
        // The online DFS shares placement states and uses an active-variation
        // cycle guard. Canonical keys are reserved for the path-independent
        // retrograde tablebase, where a reflected position is safe to merge.
        let repetition_key = board.key(turn);
        let (key, reflected) = board.canonical_key(turn);
        let mut preferred_move = None;
        if self.active_path.get(&repetition_key).copied().unwrap_or(0) >= 2 {
            return SearchValue {
                outcome: Outcome::Draw,
                plies: 0,
            };
        }
        if let Some(entry) = self.table.get(key) {
            self.cache_hits += 1;
            preferred_move = if reflected { None } else { entry.best_move };
            match entry.bound {
                Bound::Exact => return entry.value,
                Bound::Lower => alpha = alpha.max(entry.value.score()),
                Bound::Upper => beta = beta.min(entry.value.score()),
            }
            if alpha >= beta {
                return entry.value;
            }
        }
        self.cache_misses += 1;
        // The recursive fallback needs a finite horizon. Canonical tablebase
        // generation uses a separate retrograde pass and does not apply this.
        if fast_insufficient_material(board) || is_fifty_move_draw(halfmove_clock) {
            let value = SearchValue {
                outcome: Outcome::Draw,
                plies: 0,
            };
            self.table.insert(
                key,
                TableEntry {
                    value,
                    bound: Bound::Exact,
                    best_move: None,
                },
            );
            return value;
        }

        // Generate legal moves exactly once. An empty list distinguishes mate
        // from stalemate using only the already-required attack test.
        let mut legal = MoveList::new();
        fast_legal_moves(board, turn, &mut legal);
        legal.order(board, preferred_move);
        self.legal_moves_generated += legal.len;
        if legal.len == 0 {
            let value = SearchValue {
                outcome: if fast_in_check(board, turn) {
                    if turn == self.root {
                        Outcome::Loss
                    } else {
                        Outcome::Win
                    }
                } else {
                    Outcome::Draw
                },
                plies: 0,
            };
            self.table.insert(
                key,
                TableEntry {
                    value,
                    bound: Bound::Exact,
                    best_move: None,
                },
            );
            return value;
        }

        self.positions_analyzed += 1;
        *self.active_path.entry(repetition_key).or_insert(0) += 1;
        let mut best: Option<SearchValue> = None;
        let mut best_move = None;
        let maximizing = turn == self.root;
        for move_ in legal.moves[..legal.len].iter().copied() {
            let moving_piece = board.piece(move_.from);
            let captured = board.piece(move_.to) != 0;
            let next = board.apply(move_);
            let next_clock =
                if (moving_piece - if moving_piece > 6 { 6 } else { 0 }) == 6 || captured {
                    0
                } else {
                    halfmove_clock + 1
                };
            let child = self.solve(next, turn.other(), next_clock, alpha, beta, depth + 1);
            let value = child.advance();
            if best.is_none_or(|current| {
                if maximizing {
                    value_is_better(value, current)
                } else {
                    value_is_better(current, value)
                }
            }) {
                best = Some(value);
                best_move = Some(move_);
            }
            let score = best.expect("at least one child exists").score();
            if maximizing {
                alpha = alpha.max(score);
            } else {
                beta = beta.min(score);
            }
            if alpha >= beta {
                self.alpha_beta_cutoffs += 1;
                break;
            }
        }
        let active_count = self
            .active_path
            .get_mut(&repetition_key)
            .expect("active position exists");
        *active_count -= 1;
        if *active_count == 0 {
            self.active_path.remove(&repetition_key);
        }
        let value = best.expect("non-terminal positions have at least one legal move");
        // Preserve alpha-beta bounds in the TT. Reusing them avoids revisiting
        // subtrees that were previously cut off, while Exact entries still
        // provide the final game-theoretic result.
        let bound = if value.score() <= original_alpha {
            Bound::Upper
        } else if value.score() >= original_beta {
            Bound::Lower
        } else {
            Bound::Exact
        };
        self.table.insert(
            key,
            TableEntry {
                value,
                bound,
                best_move: if reflected { None } else { best_move },
            },
        );
        value
    }
}

/// Exhaustively minimaxes every reachable legal position from `board`.
///
/// The solver uses a transposition table, evaluates every legal continuation,
/// and treats a third occurrence on the current variation as a draw.  Large,
/// material-rich positions can take a long time to solve; endgames are the
/// most practical inputs for full exhaustive analysis.
pub fn solve_position(board: &Board, turn: Color, halfmove_clock: u32) -> SolveResult {
    let started_at = Instant::now();
    let mut solver = Solver::new(turn);
    let board = FastBoard::from_board(board);
    let mut root_moves = MoveList::new();
    fast_legal_moves(board, turn, &mut root_moves);
    solver.legal_moves_generated += root_moves.len;
    let mut evaluations = Vec::with_capacity(root_moves.len);
    let mut best: Option<SearchValue> = None;

    for move_ in root_moves.moves[..root_moves.len].iter().copied() {
        let moving_piece = board.piece(move_.from);
        let captured = board.piece(move_.to) != 0;
        let next = board.apply(move_);
        let next_clock = if (moving_piece - if moving_piece > 6 { 6 } else { 0 }) == 6 || captured {
            0
        } else {
            halfmove_clock + 1
        };
        // Every root move is searched with a full window so the displayed
        // per-move result is exact, while alpha-beta prunes each subtree.
        let child = solver.solve(
            next,
            turn.other(),
            next_clock,
            -SearchValue::MATE_SCORE,
            SearchValue::MATE_SCORE,
            1,
        );
        let value = child.advance();
        if best.is_none_or(|current| value_is_better(value, current)) {
            best = Some(value);
        }
        evaluations.push((move_, value));
    }

    let root_value = if let Some(value) = best {
        value
    } else if fast_in_check(board, turn) {
        SearchValue {
            outcome: Outcome::Loss,
            plies: 0,
        }
    } else {
        SearchValue {
            outcome: Outcome::Draw,
            plies: 0,
        }
    };
    let best_moves = evaluations
        .iter()
        .filter(|(_, value)| values_are_equal(*value, root_value))
        .map(|(move_, _)| public_move(*move_))
        .collect();
    let moves = evaluations
        .into_iter()
        .map(|(move_, value)| MoveEvaluation {
            move_: public_move(move_),
            outcome: value.outcome,
            plies_to_result: published_plies(value),
        })
        .collect();
    let elapsed = started_at.elapsed();
    let positions_per_second = if elapsed.is_zero() {
        0.0
    } else {
        solver.positions_analyzed as f64 / elapsed.as_secs_f64()
    };
    SolveResult {
        outcome: root_value.outcome,
        plies_to_result: published_plies(root_value),
        best_moves,
        moves,
        positions_analyzed: solver.positions_analyzed,
        elapsed,
        positions_per_second,
        alpha_beta_cutoffs: solver.alpha_beta_cutoffs,
        cache_hits: solver.cache_hits,
        cache_misses: solver.cache_misses,
        legal_moves_generated: solver.legal_moves_generated,
        maximum_depth: solver.maximum_depth,
    }
}

/// Runs a shared-table, two-ply work-stealing search.  Splitting beneath the
/// root provides far more tasks than the number of legal root moves, avoids
/// the former 13-worker ceiling, and gives slow branches to idle workers.
///
/// The table is striped rather than globally locked.  A worker can still race
/// another worker which is evaluating the same not-yet-cached state, but all
/// completed exact/bound entries are immediately reusable by every worker.
pub fn solve_position_parallel_legacy(
    board: &Board,
    turn: Color,
    halfmove_clock: u32,
    workers: usize,
) -> SolveResult {
    let root_moves = legal_moves(board, turn);
    if workers <= 1 || root_moves.len() <= 1 {
        return solve_position(board, turn, halfmove_clock);
    }

    let started_at = Instant::now();
    let mut root_terminal_values = vec![None; root_moves.len()];
    let mut reply_counts = vec![0_usize; root_moves.len()];
    let mut tasks = Vec::new();

    // Make tasks at ply two.  This retains exact root aggregation: the side
    // to move after a root move chooses the minimum value over every reply.
    for (root_index, root_move) in root_moves.iter().copied().enumerate() {
        let moving_piece = board
            .get(root_move.from)
            .expect("legal root move has a source");
        let captured = board.get(root_move.to).is_some();
        let mut after_root = *board;
        after_root
            .apply(root_move)
            .expect("legal root move applies");
        let root_clock = if moving_piece.kind == PieceKind::Pawn || captured {
            0
        } else {
            halfmove_clock + 1
        };
        let replies = legal_moves(&after_root, turn.other());
        if replies.is_empty() {
            let child = SearchValue {
                outcome: if is_in_check(&after_root, turn.other()) {
                    Outcome::Win
                } else {
                    Outcome::Draw
                },
                plies: 0,
            };
            root_terminal_values[root_index] = Some(child.advance());
            continue;
        }
        reply_counts[root_index] = replies.len();
        for (reply_index, reply) in replies.into_iter().enumerate() {
            let reply_piece = after_root
                .get(reply.from)
                .expect("legal reply has a source");
            let reply_captured = after_root.get(reply.to).is_some();
            let mut after_reply = after_root;
            after_reply.apply(reply).expect("legal reply applies");
            let reply_clock = if reply_piece.kind == PieceKind::Pawn || reply_captured {
                0
            } else {
                root_clock + 1
            };
            tasks.push(SearchTask {
                root_index,
                reply_index,
                board: after_reply,
                turn,
                halfmove_clock: reply_clock,
            });
        }
    }

    // Every root child was terminal; this is uncommon but avoids creating a
    // zero-worker scope and preserves the normal result shape.
    if tasks.is_empty() {
        return solve_position(board, turn, halfmove_clock);
    }

    let shared_table = SharedTranspositionTable::new();
    let task_index = AtomicUsize::new(0);
    let worker_count = workers.min(tasks.len());
    let worker_outputs = thread::scope(|scope| {
        let tasks = &tasks;
        let mut handles = Vec::with_capacity(worker_count);
        for _ in 0..worker_count {
            let task_index = &task_index;
            let shared_table = Arc::clone(&shared_table);
            handles.push(scope.spawn(move || {
                let mut solver = Solver::with_shared_table(turn, shared_table);
                let mut evaluations = Vec::new();
                loop {
                    let index = task_index.fetch_add(1, Ordering::Relaxed);
                    if index >= tasks.len() {
                        break;
                    }
                    let task = tasks[index];
                    let child = solver.solve(
                        FastBoard::from_board(&task.board),
                        task.turn,
                        task.halfmove_clock,
                        -SearchValue::MATE_SCORE,
                        SearchValue::MATE_SCORE,
                        2,
                    );
                    evaluations.push((
                        task.root_index,
                        task.reply_index,
                        child.advance().advance(),
                    ));
                }
                WorkerResult {
                    evaluations,
                    solver,
                }
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("solver worker panicked"))
            .collect::<Vec<_>>()
    });

    let mut reply_values = reply_counts
        .iter()
        .map(|&count| vec![None; count])
        .collect::<Vec<Vec<Option<SearchValue>>>>();
    for (root_index, reply_index, value) in worker_outputs
        .iter()
        .flat_map(|output| output.evaluations.iter().copied())
    {
        reply_values[root_index][reply_index] = Some(value);
    }
    let evaluations = root_moves
        .iter()
        .copied()
        .enumerate()
        .map(|(root_index, move_)| {
            let value = root_terminal_values[root_index].unwrap_or_else(|| {
                reply_values[root_index]
                    .iter()
                    .flatten()
                    .copied()
                    .reduce(|best, candidate| {
                        if value_is_better(best, candidate) {
                            candidate
                        } else {
                            best
                        }
                    })
                    .expect("each non-terminal root move has every reply evaluated")
            });
            (move_, value)
        })
        .collect::<Vec<_>>();
    let positions_analyzed = worker_outputs
        .iter()
        .map(|output| output.solver.positions_analyzed)
        .sum();
    let alpha_beta_cutoffs = worker_outputs
        .iter()
        .map(|output| output.solver.alpha_beta_cutoffs)
        .sum();
    let cache_hits = worker_outputs
        .iter()
        .map(|output| output.solver.cache_hits)
        .sum();
    let cache_misses = worker_outputs
        .iter()
        .map(|output| output.solver.cache_misses)
        .sum();
    let legal_moves_generated = root_moves.len()
        + reply_counts.iter().sum::<usize>()
        + worker_outputs
            .iter()
            .map(|output| output.solver.legal_moves_generated)
            .sum::<usize>();
    let maximum_depth = worker_outputs
        .iter()
        .map(|output| output.solver.maximum_depth)
        .max()
        .unwrap_or(0);
    let root_value = evaluations
        .iter()
        .map(|(_, value)| *value)
        .max_by_key(|value| value.score())
        .unwrap_or(SearchValue {
            outcome: if is_checkmate(board, turn) {
                Outcome::Loss
            } else {
                Outcome::Draw
            },
            plies: 0,
        });
    let best_moves = evaluations
        .iter()
        .filter(|(_, value)| values_are_equal(*value, root_value))
        .map(|(move_, _)| *move_)
        .collect();
    let moves = evaluations
        .into_iter()
        .map(|(move_, value)| MoveEvaluation {
            move_,
            outcome: value.outcome,
            plies_to_result: published_plies(value),
        })
        .collect();
    let elapsed = started_at.elapsed();
    SolveResult {
        outcome: root_value.outcome,
        plies_to_result: published_plies(root_value),
        best_moves,
        moves,
        positions_analyzed,
        elapsed,
        positions_per_second: if elapsed.is_zero() {
            0.0
        } else {
            positions_analyzed as f64 / elapsed.as_secs_f64()
        },
        alpha_beta_cutoffs,
        cache_hits,
        cache_misses,
        legal_moves_generated,
        maximum_depth,
    }
}

/// Exhaustively solve a position using up to `workers` CPU workers.  Work is
/// split below the root so an Iridis allocation can use more workers than the
/// number of legal first moves.
pub fn solve_position_parallel(
    board: &Board,
    turn: Color,
    halfmove_clock: u32,
    workers: usize,
) -> SolveResult {
    solve_position_parallel_legacy(board, turn, halfmove_clock, workers)
}

/// Solve a legal K+K+three-piece position. This is the cluster-facing entry
/// point: it rejects malformed or non-five-piece FENs before starting the
/// exhaustive parallel search.
pub fn solve_five_piece_position(
    board: &Board,
    turn: Color,
    halfmove_clock: u32,
    workers: usize,
) -> Result<SolveResult, String> {
    validate_five_piece_position(board, turn)?;
    Ok(solve_position_parallel(
        board,
        turn,
        halfmove_clock,
        workers,
    ))
}

/// Summary emitted after building a material-partition WDL tablebase.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TablebaseBuildStats {
    pub positions: usize,
    pub wins: usize,
    pub losses: usize,
    pub draws: usize,
}

/// Build an exact, clock-independent KXvK WDL tablebase, where X is one
/// queen, rook, bishop, or knight.
///
/// This is the first persistent retrograde partition in the complete-solver
/// pipeline.  The queen may belong to either side.  Capturing it leads to the
/// known K-v-K draw subtable; checkmate and stalemate are terminal anchors.
/// The binary output is sorted by packed position key and consists of the
/// `DPKQK1` header followed by `(u128 little-endian key, u8 WDL)` records,
/// where WDL is 0=loss, 1=draw, 2=win from the side to move.
fn build_one_piece_layer(
    piece_kinds: &[PieceKind],
    path: impl AsRef<Path>,
) -> std::io::Result<TablebaseBuildStats> {
    #[derive(Clone, Copy)]
    struct State {
        board: Board,
        turn: Color,
    }
    const UNKNOWN: u8 = 3;
    const LOSS: u8 = 0;
    const DRAW: u8 = 1;
    const WIN: u8 = 2;

    let mut states = Vec::new();
    let mut index = PositionMap::default();
    debug_assert!(piece_kinds.iter().all(|kind| matches!(
        kind,
        PieceKind::Pawn
            | PieceKind::Queen
            | PieceKind::Rook
            | PieceKind::Bishop
            | PieceKind::Knight
    )));
    for &piece_kind in piece_kinds {
        for queen_color in [Color::White, Color::Black] {
            for white_king in 0..25 {
                for black_king in 0..25 {
                    if black_king == white_king {
                        continue;
                    }
                    for queen in 0..25 {
                        if queen == white_king || queen == black_king {
                            continue;
                        }
                        let mut board = Board::empty();
                        let square = |n: usize| Square {
                            file: n % 5,
                            rank: n / 5 + 1,
                        };
                        board.set(
                            square(white_king),
                            Some(Piece {
                                color: Color::White,
                                kind: PieceKind::King,
                            }),
                        );
                        board.set(
                            square(black_king),
                            Some(Piece {
                                color: Color::Black,
                                kind: PieceKind::King,
                            }),
                        );
                        board.set(
                            square(queen),
                            Some(Piece {
                                color: queen_color,
                                kind: piece_kind,
                            }),
                        );
                        // Both kings in check is illegal, as is a position where
                        // the player who just moved left their own king attacked.
                        if is_in_check(&board, Color::White) && is_in_check(&board, Color::Black) {
                            continue;
                        }
                        for turn in [Color::White, Color::Black] {
                            if is_in_check(&board, turn.other()) {
                                continue;
                            }
                            let key = board.transposition_key(turn, 0);
                            let next_index = states.len();
                            index.insert(key, next_index);
                            states.push(State { board, turn });
                        }
                    }
                }
            }
        }
    }

    let mut predecessors = vec![Vec::<usize>::new(); states.len()];
    let mut remaining = vec![0_u16; states.len()];
    let mut value = vec![UNKNOWN; states.len()];
    let mut queue = VecDeque::new();

    for (state_index, state) in states.iter().enumerate() {
        let moves = legal_moves(&state.board, state.turn);
        if moves.is_empty() {
            value[state_index] = if is_in_check(&state.board, state.turn) {
                LOSS
            } else {
                DRAW
            };
            if value[state_index] == LOSS {
                queue.push_back(state_index);
            }
            continue;
        }
        for move_ in moves {
            let captured_queen = state
                .board
                .get(move_.to)
                .is_some_and(|piece| piece.kind != PieceKind::King);
            if captured_queen {
                // K-v-K is an already solved draw successor.  It must count
                // toward the outdegree so this state can never be inferred
                // as a loss merely from its in-partition edges.
                remaining[state_index] += 1;
                continue;
            }
            let mut child = state.board;
            child.apply(move_).expect("legal move applies");
            let child_key = child.transposition_key(state.turn.other(), 0);
            let child_index = *index
                .get(&child_key)
                .expect("KXvK move remains in its material partition");
            predecessors[child_index].push(state_index);
            remaining[state_index] += 1;
        }
    }

    while let Some(child) = queue.pop_front() {
        let child_value = value[child];
        for &parent in &predecessors[child] {
            if value[parent] != UNKNOWN {
                continue;
            }
            if child_value == LOSS {
                value[parent] = WIN;
                queue.push_back(parent);
            } else {
                remaining[parent] -= 1;
                if remaining[parent] == 0 {
                    value[parent] = LOSS;
                    queue.push_back(parent);
                }
            }
        }
    }
    for outcome in &mut value {
        if *outcome == UNKNOWN {
            *outcome = DRAW;
        }
    }

    let mut records = states
        .iter()
        .enumerate()
        .map(|(i, state)| {
            let outcome = value[i];
            let best_move = legal_moves(&state.board, state.turn)
                .into_iter()
                .find(|move_| {
                    let captured = state.board.get(move_.to);
                    let mut child = state.board;
                    child.apply(*move_).expect("legal move applies");
                    let child_outcome =
                        if captured.is_some_and(|piece| piece.kind != PieceKind::King) {
                            TB_DRAW
                        } else {
                            let child_key = child.transposition_key(state.turn.other(), 0);
                            value[*index.get(&child_key).expect("child is indexed")]
                        };
                    matches!(
                        (outcome, child_outcome),
                        (TB_WIN, TB_LOSS) | (TB_DRAW, TB_DRAW) | (TB_LOSS, TB_WIN)
                    )
                });
            (
                state.board.transposition_key(state.turn, 0),
                outcome,
                best_move,
            )
        })
        .collect::<Vec<_>>();
    records.sort_unstable_by_key(|(key, _, _)| *key);
    let mut output = BufWriter::new(File::create(path)?);
    output.write_all(b"DPKXK2\0")?;
    output.write_all(&(records.len() as u64).to_le_bytes())?;
    for (key, outcome, best_move) in records {
        output.write_all(&key.to_le_bytes())?;
        output.write_all(&[outcome])?;
        let encoded = best_move
            .map(|move_| {
                let from = ((move_.from.rank - 1) * 5 + move_.from.file) as u8;
                let to = ((move_.to.rank - 1) * 5 + move_.to.file) as u8;
                let promotion = move_
                    .promotion
                    .map(|kind| match kind {
                        PieceKind::Queen => 2,
                        PieceKind::Rook => 3,
                        PieceKind::Bishop => 4,
                        PieceKind::Knight => 5,
                        _ => 0,
                    })
                    .unwrap_or(0);
                [from, to, promotion]
            })
            .unwrap_or([255, 255, 0]);
        output.write_all(&encoded)?;
    }
    output.flush()?;
    Ok(TablebaseBuildStats {
        positions: value.len(),
        wins: value.iter().filter(|&&outcome| outcome == WIN).count(),
        losses: value.iter().filter(|&&outcome| outcome == LOSS).count(),
        draws: value.iter().filter(|&&outcome| outcome == DRAW).count(),
    })
}

pub fn build_kqk_wdl(path: impl AsRef<Path>) -> std::io::Result<TablebaseBuildStats> {
    build_one_piece_layer(&[PieceKind::Queen], path)
}

pub fn build_krk_wdl(path: impl AsRef<Path>) -> std::io::Result<TablebaseBuildStats> {
    build_one_piece_layer(&[PieceKind::Rook], path)
}

pub fn build_kbk_wdl(path: impl AsRef<Path>) -> std::io::Result<TablebaseBuildStats> {
    build_one_piece_layer(&[PieceKind::Bishop], path)
}

pub fn build_knk_wdl(path: impl AsRef<Path>) -> std::io::Result<TablebaseBuildStats> {
    build_one_piece_layer(&[PieceKind::Knight], path)
}

/// Solve all 0/1-non-king-piece positions as one promotion-aware layer.
/// A pawn reaching the last rank remains inside this graph as its promoted
/// queen, rook, bishop, or knight; captures transition to the K-v-K draw.
pub fn build_one_piece_wdl(path: impl AsRef<Path>) -> std::io::Result<TablebaseBuildStats> {
    build_one_piece_layer(
        &[
            PieceKind::Pawn,
            PieceKind::Knight,
            PieceKind::Bishop,
            PieceKind::Rook,
            PieceKind::Queen,
        ],
        path,
    )
}

/// Result of probing a persistent WDL tablebase. `Unknown` means that no
/// matching material partition has been generated or loaded yet.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TablebaseProbe {
    Win { best_move: Option<Move> },
    Draw { best_move: Option<Move> },
    Loss { best_move: Option<Move> },
    Unknown,
}

const TB_LOSS: u8 = 0;
const TB_DRAW: u8 = 1;
const TB_WIN: u8 = 2;

#[derive(Clone, Copy)]
struct DiskTableEntry {
    outcome: u8,
    best_move: Option<Move>,
}

static KQK_TABLE: OnceLock<Option<PositionMap<DiskTableEntry>>> = OnceLock::new();
static KRK_TABLE: OnceLock<Option<PositionMap<DiskTableEntry>>> = OnceLock::new();
static KBK_TABLE: OnceLock<Option<PositionMap<DiskTableEntry>>> = OnceLock::new();
static KNK_TABLE: OnceLock<Option<PositionMap<DiskTableEntry>>> = OnceLock::new();

fn tablebase_directory() -> String {
    std::env::var("DEEP_PAWN_TABLEBASE_DIR").unwrap_or_else(|_| "tablebases".to_owned())
}

fn load_wdl_table(filename: &str) -> Option<PositionMap<DiskTableEntry>> {
    let bytes = fs::read(Path::new(&tablebase_directory()).join(filename)).ok()?;
    // Seven-byte NUL-terminated magic followed immediately by u64 count.
    if bytes.len() < 15 {
        return None;
    }
    let record_size = if &bytes[..6] == b"DPKXK2" {
        20
    } else if &bytes[..6] == b"DPKXK1" || &bytes[..6] == b"DPKQK1" {
        17
    } else {
        return None;
    };
    let count = u64::from_le_bytes(bytes[7..15].try_into().ok()?) as usize;
    if bytes.len() != 15 + count * record_size {
        return None;
    }
    let mut table = PositionMap::default();
    table.reserve(count);
    for record in bytes[15..].chunks_exact(record_size) {
        let key = u128::from_le_bytes(record[..16].try_into().expect("record has key"));
        let outcome = record[16];
        if outcome > TB_WIN {
            return None;
        }
        let best_move = if record_size == 20 && record[17] < 25 && record[18] < 25 {
            let promotion = match record[19] {
                0 => None,
                2 => Some(PieceKind::Queen),
                3 => Some(PieceKind::Rook),
                4 => Some(PieceKind::Bishop),
                5 => Some(PieceKind::Knight),
                _ => return None,
            };
            Some(Move {
                from: Square {
                    file: (record[17] % 5) as usize,
                    rank: (record[17] / 5 + 1) as usize,
                },
                to: Square {
                    file: (record[18] % 5) as usize,
                    rank: (record[18] / 5 + 1) as usize,
                },
                promotion,
            })
        } else {
            None
        };
        table.insert(key, DiskTableEntry { outcome, best_move });
    }
    Some(table)
}

fn table_for_piece(kind: PieceKind) -> Option<&'static PositionMap<DiskTableEntry>> {
    match kind {
        PieceKind::Queen => KQK_TABLE
            .get_or_init(|| load_wdl_table("kqk.dptb"))
            .as_ref(),
        PieceKind::Rook => KRK_TABLE
            .get_or_init(|| load_wdl_table("krk.dptb"))
            .as_ref(),
        PieceKind::Bishop => KBK_TABLE
            .get_or_init(|| load_wdl_table("kbk.dptb"))
            .as_ref(),
        PieceKind::Knight => KNK_TABLE
            .get_or_init(|| load_wdl_table("knk.dptb"))
            .as_ref(),
        _ => None,
    }
}

fn three_piece_kind(board: &Board) -> Option<PieceKind> {
    let pieces = board
        .each_square()
        .filter_map(|square| board.get(square))
        .collect::<Vec<_>>();
    if pieces.len() != 3
        || pieces
            .iter()
            .filter(|piece| piece.kind == PieceKind::King)
            .count()
            != 2
    {
        return None;
    }
    pieces
        .into_iter()
        .find(|piece| piece.kind != PieceKind::King)
        .map(|piece| piece.kind)
}

fn probe_wdl_code(board: &Board, turn: Color) -> Option<u8> {
    let kind = three_piece_kind(board)?;
    table_for_piece(kind)?
        .get(&board.transposition_key(turn, 0))
        .map(|entry| entry.outcome)
}

/// Probe the loaded 3-piece tablebase for `board`.  Values and the optional
/// move are from the side-to-move perspective. Set `DEEP_PAWN_TABLEBASE_DIR`
/// before first probing to use a non-default tablebase directory.
pub fn probe_tablebase(board: &Board, turn: Color) -> TablebaseProbe {
    let Some(outcome) = probe_wdl_code(board, turn) else {
        return TablebaseProbe::Unknown;
    };
    let kind = three_piece_kind(board).expect("probe established a 3-piece signature");
    let stored_move = table_for_piece(kind)
        .and_then(|table| table.get(&board.transposition_key(turn, 0)))
        .and_then(|entry| entry.best_move);
    let best_move = stored_move.or_else(|| {
        legal_moves(board, turn).into_iter().find(|move_| {
            let captured = board.get(move_.to);
            let mut child = *board;
            child.apply(*move_).expect("legal move applies");
            let child_outcome = if captured.is_some_and(|piece| piece.kind == kind) {
                Some(TB_DRAW) // K-v-K
            } else {
                probe_wdl_code(&child, turn.other())
            };
            matches!(
                (outcome, child_outcome),
                (TB_WIN, Some(TB_LOSS)) | (TB_DRAW, Some(TB_DRAW)) | (TB_LOSS, Some(TB_WIN))
            )
        })
    });
    match outcome {
        TB_WIN => TablebaseProbe::Win { best_move },
        TB_DRAW => TablebaseProbe::Draw { best_move },
        TB_LOSS => TablebaseProbe::Loss { best_move },
        _ => unreachable!(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn place(board: &mut Board, square: &str, color: Color, kind: PieceKind) {
        board.set(Square::parse(square).unwrap(), Some(Piece { color, kind }));
    }

    #[test]
    fn board_and_piece_bounds_are_five_by_five() {
        let board = Board::default();
        assert_eq!(legal_moves(&board, Color::White).len(), 7);
        assert_eq!(legal_moves(&board, Color::Black).len(), 7);
        assert!(Square::parse("e5").is_some());
        assert!(Square::parse("f5").is_none());
        assert!(Square::parse("a6").is_none());
    }

    #[test]
    fn pawns_only_move_one_square_and_promote_on_last_rank() {
        let mut board = Board::empty();
        place(&mut board, "c2", Color::White, PieceKind::Pawn);
        place(&mut board, "d3", Color::Black, PieceKind::Pawn);
        let moves = pawn_moves(&board, Square::parse("c2").unwrap(), Color::White);
        assert!(moves.contains(&Square::parse("c3").unwrap()));
        assert!(!moves.contains(&Square::parse("c4").unwrap()));
        assert!(moves.contains(&Square::parse("d3").unwrap()));

        let mut promotion = Board::empty();
        place(&mut promotion, "e1", Color::White, PieceKind::King);
        place(&mut promotion, "e5", Color::Black, PieceKind::King);
        place(&mut promotion, "a4", Color::White, PieceKind::Pawn);
        assert_eq!(
            legal_moves(&promotion, Color::White)
                .iter()
                .filter(|move_| move_.from == Square::parse("a4").unwrap())
                .count(),
            4
        );
    }

    #[test]
    fn king_safety_and_endings_work() {
        let mut checkmate = Board::empty();
        place(&mut checkmate, "e5", Color::Black, PieceKind::King);
        place(&mut checkmate, "c3", Color::White, PieceKind::King);
        place(&mut checkmate, "d4", Color::White, PieceKind::Queen);
        assert!(is_checkmate(&checkmate, Color::Black));

        let mut stalemate = Board::empty();
        place(&mut stalemate, "e5", Color::Black, PieceKind::King);
        place(&mut stalemate, "c4", Color::White, PieceKind::King);
        place(&mut stalemate, "d3", Color::White, PieceKind::Queen);
        assert!(is_stalemate(&stalemate, Color::Black));
    }

    #[test]
    fn pgn_and_draw_helpers_work() {
        assert_eq!(
            export_pgn(&["e3".to_owned(), "e4".to_owned()], "*"),
            "1. e3 e4 *"
        );
        assert!(is_threefold_repetition(&[
            "a".to_owned(),
            "b".to_owned(),
            "a".to_owned(),
            "a".to_owned()
        ]));
        assert!(is_fifty_move_draw(100));
    }

    #[test]
    fn fen_and_minimax_solver_work_for_a_forced_mate() {
        let board = Board::from_fen("4k/3Q1/2K2/5/5").unwrap();
        assert_eq!(board.to_fen(), "4k/3Q1/2K2/5/5");
        let solved = solve_position(&board, Color::Black, 0);
        assert_eq!(solved.outcome, Outcome::Loss);
        assert_eq!(solved.plies_to_result, 0);
        assert!(solved.moves.is_empty());
    }

    #[test]
    fn minimax_evaluates_a_non_terminal_position() {
        let board = Board::from_fen("4k/5/2K2/3Q1/5").unwrap();
        let solved = solve_position(&board, Color::White, 0);
        assert_eq!(solved.outcome, Outcome::Win);
        assert_eq!(solved.plies_to_result, 1);
        assert_eq!(
            solved.best_moves,
            vec![Move {
                from: Square::parse("d2").unwrap(),
                to: Square::parse("d4").unwrap(),
                promotion: None,
            }]
        );
        assert!(solved.positions_analyzed > 1);
        assert!(solved.alpha_beta_cutoffs > 0);
        assert_eq!(solved.moves.len(), 17);
    }

    #[test]
    fn five_piece_solver_input_requires_legal_kings_and_three_other_pieces() {
        let legal = Board::from_fen("4k/5/2K2/1QRN1/5").unwrap();
        assert_eq!(legal.piece_count(), 5);
        assert!(validate_five_piece_position(&legal, Color::White).is_ok());

        let too_few = Board::from_fen("4k/5/2K2/3Q1/5").unwrap();
        assert!(
            validate_five_piece_position(&too_few, Color::White)
                .unwrap_err()
                .contains("exactly five pieces")
        );

        let opponent_in_check = Board::from_fen("4k/5/2K2/1NN1Q/5").unwrap();
        assert!(
            validate_five_piece_position(&opponent_in_check, Color::White)
                .unwrap_err()
                .contains("black is in check")
        );
    }

    #[test]
    fn parallel_and_serial_keep_the_same_forced_best_move() {
        let board = Board::from_fen("4k/5/2K2/3Q1/5").unwrap();
        let serial = solve_position(&board, Color::White, 0);
        let parallel = solve_position_parallel(&board, Color::White, 0, 4);
        assert_eq!(parallel.outcome, serial.outcome);
        assert_eq!(parallel.best_moves, serial.best_moves);
    }
}
