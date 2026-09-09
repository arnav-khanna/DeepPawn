const functions = require("../functions");

function kingPosition(board, file, rank, player) {
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const moves = [
        [0, 1],
        [0, -1],
        [1, 0],
        [-1, 0],
        [1, 1],
        [1, -1],
        [-1, 1],
        [-1, -1]
    ];
    const positions = [];
    const fileIndex = files.indexOf(file);
    if (fileIndex === -1) {
        return positions;
    }

    for (const [fileOffset, rankOffset] of moves) {
        const newFileIndex = fileIndex + fileOffset;
        const newRank = Number(rank) + rankOffset;
        const newFile = files[newFileIndex];

        // Make sure the square exists
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

module.exports = kingPosition;
