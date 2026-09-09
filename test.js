const assert = require("assert");
const fs = require("fs");

const functions = require("./functions");
const castlingMoves = require("./castling");
const enPassantMoves = require("./en_passant");
const pieces = {
    king: require("./pieces/king"),
    queen: require("./pieces/queen"),
    rook: require("./pieces/rook"),
    bishop: require("./pieces/bishop"),
    knight: require("./pieces/knight"),
    pawn: require("./pieces/pawn")
};

const files = ["a", "b", "c", "d", "e", "f", "g", "h"];

function emptyBoard() {
    return Object.fromEntries(
        Array.from({ length: 8 }, (_, index) => [
            String(index + 1),
            Object.fromEntries(files.map(file => [file, "  "]))
        ])
    );
}

function put(board, file, rank, piece) {
    board[String(rank)][file] = piece;
}

function hasMove(moves, file, rank) {
    return moves.some(move => move.file === file && move.rank === rank);
}

function testBoardFunctions() {
    const board = emptyBoard();
    put(board, "e", 4, "WK");
    put(board, "f", 4, "BP");

    assert.strictEqual(functions.hasPiece(board, "e", 4), true);
    assert.strictEqual(functions.hasPiece(board, "a", 1), false);
    assert.strictEqual(functions.selfPiece(board, "e", 4, "white"), true);
    assert.strictEqual(functions.selfPiece(board, "f", 4, "white"), false);
    assert.strictEqual(functions.selfPiece(board, "f", 4, "black"), true);
    assert.strictEqual(functions.validSquare(board, "a", 1), true);
    assert.strictEqual(functions.validSquare(board, "i", 1), false);
}

function testKing() {
    const board = emptyBoard();
    put(board, "d", 4, "WK");
    put(board, "e", 4, "WP");
    put(board, "c", 4, "BP");

    const moves = pieces.king(board, "d", 4, "white");
    assert.strictEqual(moves.length, 7);
    assert.strictEqual(hasMove(moves, "e", 4), false);
    assert.strictEqual(hasMove(moves, "c", 4), true);
}

function testQueen() {
    const board = emptyBoard();
    put(board, "d", 4, "WQ");
    put(board, "f", 4, "BP");
    put(board, "b", 4, "WP");

    const moves = pieces.queen(board, "d", 4, "white");
    assert.strictEqual(hasMove(moves, "f", 4), true);
    assert.strictEqual(hasMove(moves, "g", 4), false);
    assert.strictEqual(hasMove(moves, "b", 4), false);
    assert.strictEqual(hasMove(moves, "a", 4), false);
    assert.strictEqual(hasMove(moves, "e", 5), true);
}

function testRook() {
    const board = emptyBoard();
    put(board, "d", 4, "WR");
    put(board, "d", 6, "BP");

    const moves = pieces.rook(board, "d", 4, "white");
    assert.strictEqual(moves.length, 12);
    assert.strictEqual(hasMove(moves, "d", 6), true);
    assert.strictEqual(hasMove(moves, "d", 7), false);
    assert.strictEqual(hasMove(moves, "e", 4), true);
    assert.strictEqual(hasMove(moves, "e", 5), false);
}

function testBishop() {
    const board = emptyBoard();
    put(board, "d", 4, "WB");
    put(board, "f", 6, "BP");

    const moves = pieces.bishop(board, "d", 4, "white");
    assert.strictEqual(moves.length, 11);
    assert.strictEqual(hasMove(moves, "f", 6), true);
    assert.strictEqual(hasMove(moves, "g", 7), false);
    assert.strictEqual(hasMove(moves, "e", 4), false);
}

function testKnight() {
    const board = emptyBoard();
    put(board, "d", 4, "WN");
    put(board, "f", 5, "WP");

    const moves = pieces.knight(board, "d", 4, "white");
    assert.strictEqual(moves.length, 7);
    assert.strictEqual(hasMove(moves, "f", 5), false);
    assert.strictEqual(hasMove(moves, "f", 3), true);
}

