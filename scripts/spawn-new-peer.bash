#!/usr/bin/env bash

# This script spawns a new peer node (node 3) using malachitebft-eth-app
# Home directory: nodes/3

function help {
    echo "Usage: spawn-new-peer.bash [--help] [--no-reset]"
    echo "  --help      Show this help message"
    echo "  --no-reset  Don't reset the database (keep existing data)"
}

# Parse arguments
NO_RESET=0
while [[ "$#" -gt 0 ]]; do
    case $1 in
        --help) help; exit 0 ;;
        --no-reset) NO_RESET=1 ;;
        *) echo "Unknown parameter passed: $1"; help; exit 1 ;;
    esac
    shift
done

# Fixed configuration
NODE_ID=3
NODES_HOME="nodes"
APP_BINARY="malachitebft-eth-app"
NODE_HOME="$NODES_HOME/$NODE_ID"

echo "Starting new peer node $NODE_ID..."

# Compile the application
echo "Compiling '$APP_BINARY'..."
cargo build -p $APP_BINARY

if [ $? -ne 0 ]; then
    echo "Failed to compile $APP_BINARY"
    exit 1
fi

export RUST_BACKTRACE=full

# Create node directories and reset if needed
if [[ $NO_RESET -eq 0 ]]; then
    echo "[Node $NODE_ID] Resetting the database..."
    rm -rf "$NODE_HOME/db"
    mkdir -p "$NODE_HOME/db"
    rm -rf "$NODE_HOME/wal"
    mkdir -p "$NODE_HOME/wal"
else
    echo "[Node $NODE_ID] Preserving existing database..."
    mkdir -p "$NODE_HOME/db"
    mkdir -p "$NODE_HOME/wal"
fi

# Clean and create logs and traces directories
rm -rf "$NODE_HOME/logs"
mkdir -p "$NODE_HOME/logs"

rm -rf "$NODE_HOME/traces"
mkdir -p "$NODE_HOME/traces"

# Function to handle cleanup on interrupt
function exit_and_cleanup {
    if [ -f "$NODE_HOME/node.pid" ]; then
        NODE_PID=$(cat "$NODE_HOME/node.pid")
        echo "Stopping node $NODE_ID (PID: $NODE_PID)..."
        kill "$NODE_PID" 2>/dev/null
        rm -f "$NODE_HOME/node.pid"
    fi
    exit 0
}

# Trap the INT signal (Ctrl+C) to run the cleanup function
trap exit_and_cleanup INT

echo "[Node $NODE_ID] Spawning node..."
cargo run --bin $APP_BINARY -q -- start --home "$NODE_HOME" > "$NODE_HOME/logs/node.log" 2>&1 &
NODE_PID=$!
echo $NODE_PID > "$NODE_HOME/node.pid"

echo "[Node $NODE_ID] Started with PID: $NODE_PID"
echo "[Node $NODE_ID] Logs are available at: $NODE_HOME/logs/node.log"
echo "[Node $NODE_ID] Home directory: $NODE_HOME"
echo ""
echo "Press Ctrl+C to stop the node."

# Keep the script running and monitor the node
while true; do
    if ! kill -0 $NODE_PID 2>/dev/null; then
        echo "[Node $NODE_ID] Node process has stopped unexpectedly!"
        rm -f "$NODE_HOME/node.pid"
        exit 1
    fi
    sleep 1
done
