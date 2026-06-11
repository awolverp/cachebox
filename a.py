# from cachebox import Test
# import sys

# sys.setrecursionlimit(8)

# cache = Test()

# def should_raise_recursive_error(key):
#     try:
#         cache[key]
#     except KeyError:
#         pass

#     return should_raise_recursive_error(key)

# should_raise_recursive_error("same-key")


from cachebox import TTLCache, cached


def key(a, b, c, exception: bool):
    return f"{a},{b},{c}"


@cached(TTLCache(1024, 1), key_maker=key)
def calc(a, b, c, exception: bool):
    if exception:
        raise Exception("first call")

    return a + b + c


try:
    calc(1, 2, 3, exception=True)
except Exception:
    print("raised")


calc(1, 2, 3, exception=False)
