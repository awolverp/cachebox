import timeit

import cachebox

# --- Setup ---
MAXSIZE = 100_000
N = 1_000  # operations per benchmark
REPEAT = 100
NUMBER = 100


def make_cache(n: int = N) -> cachebox.Cache:
    """Create a pre-populated cache for benchmarks that need existing keys."""
    c = cachebox.Cache(maxsize=MAXSIZE, capacity=n)
    for i in range(n):
        c.insert(i, f"value_{i}")
    return c


# --- Benchmark definitions ---


def bench_insert():
    c = cachebox.Cache(maxsize=MAXSIZE, capacity=N)
    for i in range(N):
        c.insert(i, f"value_{i}")


def bench_get():
    c = make_cache()
    for i in range(N):
        c.get(i)


def bench_update():
    c = make_cache()
    for i in range(N):
        c.insert(i, f"new_value_{i}")  # insert on existing key = update


def bench_delete():
    c = make_cache()
    for i in range(N):
        del c[i]


# --- Runner ---

benchmarks = {
    "insert": bench_insert,
    "get": bench_get,
    "update": bench_update,
    "delete": bench_delete,
}

print(f"Benchmark: {N} ops each, best of {REPEAT}x{NUMBER} runs\n")
print(f"{'Operation':<10} {'Best (ms)':>10} {'Per-op (µs)':>12}")
print("-" * 35)

for name, fn in benchmarks.items():
    times = timeit.repeat(fn, repeat=REPEAT, number=NUMBER)
    best_ms = min(times) / NUMBER * 1000  # best total run in ms
    per_op_us = min(times) / NUMBER / N * 1_000_000  # per single op in µs
    print(f"{name:<10} {best_ms:>10.3f} {per_op_us:>12.4f}")
