const functions = require("./functions");

function moveStartedAt(move, file, rank) {
    return move &&
        move.from &&
        move.from.file === file &&
        Number(move.from.rank) === Number(rank);
}

function hasMovedFrom(board, file, rank) {
    if (!Array.isArray(board.moves)) {
        return false;
    }

    return board.moves.some(move => moveStartedAt(move, file, rank));
}

function emptySquares(board, rank, squares) {
    return squares.every(file =>
        functions.validSquare(board, file, rank) &&
        !functions.hasPiece(board, file, rank)
    );
}

function safeKingSquares(board, player, rank, squares) {
    const king = player === "white" ? "WK" : "BK";
    const kingFile = "e";

    return squares.every(file => {
        const position = Object.fromEntries(
            Object.entries(board).map(([boardRank, row]) => [boardRank, { ...row }])
        );

        // Check each square with the king actually occupying it. This prevents
        // the king's starting position from blocking an attack on a later square.
        position[String(rank)][kingFile] = "  ";
        position[String(rank)][file] = king;

        return !functions.isAttacked(position, file, rank, player);
    });
}

function castlingMoves(board, player) {
    if (player !== "white" && player !== "black") {
        return [];
    }

    const rank = player === "white" ? 1 : 8;
    const prefix = player === "white" ? "W" : "B";
    const moves = [];

    // The king must still be on e1/e8 and must never have moved.
    if (board[rank]?.e !== `${prefix}K` || hasMovedFrom(board, "e", rank)) {
        return moves;
    }

    const castles = [
        {
            side: "kingside",
            rookFile: "h",
            empty: ["f", "g"],
            safe: ["e", "f", "g"],
            kingTo: "g",
            rookTo: "f"
        },
        {
            side: "queenside",
            rookFile: "a",
            empty: ["b", "c", "d"],
            safe: ["e", "d", "c"],
            kingTo: "c",
            rookTo: "d"
        }
    ];

    for (const castle of castles) {
        if (board[rank]?.[castle.rookFile] !== `${prefix}R` ||
            hasMovedFrom(board, castle.rookFile, rank) ||
            !emptySquares(board, rank, castle.empty) ||
            !safeKingSquares(board, player, rank, castle.safe)) {
            continue;
        }

        moves.push({
            side: castle.side,
            king: {
                from: { file: "e", rank },
                to: { file: castle.kingTo, rank }
            },
            rook: {
                from: { file: castle.rookFile, rank },
                to: { file: castle.rookTo, rank }
            }
        });
    }

    return moves;
}

module.exports = castlingMoves;
