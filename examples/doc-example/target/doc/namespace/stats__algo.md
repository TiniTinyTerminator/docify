# namespace `stats::algo`

[← Index](../index.md) | [All symbols](../symbols.md)

---

## Functions

| Signature | Summary |
|-----------|---------|
| [`LinRegResult linreg(const std::vector<double>& xs, const std::vector<double>& ys)`](#fn-linreg) | Ordinary least-squares linear regression.  Fits the model $y = a x + b$ by minimising $\sum(y_i - a x_i - b)^2$. |
| [`double covariance(const std::vector<double>& xs, const std::vector<double>& ys)`](#fn-covariance) | Covariance of two equal-length samples.  $\text{Cov}(X,Y) = \frac{1}{n-1}\sum (x_i - \bar{x})(y_i - \bar{y})$ |

---

## Detailed Documentation

<a id="fn-linreg"></a>
## fn `linreg`

<sub>line 84</sub>

```
LinRegResult linreg(const std::vector<double>& xs, const std::vector<double>& ys)
```

Ordinary least-squares linear regression.  Fits the model $y = a x + b$ by minimising $\sum(y_i - a x_i - b)^2$.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>xs</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Predictor values.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>ys</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Response values (same length as xs).</td></tr>
  </tbody>
</table>

**Returns:** `{slope, intercept}` as a `LinRegResult`.

**See also:** [stats::pearson](stats.md#fn-pearson)

**Warning:** Requires at least two distinct x-values; otherwise slope is NaN.

---

<a id="fn-covariance"></a>
## fn `covariance`

<sub>line 97</sub>

```
double covariance(const std::vector<double>& xs, const std::vector<double>& ys)
```

Covariance of two equal-length samples.  $\text{Cov}(X,Y) = \frac{1}{n-1}\sum (x_i - \bar{x})(y_i - \bar{y})$

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

**Returns:** Sample covariance.

---

