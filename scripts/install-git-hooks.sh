#!/bin/bash
# Install git hooks for ROUP development

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
HOOKS_DIR="$REPO_ROOT/.git/hooks"

echo "Installing ROUP git hooks..."

# Install pre-push hook
if [ -f "$SCRIPT_DIR/hooks/pre-push" ]; then
    cp "$SCRIPT_DIR/hooks/pre-push" "$HOOKS_DIR/pre-push"
    chmod +x "$HOOKS_DIR/pre-push"
    echo "✓ Installed pre-push hook (runs test.sh before push)"
else
    echo "✗ Warning: pre-push hook not found at $SCRIPT_DIR/hooks/pre-push"
    exit 1
fi

echo ""
echo "Git hooks installed successfully!"
echo ""
echo "The pre-push hook will automatically run ./test.sh before each push."
echo "This ensures all tests pass before code is pushed to the remote repository."
