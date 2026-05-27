# `libs/mathlib/src/numerics.h`

**Language:** C

[← Index](index.md) | [All symbols](symbols.md)

---

## Contents

- [fn `bisect`](#fn-bisect-line-18)
- [fn `integrate`](#fn-integrate-line-37)
- [fn `lu_decompose`](#fn-lu-decompose-line-58)
- [fn `lu_solve`](#fn-lu-solve-line-79)

---

<a id="fn-bisect-line-18"></a>
## fn `bisect`

<sub>line 18</sub>

```
fn bisect(double (*f)(double), double a, double b, double tol) -> double
```

Solve $f(x) = 0$ in $[a, b]$ via bisection.  Requires $f(a) \cdot f(b) < 0$ (sign change guarantees a root by the **Intermediate Value Theorem**).  Convergence rate: the interval width halves each iteration, so after $n$ steps the error satisfies:  $$|e_n| \leq \frac{b - a}{2^{n+1}}$$

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>f</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Continuous function $f : \mathbb{R} \to \mathbb{R}$.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>a</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Left endpoint.</td></tr>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>b</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Right endpoint ($b &gt; a$, $f(a) \cdot f(b) &lt; 0$).</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>tol</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Absolute tolerance; stops when $|b - a| &lt; \text{tol}$.</td></tr>
  </tbody>
</table>

**Returns:** Approximate root $x^* \approx f^{-1}(0)$.

---

<a id="fn-integrate-line-37"></a>
## fn `integrate`

<sub>line 37</sub>

```
fn integrate(double (*f)(double), double a, double b, double eps) -> double
```

Adaptive Simpson's rule for $\int_a^b f(x)\,dx$.  Uses recursive subdivision until the local error estimate satisfies $|\Delta| < \varepsilon / (b - a)$.  The composite rule on a single panel $[c, d]$ is:  $$S(c,d) = \frac{d-c}{6}\left[f(c) + 4f\!\left(\frac{c+d}{2}\right) + f(d)\right]$$  | Parameter | Meaning | |-----------|---------| | `f`   | Integrand (must be smooth on $[a,b]$) | | `a`   | Lower limit | | `b`   | Upper limit | | `eps` | Absolute error tolerance $\varepsilon > 0$ |

**Returns:** Approximation of $\int_a^b f(x)\,dx$ with error $< \varepsilon$.

**See also:** bisect

---

<a id="fn-lu-decompose-line-58"></a>
## fn `lu_decompose`

<sub>line 58</sub>

```
fn lu_decompose(double *A, int *piv, int n) -> int
```

LU decomposition of an $n \times n$ matrix $A$.  Factors $A = P L U$ where: - $P$ is a permutation matrix (partial pivoting) - $L$ is unit lower-triangular: $L_{ii} = 1$, $L_{ij} = 0$ for $j > i$ - $U$ is upper-triangular  The factorisation is stored **in-place** in `A`; the strictly lower part holds $L$ (without the diagonal ones) and the upper part holds $U$.  Complexity: $\mathcal{O}(n^3)$ flops.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>A</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Input matrix ($n \times n$), overwritten with $L$ and $U$.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>piv</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Output pivot index array of length $n$.</td></tr>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>n</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Matrix dimension.</td></tr>
  </tbody>
</table>

**Returns:** 0 on success, -1 if $A$ is singular.

**Warning:** `A` must be stored in **row-major** order.

---

<a id="fn-lu-solve-line-79"></a>
## fn `lu_solve`

<sub>line 79</sub>

```
fn lu_solve(const double *LU, const int *piv, double *b, int n)
```

Solve $Ax = b$ using a pre-computed LU factorisation.  Given the output of `lu_decompose`, solves $PLUx = b$ in two steps:  1. Forward substitution: $Ly = Pb$ — $\mathcal{O}(n^2)$ 2. Back substitution:    $Ux = y$  — $\mathcal{O}(n^2)$

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>LU</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Combined $L/U$ factors from `lu_decompose`.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>piv</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Pivot array from `lu_decompose`.</td></tr>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>b</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Right-hand side vector (length $n$), overwritten with $x$.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>n</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">System dimension.</td></tr>
  </tbody>
</table>

---

