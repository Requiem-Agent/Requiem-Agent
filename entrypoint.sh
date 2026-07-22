#!/bin/bash
set -e

echo "[entrypoint] Starting Requiem Agent v2 (Sprint 1-2)"

# Start Rust backend
echo "[entrypoint] Starting Rust backend on :3001"
./requiem-server &
BACKEND_PID=$!
sleep 2

# Start ttyd terminal on :7681
echo "[entrypoint] Starting ttyd terminal on :7681"
ttyd --port 7681 --writable --once \
    -t 'theme={"background":"#1a1b26","foreground":"#a9b1d6","cursor":"#c0caf5"}' \
    -t fontSize=14 \
    bash &
TTYD_PID=$!
sleep 1

# Start nginx on :7860
echo "[entrypoint] Starting nginx on :7860"
nginx -g 'daemon off;' &
NGINX_PID=$!

echo "[entrypoint] All services running:"
echo "  Backend: :3001 (pid=$BACKEND_PID)"
echo "  Terminal: :7681 (pid=$TTYD_PID)"
echo "  Nginx: :7860 (pid=$NGINX_PID)"

# Wait for any process to exit
wait -n
EXIT_CODE=$?
echo "[entrypoint] Process exited with code $EXIT_CODE, shutting down..."
kill $BACKEND_PID $TTYD_PID $NGINX_PID 2>/dev/null || true
exit $EXIT_CODE
