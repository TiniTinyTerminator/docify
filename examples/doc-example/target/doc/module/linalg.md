# module `linalg`

[← Index](../index.md) | [All symbols](../symbols.md)

---

Basic linear algebra routines — Fortran with FORD-style comments.

## Procedures

| Name | Summary |
|------|---------|
| [`dot`](#fn-dot) | Compute the dot product of two real vectors. |
| [`scale`](#sub-scale) | Scale a vector by a scalar factor in-place. |
| [`norm2`](#fn-norm2) | Euclidean norm of a vector. |
| [`solve2`](#fn-solve2) | Solve a 2×2 linear system Ax = b via Cramer's rule. |

## Module Variables

| Name | Summary |
|------|---------|
| [`dp`](#var-dp) | Double-precision alias for clarity. |
| [`pi`](#var-pi) | Mathematical constant pi (double precision). |
| [`default_lda`](#var-default-lda) | Default leading dimension used for allocatable matrix layouts. |
| [`singular_tol`](#var-singular-tol) | Absolute tolerance used by `solve2` to detect singular matrices. |

---

## Detailed Documentation

<a id="fn-dot"></a>
## fn `dot`

<sub>line 23</sub>

```
fn dot(u, v)
```

Compute the dot product of two real vectors.

Iterates over the common length; shorter vector is treated as zero-padded.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>u</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">First vector.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>v</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Second vector.</td></tr>
  </tbody>
</table>

**Returns:** u · v

---

<a id="sub-scale"></a>
## sub `scale`

<sub>line 39</sub>

```
fn scale(v, alpha)
```

Scale a vector by a scalar factor in-place.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>v</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Vector to scale (modified).</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>alpha</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Scale factor.</td></tr>
  </tbody>
</table>

---

<a id="fn-norm2"></a>
## fn `norm2`

<sub>line 48</sub>

```
fn norm2(v)
```

Euclidean norm of a vector.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>v</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Input vector.</td></tr>
  </tbody>
</table>

**Returns:** ||v||₂

---

<a id="fn-solve2"></a>
## fn `solve2`

<sub>line 57</sub>

```
fn solve2(A, b, x) -> logical
```

Solve a 2×2 linear system Ax = b via Cramer's rule.

Returns .false. if A is singular (det ≈ 0).

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>A</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">2×2 coefficient matrix.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>b</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Right-hand side vector (length 2).</td></tr>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>x</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Solution vector (output, length 2).</td></tr>
  </tbody>
</table>

**Returns:** .true. on success, .false. if singular.

---

<a id="var-dp"></a>
## var `dp`

<sub>line 7</sub>

```
integer, parameter :: dp = kind(1.0d0)
```

Double-precision alias for clarity.

---

<a id="var-pi"></a>
## var `pi`

<sub>line 10</sub>

```
real(dp), parameter :: pi = 3.141592653589793_dp
```

Mathematical constant pi (double precision).

---

<a id="var-default-lda"></a>
## var `default_lda`

<sub>line 13</sub>

```
integer, parameter :: default_lda = 64
```

Default leading dimension used for allocatable matrix layouts.

Increase this when working with very wide matrices to avoid
cache-line aliasing on AVX-512 systems.

---

<a id="var-singular-tol"></a>
## var `singular_tol`

<sub>line 18</sub>

```
real(dp), parameter :: singular_tol = 1.0e-14_dp
```

Absolute tolerance used by `solve2` to detect singular matrices.

---

