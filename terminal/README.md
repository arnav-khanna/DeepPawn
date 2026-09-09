# Terminal chess debugger

Run from the project root:

```bash
node terminal/chess.js
```

The terminal application uses the existing engine functions directly and supports legal movement, check, checkmate, stalemate, castling, en passant, promotion, pawn double-steps, insufficient material, the fifty-move rule, and threefold repetition.

Useful commands:

```text
show
moves
move e2 e4
debug pawnMoves e2 white
debug allLegalMoves - white
debug isAttacked e4 white
pgn
savepgn game.pgn
text
savetext game.txt
reset
quit
```

`text` and `savetext` produce an ASCII, key-value state containing the board, turn, check status, legal moves, draw status, move history, and PGN moves for later training or replay tooling.
