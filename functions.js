function hasPiece(board, file, rank) {
    const square = board[rank]?.[file];

    if (square === undefined) {
        return false;
    }

    return square !== "  ";
}


function selfPiece(board, file, rank, player) {
    const square = board[rank]?.[file];

    if (square === undefined) {
        return false;
    }

    if (player === "white") {
        return square.startsWith("W");
    }

    if (player === "black") {
        return square.startsWith("B");
    }

    return false;
}


function validSquare(board, file, rank) {
    return board[rank]?.[file] !== undefined;
}


function pieceMoveFunctions() {
    // Require lazily so the piece files can continue requiring this module.
    return {
        K: require("./pieces/king"),
        Q: require("./pieces/queen"),
        R: require("./pieces/rook"),
        B: require("./pieces/bishop"),
        N: require("./pieces/knight"),
        P: require("./pieces/pawn")
    };
}


function isAttacked(board, file, rank, player) {
    if ((player !== "white" && player !== "black") ||
        !validSquare(board, file, rank)) {
        return false;
    }

    const opponent = player === "white" ? "black" : "white";
    const opponentPrefix = opponent === "white" ? "W" : "B";
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const targetRank = Number(rank);
    const targetFileIndex = files.indexOf(file);

    for (const sourceRank of Object.keys(board)) {
        for (const sourceFile of files) {
            const piece = board[sourceRank]?.[sourceFile];

            if (!piece || !piece.startsWith(opponentPrefix)) {
                continue;
            }

            // A pawn attacks diagonally even when the target square is empty.
            if (piece[1] === "P") {
                const direction = opponent === "white" ? 1 : -1;
                const sourceFileIndex = files.indexOf(sourceFile);

                if (targetRank === Number(sourceRank) + direction &&
                    Math.abs(targetFileIndex - sourceFileIndex) === 1) {
                    return true;
                }

                continue;
            }

            const calculateMoves = pieceMoveFunctions()[piece[1]];

            if (calculateMoves && calculateMoves(board, sourceFile, sourceRank, opponent)
                .some(move => move.file === file && move.rank === targetRank)) {
                return true;
            }
        }
    }

    return false;
}


function copyBoard(board) {
    return Object.fromEntries(
        Object.entries(board).map(([rank, row]) => [rank, { ...row }])
    );
}


function kingPosition(board, player) {
    const king = player === "white" ? "WK" : "BK";
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];

    for (const rank of Object.keys(board)) {
        for (const file of files) {
            if (board[rank]?.[file] === king) {
                return { file, rank };
            }
        }
    }

    return null;
}


function moveLeavesKingAttacked(board, move, piece, player) {
    const nextBoard = copyBoard(board);
    const promotedPiece = move.promotion
        ? `${player === "white" ? "W" : "B"}${move.promotion}`
        : piece;

    nextBoard[move.from.rank][move.from.file] = "  ";

    if (move.enPassant && move.capture) {
        nextBoard[String(move.capture.rank)][move.capture.file] = "  ";
    }

    nextBoard[String(move.to.rank)][move.to.file] = promotedPiece;

    if (move.castle && move.rook) {
        const rook = nextBoard[String(move.rook.from.rank)][move.rook.from.file];
        nextBoard[String(move.rook.from.rank)][move.rook.from.file] = "  ";
        nextBoard[String(move.rook.to.rank)][move.rook.to.file] = rook;
    }

    const king = kingPosition(nextBoard, player);

    // Keep partial-board testing possible when no king is present.
    if (!king) {
        return false;
    }

    return isAttacked(nextBoard, king.file, king.rank, player);
}


