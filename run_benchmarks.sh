#!/bin/bash
# Harper Performance Benchmark Script
# Runs Criterion benchmarks across all worktrees and saves results to /tmp
# 
# Based on the benchmarking approach used in PR #726 (perf-fixes branch)
# which used Criterion.rs for performance validation

set -euo pipefail

# Worktrees to benchmark - these are the standard Harper development worktrees
WORKTREES=(
    "~/gallery/harper_master"
    "~/gallery/harper_prod"
    "~/gallery/harper_german"
    "~/gallery/harper"
)

# Output file with timestamp
OUTPUT_FILE="/tmp/harper_benchmarks_$(date +%Y%m%d_%H%M%S).log"

# Create output file with header
{
    echo "================================================================================"
    echo "                    Harper Performance Benchmark Results"
    echo "================================================================================"
    echo "Generated:     $(date)"
    echo "Machine:      $(uname -a)"
    echo "Script:       $0"
    echo "Output file:  $OUTPUT_FILE"
    echo "================================================================================"
    echo ""
} | tee "$OUTPUT_FILE"

# Track overall start time
OVERALL_START=$(date +%s)

# Function to run benchmarks for a single worktree
run_worktree() {
    local worktree_path="$1"
    local worktree_name="$2"
    
    echo "================================================================================" | tee -a "$OUTPUT_FILE"
    echo "Worktree: $worktree_name" | tee -a "$OUTPUT_FILE"
    echo "Path:     $worktree_path" | tee -a "$OUTPUT_FILE"
    echo "--------------------------------------------------------------------------------" | tee -a "$OUTPUT_FILE"
    
    # Get git information
    cd "$worktree_path"
    local branch=$(git branch --show-current 2>/dev/null || echo "unknown")
    local commit=$(git log --oneline -1 2>/dev/null || echo "unknown")
    local date=$(git log -1 --format=%cd --date=iso 2>/dev/null || echo "unknown")
    
    echo "Branch:    $branch" | tee -a "$OUTPUT_FILE"
    echo "Commit:    $commit" | tee -a "$OUTPUT_FILE"
    echo "Date:      $date" | tee -a "$OUTPUT_FILE"
    echo "" | tee -a "$OUTPUT_FILE"
    
    cd "$worktree_path/harper-core"
    
    # Track worktree start time
    local worktree_start=$(date +%s)
    
    # Build release first (benchmarks need release profile)
    echo "[1/3] Building release profile..." | tee -a "$OUTPUT_FILE"
    cargo build --release 2>&1 | tee -a "$OUTPUT_FILE"
    echo "" | tee -a "$OUTPUT_FILE"
    
    # Run parse_essay benchmark
    echo "[2/3] Running parse_essay benchmark..." | tee -a "$OUTPUT_FILE"
    echo "---" | tee -a "$OUTPUT_FILE"
    cargo bench --bench parse_essay 2>&1 | tee -a "$OUTPUT_FILE"
    echo "" | tee -a "$OUTPUT_FILE"
    
    # Run spellcheck benchmark
    echo "[3/3] Running spellcheck benchmark..." | tee -a "$OUTPUT_FILE"
    echo "---" | tee -a "$OUTPUT_FILE"
    cargo bench --bench spellcheck 2>&1 | tee -a "$OUTPUT_FILE"
    echo "" | tee -a "$OUTPUT_FILE"
    
    # Calculate worktree duration
    local worktree_end=$(date +%s)
    local worktree_duration=$((worktree_end - worktree_start))
    local worktree_min=$((worktree_duration / 60))
    local worktree_sec=$((worktree_duration % 60))
    
    echo "--------------------------------------------------------------------------------" | tee -a "$OUTPUT_FILE"
    echo "Worktree completed in: ${worktree_min}m ${worktree_sec}s" | tee -a "$OUTPUT_FILE"
    echo "" | tee -a "$OUTPUT_FILE"
}

# Main loop - run benchmarks for each worktree
for worktree_path in "${WORKTREES[@]}"; do
    # Expand tilde
    worktree_path=$(eval echo "$worktree_path")
    
    # Extract name from path
    worktree_name=$(basename "$worktree_path")
    
    case "$worktree_name" in
        "harper_master") worktree_name="master" ;;
        "harper_prod") worktree_name="harper_prod (feature/merged-german-diagnostics)" ;;
        "harper_german") worktree_name="harper_german (feature/german-language-support)" ;;
        "harper") worktree_name="harper (feature/diagnostics-in-normal-mode)" ;;
    esac
    
    run_worktree "$worktree_path" "$worktree_name"
done

# Calculate overall duration
OVERALL_END=$(date +%s)
OVERALL_DURATION=$((OVERALL_END - OVERALL_START))
OVERALL_MIN=$((OVERALL_DURATION / 60))
OVERALL_SEC=$((OVERALL_DURATION % 60))

# Write footer
{
    echo "================================================================================"
    echo "                            Benchmark Summary"
    echo "================================================================================"
    echo "Total execution time: ${OVERALL_MIN}m ${OVERALL_SEC}s"
    echo "Results saved to:     $OUTPUT_FILE"
    echo ""
    echo "Benchmark data includes:"
    echo "  - parse_essay: Markdown parsing performance"
    echo "  - spellcheck: Fuzzy matching and spelling suggestions"
    echo ""
    echo "For comparison, use Criterion's change detection or manually compare"
    echo "the time values between worktree sections."
    echo ""
    echo "Note: First run in a worktree establishes baseline. Subsequent runs show"
    echo "      percentage changes relative to that baseline."
    echo "================================================================================"
} | tee -a "$OUTPUT_FILE"

echo ""
echo "Benchmark complete. Results saved to: $OUTPUT_FILE"
