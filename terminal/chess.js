const fs = require("fs");
const path = require("path");
const readline = require("readline");

const functions = require("../functions");
const castlingMoves = require("../castling");
const enPassantMoves = require("../en_passant");
const pawn = require("../pieces/pawn");
const pgn = require("./pgn");
const textState = require("./text_state");

const defaultBoard = JSON.parse(
    fs.readFileSync(path.join(__dirname, "../board/default_board.json"), "utf8")
);

const pieces = {
    K: require("../pieces/king"),
    Q: require("../pieces/queen"),
    R: require("../pieces/rook"),
    B: require("../pieces/bishop"),
    N: require("../pieces/knight"),
    P: pawn
};

const symbols = {
    WK: "♔", WQ: "♕", WR: "♖", WB: "♗", WN: "♘", WP: "♙",
    BK: "♚", BQ: "♛", BR: "♜", BB: "♝", BN: "♞", BP: "♟",
    "  ": " "
};

let board;
let turn;
let halfmoveClock;
let positionHistory;
let sanMoves;

function reset() {
    board = JSON.parse(JSON.stringify(defaultBoard));
    board.moves = [];
    turn = "white";
    halfmoveClock = 0;
    positionHistory = [positionKey()];
    sanMoves = [];
}

function positionKey() {
    const position = {};
    for (const rank of Object.keys(board)) {
        if (rank !== "moves") position[rank] = board[rank];
    }
    return JSON.stringify({ turn, position });
}

function printBoard() {
    console.log("\n    a b c d e f g h");
    console.log("   -----------------");
    for (let rank = 8; rank >= 1; rank--) {
        const row = [];
        for (const file of "abcdefgh") {
            row.push(symbols[board[String(rank)]?.[file]] || "?");
        }
        console.log(`${rank}  ${row.join(" ")}  ${rank}`);
    }
    console.log("   -----------------");
    console.log("    a b c d e f g h\n");
}

function moveLabel(move) {
    let label = `${move.from.file}${move.from.rank}-${move.to.file}${move.to.rank}`;
    if (move.castle) label += ` (${move.castle} castle)`;
    if (move.enPassant) label += " (en passant)";
    if (move.promotion) label += `=${move.promotion}`;
    return label;
}

function status() {
    const inCheck = functions.isInCheck(board, turn);
    const checkmate = functions.isCheckmate(board, turn);
    const stalemate = functions.isStalemate(board, turn);
    const insufficient = functions.isInsufficientMaterial(board);
    const repetition = functions.isThreefoldRepetition(positionHistory);
    const fiftyMove = functions.isFiftyMoveDraw(halfmoveClock);

    console.log(`Turn: ${turn}${inCheck ? " — CHECK" : ""}`);
    if (checkmate) console.log(`CHECKMATE — ${turn === "white" ? "Black" : "White"} wins.`);
    if (stalemate) console.log("STALEMATE — draw.");
    if (insufficient) console.log("DRAW — insufficient material.");
    if (repetition) console.log("DRAW CLAIM — threefold repetition.");
    if (fiftyMove) console.log("DRAW CLAIM — fifty-move rule.");
}

function resultCode() {
    if (functions.isCheckmate(board, turn)) return turn === "white" ? "0-1" : "1-0";
    if (functions.isStalemate(board, turn) ||
        functions.isInsufficientMaterial(board) ||
        functions.isThreefoldRepetition(positionHistory) ||
        functions.isFiftyMoveDraw(halfmoveClock)) return "1/2-1/2";
    return "*";
}

function showMoves() {
    const legalMoves = functions.allLegalMoves(board, turn);
    if (!legalMoves.length) {
        console.log("No legal moves.");
        return;
    }
    console.log(`\n${turn} legal moves (${legalMoves.length}):`);
    console.log(legalMoves.map(moveLabel).join("  "));
}

function sameSquare(a, b) {
    return a?.file === b?.file && Number(a?.rank) === Number(b?.rank);
}

function findMove(from, to, promotion) {
    return functions.allLegalMoves(board, turn).find(move =>
        sameSquare(move.from, from) &&
        sameSquare(move.to, to) &&
        (move.promotion || null) === (promotion || null)
    );
}

function applyMove(move) {
    const source = board[String(move.from.rank)][move.from.file];
    const target = board[String(move.to.rank)][move.to.file];
    const capture = move.enPassant || target !== "  ";

    board[String(move.from.rank)][move.from.file] = "  ";
    if (move.enPassant) {
        board[String(move.capture.rank)][move.capture.file] = "  ";
    }

    board[String(move.to.rank)][move.to.file] = move.promotion
        ? `${turn === "white" ? "W" : "B"}${move.promotion}`
        : source;

    if (move.castle) {
        const rook = board[String(move.rook.from.rank)][move.rook.from.file];
        board[String(move.rook.from.rank)][move.rook.from.file] = "  ";
        board[String(move.rook.to.rank)][move.rook.to.file] = rook;
        board.moves.push({ from: move.rook.from, to: move.rook.to });
    }

    board.moves.push({ from: move.from, to: move.to });
    halfmoveClock = source[1] === "P" || capture ? 0 : halfmoveClock + 1;
    turn = turn === "white" ? "black" : "white";
    positionHistory.push(positionKey());
}

