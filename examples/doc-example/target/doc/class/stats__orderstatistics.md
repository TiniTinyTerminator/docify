# class `OrderStatistics`

[← Index](../index.md) | [All symbols](../symbols.md)
**Namespace:** [`stats`](../namespace/stats.md)

---

Immutable sorted view into a sample for order-statistic queries.

## Public Member Functions

| Signature | Summary |
|-----------|---------|
| [`OrderStatistics(std::vector<double> xs)`](#fn-orderstatistics) | Construct from a sample (makes a sorted copy). |
| [`double median() const`](#fn-median) | Median of the sample. |
| [`double percentile(double p) const`](#fn-percentile) | Percentile via linear interpolation. |

---

## Detailed Documentation

<a id="fn-orderstatistics"></a>
## fn `OrderStatistics`

<sub>line 59</sub>

```
OrderStatistics(std::vector<double> xs)
```

Construct from a sample (makes a sorted copy).

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>xs</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Input sample.</td></tr>
  </tbody>
</table>

---

<a id="fn-median"></a>
## fn `median`

<sub>line 65</sub>

```
double median() const
```

Median of the sample.

---

<a id="fn-percentile"></a>
## fn `percentile`

<sub>line 68</sub>

```
double percentile(double p) const
```

Percentile via linear interpolation.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>p</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Percentile in [0, 100].</td></tr>
  </tbody>
</table>

**Returns:** Interpolated value.

---

