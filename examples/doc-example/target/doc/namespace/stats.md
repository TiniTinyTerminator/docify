# namespace `stats`

[← Index](../index.md) | [All symbols](../symbols.md)

---

## Types

| Name | Summary |
|------|---------|
| [class `OrderStatistics`](../class/stats__orderstatistics.md) | Immutable sorted view into a sample for order-statistic queries. |

## Functions

| Signature | Summary |
|-----------|---------|
| [`double mean(const std::vector<double>& xs)`](#fn-mean) | Arithmetic mean of a sample. |
| [`double variance(const std::vector<double>& xs)`](#fn-variance) | Sample variance (unbiased, Bessel-corrected).  Uses $s^2 = \frac{1}{n-1}\sum(x_i - \bar{x})^2$. |
| [`double stddev(const std::vector<double>& xs)`](#fn-stddev) | Standard deviation of a sample. |
| [`double pearson(const std::vector<double>& xs, const std::vector<double>& ys)`](#fn-pearson) | Pearson correlation coefficient between two equal-length samples.  Returns a value in $[-1, 1]$.  Returns NaN when either sample has zero variance. |

---

## Detailed Documentation

<a id="fn-mean"></a>
## fn `mean`

<sub>line 17</sub>

```
double mean(const std::vector<double>& xs)
```

Arithmetic mean of a sample.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>xs</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Non-empty input vector.</td></tr>
  </tbody>
</table>

**Returns:** Sample mean $\bar{x} = \frac{1}{n}\sum x_i$.

**throws std::invalid_argument:** if xs is empty.

---

<a id="fn-variance"></a>
## fn `variance`

<sub>line 26</sub>

```
double variance(const std::vector<double>& xs)
```

Sample variance (unbiased, Bessel-corrected).  Uses $s^2 = \frac{1}{n-1}\sum(x_i - \bar{x})^2$.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>xs</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Input sample (≥ 2 elements required).</td></tr>
  </tbody>
</table>

**Returns:** $s^2$

**See also:** [stats::mean](stats.md#fn-mean)

---

<a id="fn-stddev"></a>
## fn `stddev`

<sub>line 37</sub>

```
double stddev(const std::vector<double>& xs)
```

Standard deviation of a sample.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>xs</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Input sample (≥ 2 elements required).</td></tr>
  </tbody>
</table>

**Returns:** $\sqrt{s^2}$

---

<a id="fn-pearson"></a>
## fn `pearson`

<sub>line 42</sub>

```
double pearson(const std::vector<double>& xs, const std::vector<double>& ys)
```

Pearson correlation coefficient between two equal-length samples.  Returns a value in $[-1, 1]$.  Returns NaN when either sample has zero variance.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>xs</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">First sample.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>ys</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Second sample (must be the same length as xs).</td></tr>
  </tbody>
</table>

**Returns:** $r \in [-1, 1]$.

---

