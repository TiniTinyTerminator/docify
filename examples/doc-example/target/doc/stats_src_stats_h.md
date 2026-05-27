# `stats/src/stats.h`

**Language:** C++

[← Index](index.md) | [All symbols](symbols.md)

---

<a id="item-anonymous-line-1"></a>
## item *(anonymous)*

<sub>line 1</sub>

```
#pragma once
```

Descriptive statistics — C++17 public API.  All symbols live in the `stats` namespace.  Functions operate on `std::vector<double>` samples and throw `std::invalid_argument` for degenerate inputs (empty vectors, length mismatches, or samples too small for the requested statistic).

**file:** stats.h

---

