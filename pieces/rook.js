const functions = require("../functions");

function rookPosition(board, file, rank, player) {
    const files = ["a", "b", "c", "d", "e", "f", "g", "h"];
    const directions = [
        [1, 0],
        [-1, 0],
        [0, 1],
        [0, -1]
    ];
    const positions = [];
    const fileIndex = files.indexOf(file);

    if (fileIndex === -1) {
        return positions;
    }

    for (const [fileOffset, rankOffset] of directions) {
        let currentFileIndex = fileIndex + fileOffset;
        let currentRank = Number(rank) + rankOffset;

        while (functions.validSquare(board, files[currentFileIndex], currentRank)) {
            const currentFile = files[currentFileIndex];

            if (functions.selfPiece(board, currentFile, currentRank, player)) {
                break;
            }

            positions.push({ file: currentFile, rank: currentRank });

            if (functions.hasPiece(board, currentFile, currentRank)) {
                break;
            }

            currentFileIndex += fileOffset;
            currentRank += rankOffset;
        }
    }

    return positions;
}

module.exports = rookPosition;
