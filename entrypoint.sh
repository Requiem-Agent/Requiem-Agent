#!/bin/bash
echo "[entrypoint] Requiem Agent v2 — Starting services..."

# Start Rust backend (uses PORT from env, default 7860 in code)
export PORT=3001
echo "[entrypoint] Starting backend on :${PORT}..."
./requiem-server > /tmp/backend.log 2>&1 &
BACKEND_PID=$!

# Health check loop: wait for backend with timeout
echo "[entrypoint] Waiting for backend to become ready..."
BACKEND_READY=false
for i in $(seq 1 15); do
    if kill -0 $BACKEND_PID 2>/dev/null; then
        if curl -sf http://127.0.0.1:${PORT}/api/healthz > /dev/null 2>&1; then
            BACKEND_READY=true
            echo "[entrypoint] Backend ready after ${i}s"
            break
        fi
    else
        echo "[entrypoint] Backend process died! Logs:"
        cat /tmp/backend.log
        exit 1
    fi
    sleep 1
done

if [ "$BACKEND_READY" != "true" ]; then
    echo "[entrypoint] Backend failed to become ready within 15s! Logs:"
    cat /tmp/backend.log
    exit 1
fi

# Start ttyd on port 7681
echo "[entrypoint] Starting terminal on :7681..."
ttyd --port 7681 --writable \
    -t fontSize=14 \
    bash > /tmp/ttyd.log 2>&1 &
TTYD_PID=$!
sleep 1
echo "[entrypoint] Terminal OK (pid=$TTYD_PID)"

# Start nginx
echo "[entrypoint] Starting nginx..."
nginx -g 'daemon off;' > /tmp/nginx.log 2>&1 &
NGINX_PID=$!
sleep 1
echo "[entrypoint] Nginx OK (pid=$NGINX_PID)"
echo "[entrypoint] All services running on :7860"

# Wait for backend (main service)
wait $BACKEND_PID
EXIT=$?
echo "[entrypoint] Backend exited with code $EXIT"
kill $TTYD_PID $NGINX_PID 2>/dev/null
exit $EXIT
