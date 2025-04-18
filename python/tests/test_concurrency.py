from cachebox import cached, LRUCache
from concurrent import futures
import asyncio
import pytest
import time


def test_threading_return():
    calls = 0

    @cached(LRUCache(0))
    def func():
        nonlocal calls
        time.sleep(1)
        calls += 1
        return "Hello"

    with futures.ThreadPoolExecutor(max_workers=10) as executor:
        future_list = [executor.submit(func) for _ in range(10)]
        for future in futures.as_completed(future_list):
            assert future.result() == "Hello"

    assert calls == 1


def test_threading_exc():
    calls = 0

    @cached(LRUCache(0))
    def func():
        nonlocal calls
        time.sleep(1)
        calls += 1
        raise RuntimeError

    with futures.ThreadPoolExecutor(max_workers=5) as executor:
        future_list = [executor.submit(func) for _ in range(5)]
        for future in futures.as_completed(future_list):
            assert isinstance(future.exception(), RuntimeError)

    assert calls == 1

    with futures.ThreadPoolExecutor(max_workers=5) as executor:
        future_list = [executor.submit(func) for _ in range(5)]
        for future in futures.as_completed(future_list):
            assert isinstance(future.exception(), RuntimeError)

    assert calls == 2


@pytest.mark.asyncio
async def test_asyncio_return():
    calls = 0

    @cached(LRUCache(0))
    async def func():
        nonlocal calls
        await asyncio.sleep(1)
        calls += 1
        return "Hello"

    await asyncio.gather(
        func(),
        func(),
        func(),
        func(),
        func(),
    )

    assert calls == 1


@pytest.mark.asyncio
async def test_asyncio_exc():
    calls = 0

    @cached(LRUCache(0))
    async def func():
        nonlocal calls
        await asyncio.sleep(1)
        calls += 1
        raise RuntimeError

    tasks = await asyncio.gather(
        func(),
        func(),
        func(),
        func(),
        func(),
        return_exceptions=True,
    )
    for future in tasks:
        assert isinstance(future, RuntimeError)

    assert calls == 1

    tasks = await asyncio.gather(
        func(),
        func(),
        func(),
        func(),
        func(),
        return_exceptions=True,
    )
    for future in tasks:
        assert isinstance(future, RuntimeError)

    assert calls == 2
