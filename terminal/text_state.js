const functions = require("../functions");

function boardText(board) {
    const lines = ["board:"];

    for (let rank = 8; rank >= 1; rank--) {
        const row = [];
        for (const file of "abcdefgh") {
            row.push(board[String(rank)]?.[file] || "  ");
        }
        lines.push(`${rank}: ${row.join(" ")}`);
    }

    return lines.join("\n");
}

function stateText(board, turn, halfmoveClock, positionHistory, sanMoves) {
    const legalMoves = functions.allLegalMoves(board, turn);
    const moveText = legalMoves.map(move => {
        const suffix = move.promotion ? `=${move.promotion}` : "";
        return `${move.from.file}${move.from.rank}-${move.to.file}${move.to.rank}${suffix}`;
    });

    return [
        `turn: ${turn}`,
        `in_check: ${functions.isInCheck(board, turn)}`,
        `checkmate: ${functions.isCheckmate(board, turn)}`,
        `stalemate: ${functions.isStalemate(board, turn)}`,
        `insufficient_material: ${functions.isInsufficientMaterial(board)}`,
        `fifty_move_draw: ${functions.isFiftyMoveDraw(halfmoveClock)}`,
        `threefold_repetition: ${functions.isThreefoldRepetition(positionHistory)}`,
        `halfmove_clock: ${halfmoveClock}`,
        `move_count: ${board.moves.length}`,
        `legal_moves: ${moveText.join(",")}`,
        `pgn_moves: ${sanMoves.join(" ")}`,
        boardText(board)
    ].join("\n");
}

module.exports = { boardText, stateText };
