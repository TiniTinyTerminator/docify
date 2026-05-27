# Symbols

[Files](index.md) | **Symbols**

*29 named documented items.*

## Classes

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [class `stats::OrderStatistics`](stats_src_stats_h.md#class-stats-orderstatistics-line-54) | [stats/src/stats.h](stats_src_stats_h.md) | 54 | Immutable sorted view into a sample for order-statistic queries. |

## Functions

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [fn `bisect`](mathlib_src_numerics_h.md#fn-bisect-line-18) | [mathlib/src/numerics.h](mathlib_src_numerics_h.md) | 18 | Solve $f(x) = 0$ in $[a, b]$ via bisection.  Requires $f(a) \cdot f(b) < 0$ (sign change guarantees a root by the **Intermediate Value Theorem**).  Convergence rate: the interval width halves each iteration, so after $n$ steps the error satisfies:  $$\|e_n\| \leq \frac{b - a}{2^{n+1}}$$ |
| [fn `clamp`](mathlib_src_mathlib_h.md#fn-clamp-line-31) | [mathlib/src/mathlib.h](mathlib_src_mathlib_h.md) | 31 | Clamp a value to [lo, hi]. |
| [fn `dot`](linalg_src_linalg_f90.md#fn-dot-line-23) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 23 | Compute the dot product of two real vectors. |
| [fn `factorial`](mathlib_src_mathlib_c.md#fn-factorial-line-9) | [mathlib/src/mathlib.c](mathlib_src_mathlib_c.md) | 9 | Compute the factorial of n.  Iterative implementation to avoid stack overflow for large n. |
| [fn `gcd`](mathlib_src_mathlib_h.md#fn-gcd-line-41) | [mathlib/src/mathlib.h](mathlib_src_mathlib_h.md) | 41 | Greatest common divisor of two non-negative integers. |
| [fn `integrate`](mathlib_src_numerics_h.md#fn-integrate-line-37) | [mathlib/src/numerics.h](mathlib_src_numerics_h.md) | 37 | Adaptive Simpson's rule for $\int_a^b f(x)\,dx$.  Uses recursive subdivision until the local error estimate satisfies $\|\Delta\| < \varepsilon / (b - a)$.  The composite rule on a single panel $[c, d]$ is:  $$S(c,d) = \frac{d-c}{6}\left[f(c) + 4f\!\left(\frac{c+d}{2}\right) + f(d)\right]$$  \| Parameter \| Meaning \| \|-----------\|---------\| \| `f`   \| Integrand (must be smooth on $[a,b]$) \| \| `a`   \| Lower limit \| \| `b`   \| Upper limit \| \| `eps` \| Absolute error tolerance $\varepsilon > 0$ \| |
| [fn `lerp`](mathlib_src_mathlib_h.md#fn-lerp-line-48) | [mathlib/src/mathlib.h](mathlib_src_mathlib_h.md) | 48 | Linear interpolation between two values. |
| [fn `lu_decompose`](mathlib_src_numerics_h.md#fn-lu-decompose-line-58) | [mathlib/src/numerics.h](mathlib_src_numerics_h.md) | 58 | LU decomposition of an $n \times n$ matrix $A$.  Factors $A = P L U$ where: - $P$ is a permutation matrix (partial pivoting) - $L$ is unit lower-triangular: $L_{ii} = 1$, $L_{ij} = 0$ for $j > i$ - $U$ is upper-triangular  The factorisation is stored **in-place** in `A`; the strictly lower part holds $L$ (without the diagonal ones) and the upper part holds $U$.  Complexity: $\mathcal{O}(n^3)$ flops. |
| [fn `lu_solve`](mathlib_src_numerics_h.md#fn-lu-solve-line-79) | [mathlib/src/numerics.h](mathlib_src_numerics_h.md) | 79 | Solve $Ax = b$ using a pre-computed LU factorisation.  Given the output of `lu_decompose`, solves $PLUx = b$ in two steps:  1. Forward substitution: $Ly = Pb$ — $\mathcal{O}(n^2)$ 2. Back substitution:    $Ux = y$  — $\mathcal{O}(n^2)$ |
| [fn `norm2`](linalg_src_linalg_f90.md#fn-norm2-line-48) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 48 | Euclidean norm of a vector. |
| [fn `solve2`](linalg_src_linalg_f90.md#fn-solve2-line-57) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 57 | Solve a 2×2 linear system Ax = b via Cramer's rule. |
| [fn `stats::OrderStatistics::OrderStatistics`](stats_src_stats_h.md#fn-stats-orderstatistics-orderstatistics-line-59) | [stats/src/stats.h](stats_src_stats_h.md) | 59 | Construct from a sample (makes a sorted copy). |
| [fn `stats::OrderStatistics::median`](stats_src_stats_h.md#fn-stats-orderstatistics-median-line-65) | [stats/src/stats.h](stats_src_stats_h.md) | 65 | Median of the sample. |
| [fn `stats::OrderStatistics::percentile`](stats_src_stats_h.md#fn-stats-orderstatistics-percentile-line-68) | [stats/src/stats.h](stats_src_stats_h.md) | 68 | Percentile via linear interpolation. |
| [fn `stats::algo::covariance`](stats_src_stats_h.md#fn-stats-algo-covariance-line-97) | [stats/src/stats.h](stats_src_stats_h.md) | 97 | Covariance of two equal-length samples.  $\text{Cov}(X,Y) = \frac{1}{n-1}\sum (x_i - \bar{x})(y_i - \bar{y})$ |
| [fn `stats::algo::linreg`](stats_src_stats_h.md#fn-stats-algo-linreg-line-84) | [stats/src/stats.h](stats_src_stats_h.md) | 84 | Ordinary least-squares linear regression.  Fits the model $y = a x + b$ by minimising $\sum(y_i - a x_i - b)^2$. |
| [fn `stats::mean`](stats_src_stats_h.md#fn-stats-mean-line-17) | [stats/src/stats.h](stats_src_stats_h.md) | 17 | Arithmetic mean of a sample. |
| [fn `stats::pearson`](stats_src_stats_h.md#fn-stats-pearson-line-42) | [stats/src/stats.h](stats_src_stats_h.md) | 42 | Pearson correlation coefficient between two equal-length samples.  Returns a value in $[-1, 1]$.  Returns NaN when either sample has zero variance. |
| [fn `stats::stddev`](stats_src_stats_h.md#fn-stats-stddev-line-37) | [stats/src/stats.h](stats_src_stats_h.md) | 37 | Standard deviation of a sample. |
| [fn `stats::variance`](stats_src_stats_h.md#fn-stats-variance-line-26) | [stats/src/stats.h](stats_src_stats_h.md) | 26 | Sample variance (unbiased, Bessel-corrected).  Uses $s^2 = \frac{1}{n-1}\sum(x_i - \bar{x})^2$. |
| [fn `vec2_add`](mathlib_src_mathlib_h.md#fn-vec2-add-line-73) | [mathlib/src/mathlib.h](mathlib_src_mathlib_h.md) | 73 | Add two vectors component-wise. |
| [fn `vec2_length`](mathlib_src_mathlib_h.md#fn-vec2-length-line-68) | [mathlib/src/mathlib.h](mathlib_src_mathlib_h.md) | 68 | Compute the Euclidean length of v. |

## Modules

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [mod `linalg`](linalg_src_linalg_f90.md#mod-linalg-line-1) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 1 | Basic linear algebra routines — Fortran with FORD-style comments. |

## Subroutines

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [sub `scale`](linalg_src_linalg_f90.md#sub-scale-line-39) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 39 | Scale a vector by a scalar factor in-place. |

## Variables

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [var `default_lda`](linalg_src_linalg_f90.md#var-default-lda-line-13) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 13 | Default leading dimension used for allocatable matrix layouts. |
| [var `dp`](linalg_src_linalg_f90.md#var-dp-line-7) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 7 | Double-precision alias for clarity. |
| [var `pi`](linalg_src_linalg_f90.md#var-pi-line-10) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 10 | Mathematical constant pi (double precision). |
| [var `singular_tol`](linalg_src_linalg_f90.md#var-singular-tol-line-18) | [linalg/src/linalg.f90](linalg_src_linalg_f90.md) | 18 | Absolute tolerance used by `solve2` to detect singular matrices. |