function testPawn() {
    const board = emptyBoard();
    put(board, "d", 2, "WP");
    put(board, "e", 3, "BP");

    assert.strictEqual(pieces.pawn.canMoveTwoSquares(board, "d", 2, "white"), true);

    const moves = pieces.pawn(board, "d", 2, "white");
    assert.strictEqual(hasMove(moves, "d", 3), true);
    assert.strictEqual(hasMove(moves, "d", 4), true);
    assert.strictEqual(hasMove(moves, "e", 3), true);

    put(board, "d", 3, "BP");
    assert.strictEqual(pieces.pawn.canMoveTwoSquares(board, "d", 2, "white"), false);
    assert.strictEqual(hasMove(pieces.pawn(board, "d", 2, "white"), "d", 4), false);

    assert.strictEqual(pieces.pawn.isPromotionMove({
        from: { file: "a", rank: 7 },
        to: { file: "a", rank: 8 }
    }, "white"), true);
    assert.strictEqual(pieces.pawn.isPromotionMove({
        from: { file: "a", rank: 2 },
        to: { file: "a", rank: 1 }
    }, "black"), true);
    assert.strictEqual(pieces.pawn.isPromotionMove({
        from: { file: "a", rank: 6 },
        to: { file: "a", rank: 7 }
    }, "white"), false);

    const promotionBoard = emptyBoard();
    promotionBoard.moves = [];
    put(promotionBoard, "e", 1, "WK");
    put(promotionBoard, "h", 8, "BK");
    put(promotionBoard, "a", 7, "WP");
    const promotionMoves = functions.allLegalMoves(promotionBoard, "white")
        .filter(move => move.from.file === "a" && move.from.rank === "7");
    assert.deepStrictEqual(
        promotionMoves.map(move => move.promotion).sort(),
        ["B", "N", "Q", "R"]
    );
}

function testAttackAndAllMoves() {
    const board = emptyBoard();
    put(board, "e", 8, "BR");
    assert.strictEqual(functions.isAttacked(board, "e", 1, "white"), true);
    assert.strictEqual(functions.isAttacked(board, "a", 1, "white"), false);

    const checkBoard = emptyBoard();
    put(checkBoard, "e", 1, "WK");
    put(checkBoard, "a", 8, "BK");
    put(checkBoard, "e", 8, "BR");

    const checkMoves = functions.allLegalMoves(checkBoard, "white");
    assert.strictEqual(checkMoves.some(move =>
        move.from.file === "e" && move.from.rank === "1" &&
        move.to.file === "e" && move.to.rank === 2
    ), false);

    const pinnedBoard = emptyBoard();
    put(pinnedBoard, "e", 1, "WK");
    put(pinnedBoard, "a", 8, "BK");
    put(pinnedBoard, "e", 2, "WR");
    put(pinnedBoard, "e", 8, "BR");

    const pinnedMoves = functions.allLegalMoves(pinnedBoard, "white");
    assert.strictEqual(pinnedMoves.some(move =>
        move.from.file === "e" && move.from.rank === "2" &&
        move.to.file === "d" && move.to.rank === 2
    ), false);

    const defaultBoard = JSON.parse(
        fs.readFileSync("./board/default_board.json", "utf8")
    );
    assert.strictEqual(functions.allLegalMoves(defaultBoard, "white").length, 20);
    assert.strictEqual(functions.allLegalMoves(defaultBoard, "black").length, 20);
    assert.deepStrictEqual(functions.allLegalMoves(defaultBoard, "purple"), []);
}

function testCheckmateAndStalemate() {
    const checkmate = emptyBoard();
    put(checkmate, "h", 8, "BK");
    put(checkmate, "f", 6, "WK");
    put(checkmate, "g", 7, "WQ");

    assert.strictEqual(functions.isInCheck(checkmate, "black"), true);
    assert.strictEqual(functions.isCheckmate(checkmate, "black"), true);
    assert.strictEqual(functions.isStalemate(checkmate, "black"), false);

    const stalemate = emptyBoard();
    put(stalemate, "h", 8, "BK");
    put(stalemate, "f", 7, "WK");
    put(stalemate, "g", 6, "WQ");

    assert.strictEqual(functions.isInCheck(stalemate, "black"), false);
    assert.strictEqual(functions.isCheckmate(stalemate, "black"), false);
    assert.strictEqual(functions.isStalemate(stalemate, "black"), true);
}