function allLegalMoves(board, player) {
    if (player !== "white" && player !== "black") {
        return [];
    }

    const pieceMoves = pieceMoveFunctions();
    const enPassantMoves = require("./en_passant");
    const castlingMoves = require("./castling");
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const possibleMoves = [];
    const promotionPieces = ["Q", "R", "B", "N"];

    for (const rank of Object.keys(board)) {
        for (const file of files) {
            const piece = board[rank]?.[file];

            if (!piece || !piece.startsWith(player === "white" ? "W" : "B")) {
                continue;
            }

            const calculateMoves = pieceMoves[piece[1]];

            if (!calculateMoves) {
                continue;
            }

            for (const to of calculateMoves(board, file, rank, player)) {
                const target = board[String(to.rank)]?.[to.file];
                const opponentKing = player === "white" ? "BK" : "WK";

                // Kings are never captured; checkmate ends the game first.
                if (target === opponentKing) {
                    continue;
                }

                const move = {
                    from: { file, rank },
                    to
                };

                if (piece[1] === "P" && pieceMoves.P.isPromotionMove(move, player)) {
                    for (const promotion of promotionPieces) {
                        possibleMoves.push({ ...move, promotion });
                    }
                    continue;
                }

                possibleMoves.push(move);
            }

            if (piece[1] === "P") {
                possibleMoves.push(...enPassantMoves(board, file, rank, player));
            }
        }
    }

    for (const castle of castlingMoves(board, player)) {
        possibleMoves.push({
            from: castle.king.from,
            to: castle.king.to,
            castle: castle.side,
            rook: castle.rook
        });
    }

    return possibleMoves.filter(move => {
        const piece = board[move.from.rank]?.[move.from.file];
        return !moveLeavesKingAttacked(board, move, piece, player);
    });
}


function isInCheck(board, player) {
    const king = kingPosition(board, player);

    return Boolean(king) && isAttacked(board, king.file, king.rank, player);
}


function isCheckmate(board, player) {
    if (player !== "white" && player !== "black") {
        return false;
    }

    return isInCheck(board, player) && allLegalMoves(board, player).length === 0;
}


function isStalemate(board, player) {
    if (player !== "white" && player !== "black") {
        return false;
    }

    return Boolean(kingPosition(board, player)) &&
        !isInCheck(board, player) &&
        allLegalMoves(board, player).length === 0;
}


function isThreefoldRepetition(positionHistory) {
    if (!Array.isArray(positionHistory)) {
        return false;
    }

    const counts = new Map();

    for (const position of positionHistory) {
        const key = typeof position === "string"
            ? position
            : JSON.stringify(position);
        counts.set(key, (counts.get(key) || 0) + 1);

        if (counts.get(key) >= 3) {
            return true;
        }
    }

    return false;
}


function isFiftyMoveDraw(halfmoveClock) {
    return Number.isInteger(halfmoveClock) && halfmoveClock >= 100;
}


function isInsufficientMaterial(board) {
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const pieces = [];

    for (const rank of Object.keys(board)) {
        if (rank === "moves") {
            continue;
        }

        for (const file of ["a", "b", "c", "d", "e", "f", "g", "h"]) {
            const piece = board[rank]?.[file];
            if (piece && piece !== "  ") {
                pieces.push({ piece, file, rank: Number(rank) });
            }
        }
    }

    const nonKings = pieces.filter(item => item.piece[1] !== "K");

    // King versus king.
    if (nonKings.length === 0) {
        return true;
    }

    // Any pawn, rook, or queen means there is mating material.
    if (nonKings.some(item => ["P", "R", "Q"].includes(item.piece[1]))) {
        return false;
    }

    // King and one bishop or knight versus king.
    if (nonKings.length === 1 && ["B", "N"].includes(nonKings[0].piece[1])) {
        return true;
    }

    // King and bishop versus king and bishop is insufficient only when both
    // bishops occupy squares of the same colour.
    if (nonKings.length === 2 && nonKings.every(item => item.piece[1] === "B")) {
        const squareColours = nonKings.map(item =>
            (files.indexOf(item.file) + item.rank) % 2
        );
        return squareColours[0] === squareColours[1];
    }

    return false;
}


function isGameOver(board, player) {
    return isCheckmate(board, player) || isStalemate(board, player);
}


module.exports = {
    hasPiece,
    selfPiece,
    validSquare,
    isAttacked,
    allLegalMoves,
    isInCheck,
    isCheckmate,
    isStalemate,
    isThreefoldRepetition,
    isFiftyMoveDraw,
    isInsufficientMaterial,
    isGameOver
};
