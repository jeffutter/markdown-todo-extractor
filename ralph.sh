# ralph.sh
# Usage: ./ralph.sh <iterations>

set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <iterations>"
  exit 1
fi

# For each iteration, run Claude Code with the following prompt.
# This prompt is basic, we'll expand it later.
for ((i=1; i<=$1; i++)); do
  result=$(claude -p "$(cat <<'EOF'
Choose one task from beads and execute it to completion.
1. Check if there are any tasks you were already in the middle of: bd ready -a claude
2. If not, choose a ready task: bd ready
  - This should be the one YOU decide has the highest priority, not necessarily the first in the list.
3. If there are no tasks remaining output: '<promise>COMPLETE</promise>' and exit
4. Once you have a task, do the follow:
  4.1. Claim the task: bd update -a claude <task_id>
  4.2. View the task
  4.3. Execute the work in the task description
  4.4. If you discover new distinct work while working on this ticket or any work that could be split off, create a new ticket and add any blockers to the ticket or update any blockers that the sub-ticket blocks
  4.5. Ensure the code compiles: cargo build
  4.6. Ensure all tests pass: cargo test
  4.7. Ensure the code is formatted: cargo fmt
  4.8. If you did anything differently than as requested in the ticket, add a comment to the ticket
  4.9. Mark the ticket complete
  4.10. Commit your changes

If, while implementing the feature, you notice that all work
is complete, output <promise>COMPLETE</promise>.
EOF
)")

  echo "========================================"
  echo "$result"
  echo "========================================"

  if [[ "$result" == *"<promise>COMPLETE</promise>"* ]]; then
    echo "PRD complete, exiting."
    exit 0
  fi
done

