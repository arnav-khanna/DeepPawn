const functions = require("../functions");

function queenPosition(board, file, rank, player) {
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];

    const directions = [
        [1, 0],   // right
        [-1, 0],  // left
        [0, 1],   // up
        [0, -1],  // down
        [1, 1],   // up-right
        [1, -1],  // down-right
        [-1, 1],  // up-left
        [-1, -1]  // down-left
    ];

    const positions = [];

    const fileIndex = files.indexOf(file);

    if (fileIndex === -1) {
        return positions;
    }

    for (const [fileOffset, rankOffset] of directions) {
        let currentFileIndex = fileIndex + fileOffset;
        let currentRank = Number(rank) + rankOffset;

        while (
            functions.validSquare(
                board,
                files[currentFileIndex],
                currentRank
            )
        ) {
            const currentFile = files[currentFileIndex];

            // Own piece blocks the queen
            if (functions.selfPiece(board, currentFile, currentRank, player)) {
                break;
            }

            // Add the square
            positions.push({
                file: currentFile,
                rank: currentRank
            });

            // If there is any piece, stop after that square
            if (functions.hasPiece(board, currentFile, currentRank)) {
                break;
            }

            currentFileIndex += fileOffset;
            currentRank += rankOffset;
        }
    }

    return positions;
}

module.exports = queenPosition;