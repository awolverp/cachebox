import tracemalloc
import time


class Bench:
    number = 10
    classes = []

    def __init_subclass__(cls) -> None:
        Bench.classes.append(cls)

    def _benchmark_func(self, func, setUp):
        best_memory = 0
        best_speed = 0
        
        print(f"  ({self.__class__.__name__}) {func.__name__} :: {func.__doc__.strip()}")
        for _ in range(self.number):
            cache = setUp()

            tracemalloc.start()
            perf = time.perf_counter()
            func(cache)
            perf = time.perf_counter() - perf
            _, peak = tracemalloc.get_traced_memory()
            tracemalloc.stop()

            if best_speed == 0 or perf < best_speed:
                best_speed = perf
                best_memory = peak
        
        best_speed *= 1000
        print(f"\t\t\t| {self.number} loops, best: {best_speed:.3f}ms / {best_memory/10**3}KB")

    def start(self):
        functions = [
            (getattr(self, i), getattr(self, i.replace("bench_", "")+"_setUp")) for i in dir(self) if i.startswith("bench_")
        ]

        print(f"*Benchmarking {self.__class__.__name__}:")

        for func in functions:
            self._benchmark_func(*func)
