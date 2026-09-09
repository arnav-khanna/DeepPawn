const functions = require("../functions");

function canMoveTwoSquares(board, file, rank, player) {
    if (player !== "white" && player !== "black") {
        return false;
    }

    const currentRank = Number(rank);
    const direction = player === "white" ? 1 : -1;
    const startingRank = player === "white" ? 2 : 7;
    const pawn = player === "white" ? "WP" : "BP";

    if (board[String(currentRank)]?.[file] !== pawn ||
        currentRank !== startingRank) {
        return false;
    }

    const oneStepRank = currentRank + direction;
    const twoStepRank = currentRank + (2 * direction);

    return functions.validSquare(board, file, oneStepRank) &&
        functions.validSquare(board, file, twoStepRank) &&
        !functions.hasPiece(board, file, oneStepRank) &&
        !functions.hasPiece(board, file, twoStepRank);
}

function pawnPosition(board, file, rank, player) {
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const positions = [];
    const fileIndex = files.indexOf(file);
    const currentRank = Number(rank);
    const direction = player === "white" ? 1 : player === "black" ? -1 : 0;

    if (fileIndex === -1 || direction === 0 || !Number.isInteger(currentRank)) {
        return positions;
    }

    const nextRank = currentRank + direction;

    // Pawns move forward only into empty squares.
    if (functions.validSquare(board, file, nextRank) &&
        !functions.hasPiece(board, file, nextRank)) {
        positions.push({ file, rank: nextRank });

        const doubleStepRank = currentRank + (2 * direction);
        if (canMoveTwoSquares(board, file, currentRank, player)) {
            positions.push({ file, rank: doubleStepRank });
        }
    }

    // Pawns capture one square diagonally, never straight ahead.
    for (const fileOffset of [-1, 1]) {
        const captureFile = files[fileIndex + fileOffset];

        if (!functions.validSquare(board, captureFile, nextRank) ||
            !functions.hasPiece(board, captureFile, nextRank) ||
            functions.selfPiece(board, captureFile, nextRank, player)) {
            continue;
        }

        positions.push({ file: captureFile, rank: nextRank });
    }

    return positions;
}

function isPromotionMove(move, player) {
    if (!move?.to || (player !== "white" && player !== "black")) {
        return false;
    }

    const promotionRank = player === "white" ? 8 : 1;
    return Number(move.to.rank) === promotionRank;
}

module.exports = pawnPosition;
module.exports.canMoveTwoSquares = canMoveTwoSquares;
module.exports.isPromotionMove = isPromotionMove;
