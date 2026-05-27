# `doc-example/libs/mathlib/src/mathlib.h`

**Language:** C

[← Index](index.md) | [All symbols](symbols.md)

---

## Contents

- [fn `clamp`](#fn-clamp-line-31)
- [fn `gcd`](#fn-gcd-line-41)
- [fn `vec2_length`](#fn-vec2-length-line-68)
- [fn `vec2_add`](#fn-vec2-add-line-73)
- [fn `lerp`](#fn-lerp-line-48)

---

<a id="fn-clamp-line-31"></a>
## fn `clamp`

<sub>line 31</sub>

```
fn clamp(double v, double lo, double hi) -> double
```

Clamp a value to [lo, hi].

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>v</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Input value.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>lo</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Lower bound (inclusive).</td></tr>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>hi</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Upper bound (inclusive).</td></tr>
  </tbody>
</table>

**Returns:** v clamped to [lo, hi].

---

<a id="fn-gcd-line-41"></a>
## fn `gcd`

<sub>line 41</sub>

```
fn gcd(unsigned int a, unsigned int b) -> unsigned int
```

Greatest common divisor of two non-negative integers.

Uses the iterative Euclidean algorithm.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>a</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">First operand.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>b</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Second operand.</td></tr>
  </tbody>
</table>

**Returns:** GCD(a, b).

---

<a id="fn-vec2-length-line-68"></a>
## fn `vec2_length`

<sub>line 68</sub>

```
fn vec2_length(Vec2 v) -> double
```

Compute the Euclidean length of v.

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

<a id="fn-vec2-add-line-73"></a>
## fn `vec2_add`

<sub>line 73</sub>

```
fn vec2_add(Vec2 a, Vec2 b) -> Vec2
```

Add two vectors component-wise.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>a</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Left operand.</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>b</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Right operand.</td></tr>
  </tbody>
</table>

**Returns:** a + b

---

<a id="fn-lerp-line-48"></a>
## fn `lerp`

<sub>line 48</sub>

```
fn lerp(double a, double b, double t) -> double
```

Linear interpolation between two values.

<table style="border-collapse: collapse; margin: 0.75rem 0 1rem; font-size: 0.92em;">
  <thead>
    <tr style="background: #eaf4ff; color: #0b3d68;">
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Parameter</th>
      <th style="border: 1px solid #b6d7f2; padding: 0.25rem 0.5rem; text-align: left;">Description</th>
    </tr>
  </thead>
  <tbody>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>a</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Start value (t == 0).</td></tr>
    <tr style="background: #f7fbff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>b</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">End value (t == 1).</td></tr>
    <tr style="background: #ffffff;"><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem; white-space: nowrap;"><code>t</code></td><td style="border: 1px solid #d0e3f4; padding: 0.2rem 0.5rem;">Interpolation parameter in [0, 1].</td></tr>
  </tbody>
</table>

**Returns:** a + t * (b - a)

---

