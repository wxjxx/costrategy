#!/bin/sh
set -eu

if [ "${RUN_MIGRATIONS:-false}" = "true" ]; then
    /app/migrate
fi

/app/costrategy-backend &
backend_pid="$!"

term() {
    kill -TERM "$backend_pid" 2>/dev/null || true
    nginx -s quit 2>/dev/null || true
}

trap term INT TERM

(
    wait "$backend_pid"
    nginx -s quit 2>/dev/null || true
) &

nginx -g "daemon off;"
status="$?"

kill -TERM "$backend_pid" 2>/dev/null || true
wait "$backend_pid" 2>/dev/null || true

exit "$status"
