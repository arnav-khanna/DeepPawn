const functions = require("../functions");

const letters = { K: "K", Q: "Q", R: "R", B: "B", N: "N" };

function square(square) {
    return `${square.file}${square.rank}`;
}

function moveToSan(board, move, player, legalMoves) {
    const piece = board[String(move.from.rank)][move.from.file];
    const isPawn = piece[1] === "P";
    const target = board[String(move.to.rank)]?.[move.to.file];
    const capture = Boolean(move.enPassant) || target !== "  ";

    if (move.castle === "kingside") return "O-O";
    if (move.castle === "queenside") return "O-O-O";

    let notation = isPawn ? "" : letters[piece[1]];

    if (!isPawn) {
        const ambiguous = legalMoves.filter(candidate =>
            candidate.from.file !== move.from.file ||
            Number(candidate.from.rank) !== Number(move.from.rank)
        ).filter(candidate => {
            const candidatePiece = board[String(candidate.from.rank)]?.[candidate.from.file];
            return candidatePiece === piece &&
                candidate.to.file === move.to.file &&
                Number(candidate.to.rank) === Number(move.to.rank);
        });

        if (ambiguous.length) {
            const sameFile = ambiguous.some(candidate => candidate.from.file === move.from.file);
            notation += sameFile ? move.from.rank : move.from.file;
        }
    } else if (capture) {
        notation += move.from.file;
    }

    if (capture) notation += "x";
    notation += square(move.to);
    if (move.promotion) notation += `=${move.promotion}`;
    return notation;
}

function checkSuffix(board, playerToMove) {
    if (functions.isCheckmate(board, playerToMove)) return "#";
    if (functions.isInCheck(board, playerToMove)) return "+";
    return "";
}

function exportPgn(sanMoves, result = "*") {
    const moveText = [];

    for (let index = 0; index < sanMoves.length; index += 2) {
        const moveNumber = Math.floor(index / 2) + 1;
        const white = sanMoves[index];
        const black = sanMoves[index + 1];
        moveText.push(`${moveNumber}. ${white}${black ? ` ${black}` : ""}`);
    }

    return `${moveText.join(" ")}${moveText.length ? " " : ""}${result}`.trim();
}

module.exports = { moveToSan, checkSuffix, exportPgn };
