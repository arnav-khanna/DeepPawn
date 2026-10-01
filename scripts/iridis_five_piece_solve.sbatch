#!/usr/bin/env bash
# Solve one legal 5x5 K+K+three-piece position on an Iridis AMD node.
#
# Submit, for example:
#   sbatch scripts/iridis_five_piece_solve.sbatch '4k/5/2K2/1QRN1/5' white
# Optional third argument overrides the worker count (default: all allocated CPUs).
# The FEN is checked by `solve5` before any long-running search begins.

#SBATCH --job-name=deeppawn-5pc
#SBATCH --account=ecsstudents
#SBATCH --partition=amd_student
#SBATCH --nodes=1
#SBATCH --ntasks=1
#SBATCH --cpus-per-task=32
#SBATCH --mem=32G
#SBATCH --time=24:00:00
#SBATCH --output=/iridisfs/home/ak32g25/deeppawn/logs/%x-%j.out
#SBATCH --error=/iridisfs/home/ak32g25/deeppawn/logs/%x-%j.err

set -euo pipefail

if [[ $# -lt 1 || $# -gt 3 ]]; then
    echo "Usage: sbatch $0 '<five-rank FEN>' [white|black] [workers]" >&2
    exit 2
fi

position_fen=$1
side_to_move=${2:-white}
worker_count=${3:-${SLURM_CPUS_PER_TASK}}

cd /iridisfs/home/ak32g25/deeppawn
mkdir -p logs
source "$HOME/.cargo/env"

echo "Source revision: $(git rev-parse --short HEAD 2>/dev/null || echo untracked)"
echo "Position: $position_fen; side: $side_to_move; workers: $worker_count"
RUSTFLAGS="${RUSTFLAGS:-} -C target-cpu=native" cargo build --release --locked

result_file="logs/${SLURM_JOB_NAME}-${SLURM_JOB_ID}.result"
printf 'solve5 %s %s %s\nquit\n' "$position_fen" "$side_to_move" "$worker_count" |
    /usr/bin/time -f 'wall_seconds=%e max_rss_kib=%M' ./target/release/deep-pawn |
    tee "$result_file"
echo "Result file: /iridisfs/home/ak32g25/deeppawn/$result_file" | tee -a "$result_file"
