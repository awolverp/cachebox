
::: cachebox._core.BaseCacheImpl
    options:
      members:
        - __init__
        - maxsize
        - getsizeof
        - current_size
        - remaining_size
        - capacity
        - __len__
        - __contains__
        - contains
        - is_empty
        - is_full
        - insert
        - __setitem__
        - update
        - get
        - __getitem__
        - setdefault
        - pop
        - __delitem__
        - popitem
        - drain
        - shrink_to_fit
        - clear
        - __eq__
        - __ne__
        - items
        - values
        - keys
        - __iter__
        - copy
        - __repr__

::: cachebox._core.Cache
    options:
      members:
        - insert
        - update
        - get
        - setdefault
        - pop
        - popitem
        - items
        - values
        - keys

::: cachebox._core.FIFOCache
    options:
      members:
        - insert
        - update
        - get
        - setdefault
        - pop
        - popitem
        - items
        - values
        - keys
        - first
        - last

::: cachebox._core.RRCache
    options:
      members:
        - insert
        - update
        - get
        - setdefault
        - pop
        - popitem
        - items
        - values
        - keys

::: cachebox._core.LRUCache
    options:
      members:
        - insert
        - update
        - get
        - setdefault
        - pop
        - popitem
        - items
        - values
        - keys
        - peek
        - least_recently_used
        - most_recently_used

::: cachebox._core.LFUCache
    options:
      members:
        - insert
        - update
        - get
        - setdefault
        - pop
        - popitem
        - items
        - values
        - keys
        - items_with_frequency
        - peek
        - least_frequently_used

::: cachebox._cachebox.TTLCache
    options:
      members:
        - __init__
        - sweep_interval
        - stop_sweeper
        - global_ttl
        - insert
        - update
        - get
        - setdefault
        - pop
        - popitem
        - items
        - values
        - keys
        - first
        - last
        - expire
        - get_with_expire
        - pop_with_expire
        - popitem_with_expire
        - items_with_expire

::: cachebox._cachebox.VTTLCache
    options:
      members:
        - __init__
        - sweep_interval
        - stop_sweeper
        - insert
        - update
        - setdefault
        - popitem
        - items
        - values
        - keys
        - first
        - last
        - expire
        - get_with_expire
        - pop_with_expire
        - popitem_with_expire
        - items_with_expire
