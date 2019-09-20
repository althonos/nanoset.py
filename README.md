# `nanoset.py`

*A memory-optimized wrapper for Python sets likely to be empty.*

[![TravisCI](https://img.shields.io/travis/althonos/nanoset.py/master.svg?logo=travis&maxAge=600&style=flat-square)](https://travis-ci.org/althonos/nanoset.py/branches)
[![AppVeyor](https://img.shields.io/appveyor/ci/althonos/nanoset-py/master?logo=appveyor&style=flat-square&maxAge=600)](https://ci.appveyor.com/project/althonos/nanoset-py)
[![License](https://img.shields.io/badge/license-MIT-blue.svg?style=flat-square&maxAge=2678400)](https://choosealicense.com/licenses/mit/)
[![Source](https://img.shields.io/badge/source-GitHub-303030.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/nanoset.py/)
[![PyPI](https://img.shields.io/pypi/v/nanoset.svg?style=flat-square&maxAge=600)](https://pypi.org/project/nanoset)
[![Wheel](https://img.shields.io/pypi/wheel/nanoset.svg?style=flat-square&maxAge=2678400)](https://pypi.org/project/nanoset/#files)
[![Python Versions](https://img.shields.io/pypi/pyversions/nanoset.svg?style=flat-square&maxAge=600)](https://pypi.org/project/nanoset/#files)
[![PyPI - Implementation](https://img.shields.io/pypi/implementation/nanoset.svg?style=flat-square&maxAge=600)](https://pypi.org/project/nanoset/#files)
[![Changelog](https://img.shields.io/badge/keep%20a-changelog-8A0707.svg?maxAge=2678400&style=flat-square)](https://github.com/althonos/nanoset.py/blob/master/CHANGELOG.md)
[![GitHub issues](https://img.shields.io/github/issues/althonos/nanoset.py.svg?style=flat-square&maxAge=600)](https://github.com/althonos/nanoset.py/issues)

## TL;DR: Where you get curious

If you have code with at least **10%** empty sets at runtime, you can reduce
the memory usage of that code with a simple line of code:
```python
from nanoset import PicoSet as set
```
and save up to 85% of the memory used by `set` instances during execution!

## Introduction: Where I tell you about Python memory usage

Python is a great programming language (*fight me*), but sometimes you start
questioning why it does things in certain ways. Since Python 2.3, the standard
library provides the [`set`](https://docs.python.org/3.7/library/stdtypes.html#set)
collection, which is a specialized container for membership testing. On the
contrary to the ubiquitous [`list`](https://docs.python.org/3.7/library/stdtypes.html#list)
collection, `set` is not ordered (or, more accurately, *does not let you access
the order it stores the elements in*). The other feature of `set` is that just
like their mathematical counterpart, they do not allow duplicate, which is very
useful for some algorithms. However, **sets are memory-expensive**:
```python
>>> import sys
>>> sys.getsizeof(list())
72
>>> sys.getsizeof(set())
232
```

An empty set takes more than three times the memory of an empty list! For some
data structures or objects with a lot of `set` attributes, they can quickly
cause a very large part of the memory to be used for nothing. This is even more
sad when you are used to [Rust](https://www.rust-lang.org/), where most
[collections](https://doc.rust-lang.org/std/collections/) allocate lazily.
**This is where `nanoset` comes to the rescue:**
```python
>>> import nanoset
>>> sys.getsizeof(nanoset.NanoSet())
56
>>> sys.getsizeof(nanoset.PicoSet())
24
```

*Actually, that's a lie, but keep reading*.

## Example: Where we build a directed graph

Let's imagine we are building an ordered graph data structure, where we may
want to store [taxonomic data](https://en.wikipedia.org/wiki/Taxonomic_database),
or any other kind of hierarchy. We can simply define the graphs and its nodes
with the two following classes:

```python
class Graph:
    root: Node
    nodes: Dict[str, Node]

class Node:
    neighbors: Set[node]
```

This makes adding an edge and querying for an edge existence between two nodes
an `O(1)` operation, and iterating over all the nodes an `O(n)` operation, which
is mot likely what we want here. We use `set` an dnot `list` because we want to
avoid storing an edge in duplicate, which is a sensible choice. But now let's
look at the [statistics](https://terminologies.gfbio.org/terminology/?ontology=NCBITAXON)
of the [NCBITaxon](https://www.ncbi.nlm.nih.gov/taxonomy) project, the
database for Organismal Classification developed by the US National Center for
Biotechnology Information:

     Metrics
        Number of classes*: 1595237              
        Number of individuals: 0
        Number of properties: 0
        Classes without definition: 1595237
        Classes without label: 0
        Average number of children: 12
        Classes with a single child: 40319
        Maximum number of children: 41761
        Classes with more than 25 children: 0
        Classes with more than 1 parent: 0
        Maximum depth: 38
        Number of leaves**: 1130671

According to these, we are going to have **1,130,671** leavers for a total of
**1,595,237** nodes, which means **70.8%** of empty sets. Now you may think:

> Ok, I got this. But in this case, I just need a special case for leaves, where
> instead of storing an empty set of `neighbors`, I just store `None`, since it
> only has a size of 16 bytes! And then, I can replace `None` with an actual
> set if I want to add a new edge from that node!

Well, glad we are on the same level: this is what **`nanoset`** does for you!


## Implementation: Where the magic happens

Actually, it's not magic at all. Just imagine a class `NanoSet` that works as
a [proxy](https://www.tutorialspoint.com/python_design_patterns/python_design_patterns_proxy.htm) to an actual Python `set` it wraps, but which is only allocated when
some data actually needs to be stored:

```python
class NanoSet(collections.abc.Set):

    def __init__(self, iterable=None):
        self.inner = None if iterable is None else set(iterable)

    def add(self, element):
        if self.inner is None:
            self.inner = set()
        self.inner.add(element)

    # ... the rest of the `set` API ...
```

That's about it! However, doing it like so in Python would not be the extremely
efficient, as the resulting object would be **64** bytes. Using
[slots](http://book.pythontips.com/en/latest/__slots__magic.html), this can be
reduced to 56 bytes, which is what we get with `nanoset.NanoSet`.

**Note that these values are only when the inner set is empty!** When actually
allocating the set to store our values, we allocate an additional **232** bytes of
data. This means that using **`nanoset`** creates an overhead, since a non-empty
set will now weigh **288** bytes.

> Well, I was way better off with my approach of storing `Optional[Set]`
> everywhere then, I don't want to pay any additional cost for nonempty sets!

Sure, this is not great. But that would mean changing your whole code. And
actually, you may not gain that much memory from doing that compared to using
`nanoset`, since the only time the wrapper performs badly is when you have a
load factor close to 100%.

> By the way, you didn't mention `PicoSet`. How did you manage to get that down
> to 24 bytes, when a slotted Python object can't be less that 56 bytes ?

Easy: `PicoSet` is basically `NanoSet`, but without the garbage-collection
protocol implementation. This saves us 32 bytes of object memory, but comes
with a drawback: the garbage collector cannot see the allocated set *inside*
the `PicoSet`. This does not change anything for execution, but debugging with
a memory profiler may be harder. As such, I'd advice to avoid using `PicoSet`
when debugging, which can be done easily with Python's `__debug__` flag:
```python
if __debug__:
    from nanoset import NanoSet as set
else:
    from nanoset import PicoSet as set
```
This will cause `PicoSet` to be used when running Python with the `-O` flag.


## Statistics: Where I convice you to use this

Okay, so let's do some maths. With `S = 232` the size of an allocated set,
`s` the size of the wrapper (`56` for `NanoSet`, `24` for `PicoSet`), the
`x` percentage of nonempty sets in our data structure, the relative size
of our sets is:

  * if we're using `set`: **S \* x / (S \* x) = 100%** (we use that as a reference)
  * if we're using `NanoSet`: **((S + 56) \* x + 56 \* (100 - x)) / (S \* x)**
  * if we're using `PicoSet`: **((S + 56) \* x + 56 \* (100 - x)) / (S * x)**

This gives us the following graph, which shows how much memory you can save
depending of the ratio of empty sets you have at runtime:

![sizegraph](https://github.com/althonos/nanoset.py/raw/master/static/sizegraph.svg)

If we get back to our NCBITaxon example, we have a total of **1,595,237** nodes
and **1,130,671** leaves, which means that by using sets we are allocating
**1,595,237 * 232 = 353.0 MiB** of memory simply for `set` after the whole
taxonomy is loaded. If we use `NanoSet` however, we
can reduce this to **188.0 MiB**, or even to **139.3 MiB** with `PicoSet`!
**We just saved about 50% memory just by using `NanoSet` in place of `set`.**


## Installing: Where you add this to your project dependencies

This module is implemented in Rust, but native Python [wheels](https://pythonwheels.com/)
are compiled for the following platforms:

* Windows x86-64: CPython 3.5, 3.6, 3.7
* Linux x86-64: CPython 3.5, 3.6, 3.7, and PyPy 3.7
* OSX x86-64: CPython 3.6, 3.7, and PyPy 3.7

If you platform is not among these, you will need a
[working Rust `nightly` toolchain](https://www.rust-lang.org/tools/install)
as well as the [`setuptools-rust`](https://pypi.org/project/setuptools-rust/)
library installed to build the extension module.


## Licensing: Where you learn when you can use this

This library is provided under the open-source
[MIT license](https://choosealicense.com/licenses/mit/), which lets you do
a lot of things freely as long as you give proper credit.
