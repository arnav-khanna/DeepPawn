const functions = require("../functions");

function knightPosition(board, file, rank, player) {
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];

    const moves = [
        [1, 2],
        [1, -2],
        [-1, 2],
        [-1, -2],
        [2, 1],
        [2, -1],
        [-2, 1],
        [-2, -1]
    ];

    const positions = [];

    const fileIndex = files.indexOf(file);

    // Invalid starting file
    if (fileIndex === -1) {
        return positions;
    }

    for (const [fileOffset, rankOffset] of moves) {
        const newFileIndex = fileIndex + fileOffset;
        const newRank = Number(rank) + rankOffset;
        const newFile = files[newFileIndex];

        // Check if destination exists on board
        if (!functions.validSquare(board, newFile, newRank)) {
            continue;
        }

        // Cannot move onto your own piece
        if (functions.selfPiece(board, newFile, newRank, player)) {
            continue;
        }

        positions.push({
            file: newFile,
            rank: newRank
        });
    }

    return positions;
}

module.exports = knightPosition;
