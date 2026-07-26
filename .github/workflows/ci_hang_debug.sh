#!/usr/bin/env bash
set -uo pipefail

TIMEOUT_SEC="${TIMEOUT_SEC:-200}"
OUTDIR="${GITHUB_WORKSPACE:-.}/hang_artifacts"
mkdir -p "$OUTDIR"

echo "[ci_hang_debug] running pytest with args: $*"
echo "[ci_hang_debug] hang timeout: ${TIMEOUT_SEC}s"

if ! command -v py-spy >/dev/null 2>&1; then
    echo "[ci_hang_debug] installing py-spy..."
    pip install py-spy --break-system-packages >/dev/null 2>&1 || \
        pip install py-spy >/dev/null 2>&1 || \
        echo "[ci_hang_debug] WARNING: could not install py-spy"
fi

PYTHONFAULTHANDLER=1 python3 -X faulthandler -m pytest "$@" \
    > "$OUTDIR/pytest_output.log" 2>&1 &
PYTEST_PID=$!

SECS=0
HUNG=0
while kill -0 "$PYTEST_PID" 2>/dev/null; do
    if [ "$SECS" -ge "$TIMEOUT_SEC" ]; then
        HUNG=1
        echo "[ci_hang_debug] !!! HANG DETECTED after ${TIMEOUT_SEC}s (pid $PYTEST_PID) !!!"

        echo "[ci_hang_debug] process tree:"
        ps --forest -o pid,ppid,cmd -g $(ps -o sid= -p "$PYTEST_PID") \
            > "$OUTDIR/process_tree.txt" 2>&1 || pstree -p "$PYTEST_PID" > "$OUTDIR/process_tree.txt" 2>&1

        echo "[ci_hang_debug] sending SIGQUIT for faulthandler dump..."
        kill -SIGQUIT "$PYTEST_PID" 2>/dev/null
        sleep 3

        if command -v py-spy >/dev/null 2>&1; then
            echo "[ci_hang_debug] running py-spy dump (with native)..."
            py-spy dump --pid "$PYTEST_PID" --native \
                > "$OUTDIR/pyspy_native.txt" 2>&1
            echo "[ci_hang_debug] running py-spy dump (python-only, fallback)..."
            py-spy dump --pid "$PYTEST_PID" \
                > "$OUTDIR/pyspy_python_only.txt" 2>&1
            echo "[ci_hang_debug] running py-spy dump --locals..."
            py-spy dump --pid "$PYTEST_PID" --locals \
                > "$OUTDIR/pyspy_locals.txt" 2>&1
        fi

        if command -v gdb >/dev/null 2>&1; then
            echo "[ci_hang_debug] running gdb bt for all threads..."
            gdb -p "$PYTEST_PID" -batch \
                -ex "thread apply all bt" \
                -ex "detach" \
                -ex "quit" \
                > "$OUTDIR/gdb_dump.txt" 2>&1
        fi

        sleep 1
        echo "[ci_hang_debug] killing hung process tree..."
        kill -9 "$PYTEST_PID" 2>/dev/null
        pkill -9 -P "$PYTEST_PID" 2>/dev/null

        break
    fi
    sleep 2
    SECS=$((SECS + 2))
done

if [ "$HUNG" -eq 1 ]; then
    echo "[ci_hang_debug] === pytest_output.log (last 200 lines) ==="
    tail -n 200 "$OUTDIR/pytest_output.log" || true
    echo "[ci_hang_debug] Hang artifacts saved to: $OUTDIR"
    echo "[ci_hang_debug] Make sure your workflow uploads this directory as an artifact."
    exit 1
else
    wait "$PYTEST_PID"
    RC=$?
    cat "$OUTDIR/pytest_output.log"
    exit $RC
fi