function testDrawRules() {
    assert.strictEqual(functions.isThreefoldRepetition(["a", "b", "a", "a"]), true);
    assert.strictEqual(functions.isThreefoldRepetition(["a", "b", "a"]), false);
    assert.strictEqual(functions.isFiftyMoveDraw(100), true);
    assert.strictEqual(functions.isFiftyMoveDraw(99), false);

    const kingsOnly = emptyBoard();
    put(kingsOnly, "e", 1, "WK");
    put(kingsOnly, "e", 8, "BK");
    assert.strictEqual(functions.isInsufficientMaterial(kingsOnly), true);

    const bishopEnding = emptyBoard();
    put(bishopEnding, "e", 1, "WK");
    put(bishopEnding, "e", 8, "BK");
    put(bishopEnding, "c", 1, "WB");
    assert.strictEqual(functions.isInsufficientMaterial(bishopEnding), true);

    put(bishopEnding, "f", 8, "BB");
    assert.strictEqual(functions.isInsufficientMaterial(bishopEnding), true);

    put(bishopEnding, "c", 8, "BP");
    assert.strictEqual(functions.isInsufficientMaterial(bishopEnding), false);
}

function testCastling() {
    const board = emptyBoard();
    board.moves = [];
    put(board, "e", 1, "WK");
    put(board, "a", 1, "WR");
    put(board, "h", 1, "WR");
    put(board, "a", 8, "BK");

    const moves = castlingMoves(board, "white");
    assert.strictEqual(moves.length, 2);
    assert.strictEqual(moves[0].side, "kingside");
    assert.strictEqual(moves[1].side, "queenside");

    board.moves.push({ from: { file: "h", rank: 1 }, to: { file: "h", rank: 2 } });
    assert.strictEqual(castlingMoves(board, "white").length, 1);
    assert.strictEqual(castlingMoves(board, "white")[0].side, "queenside");

    board.moves = [{ from: { file: "e", rank: 1 }, to: { file: "e", rank: 2 } }];
    assert.deepStrictEqual(castlingMoves(board, "white"), []);

    board.moves = [];
    put(board, "f", 8, "BR");
    assert.strictEqual(castlingMoves(board, "white").length, 1);
    assert.strictEqual(castlingMoves(board, "white")[0].side, "queenside");

    const allMoves = functions.allLegalMoves(board, "white");
    assert.strictEqual(allMoves.some(move => move.castle === "queenside"), true);
}

function testEnPassant() {
    const board = emptyBoard();
    board.moves = [];
    put(board, "e", 5, "WP");
    put(board, "d", 5, "BP");
    board.moves.push({
        from: { file: "d", rank: 7 },
        to: { file: "d", rank: 5 }
    });

    const moves = enPassantMoves(board, "e", 5, "white");
    assert.strictEqual(moves.length, 1);
    assert.deepStrictEqual(moves[0].to, { file: "d", rank: 6 });
    assert.deepStrictEqual(moves[0].capture, { file: "d", rank: 5 });

    const allMoves = functions.allLegalMoves(board, "white");
    assert.strictEqual(allMoves.some(move =>
        move.enPassant && move.to.file === "d" && move.to.rank === 6
    ), true);

    board.moves.push({
        from: { file: "a", rank: 2 },
        to: { file: "a", rank: 3 }
    });
    assert.deepStrictEqual(enPassantMoves(board, "e", 5, "white"), []);

    const blackBoard = emptyBoard();
    blackBoard.moves = [{
        from: { file: "d", rank: 2 },
        to: { file: "d", rank: 4 }
    }];
    put(blackBoard, "e", 4, "BP");
    put(blackBoard, "d", 4, "WP");
    assert.strictEqual(enPassantMoves(blackBoard, "e", 4, "black").length, 1);
}

testBoardFunctions();
testKing();
testQueen();
testRook();
testBishop();
testKnight();
testPawn();
testAttackAndAllMoves();
testCheckmateAndStalemate();
testDrawRules();
testCastling();
testEnPassant();

console.log("All functions and piece tests passed.");
