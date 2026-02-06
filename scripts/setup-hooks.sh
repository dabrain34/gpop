#!/bin/sh
#
# Install git hooks for development
#

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
HOOKS_DIR="$PROJECT_ROOT/.git/hooks"

# Only install if we're in a git repository
if [ ! -d "$PROJECT_ROOT/.git" ]; then
    echo "Not a git repository, skipping hooks installation"
    exit 0
fi

# Create hooks directory if it doesn't exist
mkdir -p "$HOOKS_DIR"

# Install pre-commit hook
cat > "$HOOKS_DIR/pre-commit" << 'EOF'
#!/bin/sh
#
# Pre-commit hook to check Rust code formatting
#

# Check if cargo is available
if ! command -v cargo >/dev/null 2>&1; then
    echo "Warning: cargo not found, skipping format check"
    exit 0
fi

echo "Checking Rust formatting..."
if ! cargo fmt --all -- --check; then
    echo ""
    echo "ERROR: Rust code is not formatted correctly."
    echo "Run 'cargo fmt --all' to fix formatting issues."
    exit 1
fi

echo "Formatting check passed."
EOF

chmod +x "$HOOKS_DIR/pre-commit"
echo "Git hooks installed successfully"
