# Chess Engine

A terminal-first chess engine written in Node.js. The project calculates legal moves and supports the main chess rules, including check, checkmate, stalemate, castling, en passant, promotion, pawn double-steps, and draw detection.

## Features

- Legal move generation for all six piece types
- King-safety filtering for every move
- Castling with move-history and attacked-square validation
- En passant using the immediately previous move
- Pawn promotion to queen, rook, bishop, or knight
- Check, checkmate, and stalemate detection
- Insufficient-material detection
- Fifty-move and threefold-repetition checks
- Interactive terminal chess game
- Direct terminal debugging for engine functions
- PGN export
- Structured text-only state export for future AI training

## Requirements

- Node.js 18 or newer

No external npm packages are required.

## Quick start

From the repository root:

```bash
node terminal/chess.js
```

## Terminal commands

```text
show                         Display the board and game status
moves                        List legal moves for the current player
move e2 e4                   Play a move
move e7 e8 Q                 Promote a pawn to a chosen piece
pgn                          Print the current game as PGN
savepgn [file]               Save the game as a PGN file
text                         Print the text-only game state
savetext [file]              Save the text state to a file
debug <function> <arguments> Call an engine function directly
reset                        Reset to the starting position
help                         Display command help
quit                         Exit
```

Examples:

```text
debug pawnMoves e2 white
debug allLegalMoves - white
debug isAttacked e4 white
debug castlingMoves - white
debug isCheckmate - black
```

## Engine API

The main engine functions are exported from [`functions.js`](./functions.js):

```js
const chess = require("./functions");

chess.allLegalMoves(board, "white");
chess.isInCheck(board, "white");
chess.isCheckmate(board, "white");
chess.isStalemate(board, "white");
chess.isAttacked(board, "e", 4, "white");
chess.isInsufficientMaterial(board);
chess.isFiftyMoveDraw(halfmoveClock);
chess.isThreefoldRepetition(positionHistory);
```

Additional rule modules:

- [`castling.js`](./castling.js)
- [`en_passant.js`](./en_passant.js)
- [`pieces/pawn.js`](./pieces/pawn.js)

## Board format

The board is an object keyed by rank. Each square contains a two-character piece code:

```json
{
  "8": { "a": "BR", "b": "BN" },
  "7": { "a": "BP" },
  "1": { "e": "WK" },
  "moves": []
}
```

Piece codes use `W` or `B` followed by:

```text
K King    Q Queen    R Rook
B Bishop  N Knight   P Pawn
```

Move history entries use:

```js
{
  from: { file: "e", rank: 2 },
  to: { file: "e", rank: 4 }
}
```

The `halfmove_clock` counts halfmoves since the last pawn move or capture. It resets to zero after either event and reaches the fifty-move threshold at 100 halfmoves.

## Tests

Run the test suite with:

```bash
node test.js
```

The tests cover piece movement, blocking, captures, king safety, special moves, checkmate, stalemate, and draw rules.

## Project structure

```text
.
├── board/
│   ├── default_board.json
│   └── render.js
├── pieces/
│   ├── bishop.js
│   ├── king.js
│   ├── knight.js
│   ├── pawn.js
│   ├── queen.js
│   └── rook.js
├── terminal/
│   ├── chess.js
│   ├── pgn.js
│   ├── text_state.js
│   └── README.md
├── castling.js
├── en_passant.js
├── functions.js
└── test.js
```

## Contributing

Please see [CONTRIBUTING.md](./CONTRIBUTING.md) for the development workflow and pull-request requirements. The repository also includes a [Code of Conduct](./CODE_OF_CONDUCT.md), [Security Policy](./SECURITY.md), [bug report template](./.github/ISSUE_TEMPLATE/bug_report.md), and [feature request template](./.github/ISSUE_TEMPLATE/feature_request.md).

## License

No license has been selected for this repository yet. Add a `LICENSE` file before publishing if you want to grant reuse rights to others.
