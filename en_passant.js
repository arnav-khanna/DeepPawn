const functions = require("./functions");

const files = ["a", "b", "c", "d", "e", "f", "g", "h"];

function enPassantMoves(board, file, rank, player) {
    if (player !== "white" && player !== "black") {
        return [];
    }

    const sourceRank = Number(rank);
    const direction = player === "white" ? 1 : -1;
    const opponent = player === "white" ? "B" : "W";
    const opponentPawn = `${opponent}P`;
    const playerPawn = `${player === "white" ? "W" : "B"}P`;
    const opponentStartingRank = player === "white" ? 7 : 2;
    const targetRank = sourceRank + direction;
    const fileIndex = files.indexOf(file);
    const lastMove = Array.isArray(board.moves)
        ? board.moves[board.moves.length - 1]
        : null;

    if (fileIndex === -1 || !Number.isInteger(sourceRank) || !lastMove ||
        board[String(sourceRank)]?.[file] !== playerPawn) {
        return [];
    }

    // En passant is only possible immediately after the opponent's move.
    const movedTwoSquares = lastMove.from && lastMove.to &&
        lastMove.from.file === lastMove.to.file &&
        Number(lastMove.from.rank) === opponentStartingRank &&
        Math.abs(Number(lastMove.to.rank) - Number(lastMove.from.rank)) === 2;

    if (!movedTwoSquares ||
        Number(lastMove.to.rank) !== sourceRank ||
        board[String(lastMove.to.rank)]?.[lastMove.to.file] !== opponentPawn) {
        return [];
    }

    const moves = [];

    for (const fileOffset of [-1, 1]) {
        const capturedFile = files[fileIndex + fileOffset];

        if (capturedFile !== lastMove.to.file ||
            board[String(sourceRank)]?.[capturedFile] !== opponentPawn ||
            !functions.validSquare(board, capturedFile, targetRank) ||
            functions.hasPiece(board, capturedFile, targetRank)) {
            continue;
        }

        moves.push({
            from: { file, rank },
            to: { file: capturedFile, rank: targetRank },
            enPassant: true,
            capture: { file: capturedFile, rank: sourceRank }
        });
    }

    return moves;
}

module.exports = enPassantMoves;