function playMove(args) {
    if (functions.isGameOver(board, turn) ||
        functions.isInsufficientMaterial(board) ||
        functions.isThreefoldRepetition(positionHistory) ||
        functions.isFiftyMoveDraw(halfmoveClock)) {
        console.log("The game is over or a draw can be claimed. Use reset to start again.");
        return;
    }

    if (args.length < 2) {
        console.log("Usage: move e2 e4 [Q|R|B|N]");
        return;
    }

    const from = { file: args[0][0], rank: Number(args[0][1]) };
    const to = { file: args[1][0], rank: Number(args[1][1]) };
    let promotion = args[2]?.toUpperCase();
    const promotionMove = functions.allLegalMoves(board, turn).find(move =>
        sameSquare(move.from, from) && sameSquare(move.to, to) && move.promotion
    );

    if (promotionMove && !promotion) {
        promotion = "Q";
        console.log("Promotion not specified; defaulting to queen.");
    }

    const legalMoves = functions.allLegalMoves(board, turn);
    const move = legalMoves.find(candidate =>
        sameSquare(candidate.from, from) &&
        sameSquare(candidate.to, to) &&
        (candidate.promotion || null) === (promotion || null)
    );
    if (!move) {
        console.log("Illegal move. Use `moves` to list legal moves.");
        return;
    }

    const notation = pgn.moveToSan(board, move, turn, legalMoves);
    applyMove(move);
    sanMoves.push(notation + pgn.checkSuffix(board, turn));
    printBoard();
    status();
}

function printPgn() {
    console.log(pgn.exportPgn(sanMoves, resultCode()));
}

function savePgn(filename) {
    const target = filename || "game.pgn";
    fs.writeFileSync(path.resolve(target), pgn.exportPgn(sanMoves, resultCode()) + "\n");
    console.log(`PGN saved to ${path.resolve(target)}`);
}

function printText() {
    console.log(textState.stateText(board, turn, halfmoveClock, positionHistory, sanMoves));
}

function saveText(filename) {
    const target = filename || "game.txt";
    fs.writeFileSync(path.resolve(target), textState.stateText(board, turn, halfmoveClock, positionHistory, sanMoves) + "\n");
    console.log(`Text state saved to ${path.resolve(target)}`);
}

function debug(args) {
    const name = args.shift();
    const file = args[0]?.[0];
    const rank = Number(args[0]?.[1]);
    const player = args[1] || turn;

    const result = {
        allLegalMoves: () => functions.allLegalMoves(board, player),
        isInCheck: () => functions.isInCheck(board, player),
        isCheckmate: () => functions.isCheckmate(board, player),
        isStalemate: () => functions.isStalemate(board, player),
        isAttacked: () => functions.isAttacked(board, file, rank, player),
        hasPiece: () => functions.hasPiece(board, file, rank),
        selfPiece: () => functions.selfPiece(board, file, rank, player),
        validSquare: () => functions.validSquare(board, file, rank),
        castlingMoves: () => castlingMoves(board, player),
        enPassantMoves: () => enPassantMoves(board, file, rank, player),
        canMoveTwoSquares: () => pawn.canMoveTwoSquares(board, file, rank, player),
        isInsufficientMaterial: () => functions.isInsufficientMaterial(board),
        isFiftyMoveDraw: () => functions.isFiftyMoveDraw(halfmoveClock),
        isThreefoldRepetition: () => functions.isThreefoldRepetition(positionHistory),
        kingMoves: () => pieces.K(board, file, rank, player),
        queenMoves: () => pieces.Q(board, file, rank, player),
        rookMoves: () => pieces.R(board, file, rank, player),
        bishopMoves: () => pieces.B(board, file, rank, player),
        knightMoves: () => pieces.N(board, file, rank, player),
        pawnMoves: () => pieces.P(board, file, rank, player)
    };

    if (name === "isPromotionMove") {
        const move = JSON.parse(args.slice(2).join(" "));
        console.log(JSON.stringify(pawn.isPromotionMove(move, player), null, 2));
        return;
    }

    if (!result[name]) {
        console.log("Unknown function. Use: debug help");
        return;
    }

    console.log(JSON.stringify(result[name](), null, 2));
}

function help() {
    console.log(`
Commands:
  show                         Display the board and game status
  moves                        List all legal moves for the current player
  move e2 e4                  Play a move
  move e7 e8 Q                Promote to a chosen piece
  pgn                          Print the current game as PGN
  savepgn [file]               Save PGN to a file
  text                         Print an ASCII/text-only game state
  savetext [file]              Save the text state to a file
  debug <function> <square>   Call an engine function directly
  reset                        Reset to the standard starting position
  help                         Show this help
  quit                         Exit

Debug examples:
  debug pawnMoves e2 white
  debug allLegalMoves - white
  debug isAttacked e4 white
  debug castlingMoves - white
  debug isCheckmate - black
  debug canMoveTwoSquares e2 white
`);
}

reset();
const input = readline.createInterface({ input: process.stdin, output: process.stdout });
console.log("Chess terminal debugger");
help();
printBoard();
status();
input.setPrompt("chess> ");
input.prompt();
input.on("line", line => {
    const args = line.trim().split(/\s+/).filter(Boolean);
    const command = args.shift();

    try {
        if (command === "show") { printBoard(); status(); }
        else if (command === "moves") showMoves();
        else if (command === "move") playMove(args);
        else if (command === "pgn") printPgn();
        else if (command === "savepgn") savePgn(args[0]);
        else if (command === "text") printText();
        else if (command === "savetext") saveText(args[0]);
        else if (command === "debug") args[0] === "help" ? help() : debug(args);
        else if (command === "reset") { reset(); printBoard(); status(); }
        else if (command === "help") help();
        else if (command === "quit" || command === "exit") input.close();
        else if (command) console.log("Unknown command. Type `help`.");
    } catch (error) {
        console.log(`Error: ${error.message}`);
    }

    if (!input.closed) input.prompt();
});

input.on("close", () => {
    console.log("Goodbye.");
});
