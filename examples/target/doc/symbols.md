# Symbols

[Files](index.md) | **Symbols**

*29 named documented items.*

## Classes

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [class `stats::OrderStatistics`](doc-example_libs_stats_src_stats_h.md#class-stats-orderstatistics-line-54) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 54 | Immutable sorted view into a sample for order-statistic queries. |

## Functions

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [fn `bisect`](doc-example_libs_mathlib_src_numerics_h.md#fn-bisect-line-18) | [doc-example/libs/mathlib/src/numerics.h](doc-example_libs_mathlib_src_numerics_h.md) | 18 | Solve $f(x) = 0$ in $[a, b]$ via bisection.  Requires $f(a) \cdot f(b) < 0$ (sign change guarantees a root by the **Intermediate Value Theorem**).  Convergence rate: the interval width halves each iteration, so after $n$ steps the error satisfies:  $$\|e_n\| \leq \frac{b - a}{2^{n+1}}$$ |
| [fn `clamp`](doc-example_libs_mathlib_src_mathlib_h.md#fn-clamp-line-31) | [doc-example/libs/mathlib/src/mathlib.h](doc-example_libs_mathlib_src_mathlib_h.md) | 31 | Clamp a value to [lo, hi]. |
| [fn `dot`](doc-example_fortran_linalg_f90.md#fn-dot-line-23) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 23 | Compute the dot product of two real vectors. |
| [fn `factorial`](doc-example_libs_mathlib_src_mathlib_c.md#fn-factorial-line-9) | [doc-example/libs/mathlib/src/mathlib.c](doc-example_libs_mathlib_src_mathlib_c.md) | 9 | Compute the factorial of n.  Iterative implementation to avoid stack overflow for large n. |
| [fn `gcd`](doc-example_libs_mathlib_src_mathlib_h.md#fn-gcd-line-41) | [doc-example/libs/mathlib/src/mathlib.h](doc-example_libs_mathlib_src_mathlib_h.md) | 41 | Greatest common divisor of two non-negative integers. |
| [fn `integrate`](doc-example_libs_mathlib_src_numerics_h.md#fn-integrate-line-37) | [doc-example/libs/mathlib/src/numerics.h](doc-example_libs_mathlib_src_numerics_h.md) | 37 | Adaptive Simpson's rule for $\int_a^b f(x)\,dx$.  Uses recursive subdivision until the local error estimate satisfies $\|\Delta\| < \varepsilon / (b - a)$.  The composite rule on a single panel $[c, d]$ is:  $$S(c,d) = \frac{d-c}{6}\left[f(c) + 4f\!\left(\frac{c+d}{2}\right) + f(d)\right]$$  \| Parameter \| Meaning \| \|-----------\|---------\| \| `f`   \| Integrand (must be smooth on $[a,b]$) \| \| `a`   \| Lower limit \| \| `b`   \| Upper limit \| \| `eps` \| Absolute error tolerance $\varepsilon > 0$ \| |
| [fn `lerp`](doc-example_libs_mathlib_src_mathlib_h.md#fn-lerp-line-48) | [doc-example/libs/mathlib/src/mathlib.h](doc-example_libs_mathlib_src_mathlib_h.md) | 48 | Linear interpolation between two values. |
| [fn `lu_decompose`](doc-example_libs_mathlib_src_numerics_h.md#fn-lu-decompose-line-58) | [doc-example/libs/mathlib/src/numerics.h](doc-example_libs_mathlib_src_numerics_h.md) | 58 | LU decomposition of an $n \times n$ matrix $A$.  Factors $A = P L U$ where: - $P$ is a permutation matrix (partial pivoting) - $L$ is unit lower-triangular: $L_{ii} = 1$, $L_{ij} = 0$ for $j > i$ - $U$ is upper-triangular  The factorisation is stored **in-place** in `A`; the strictly lower part holds $L$ (without the diagonal ones) and the upper part holds $U$.  Complexity: $\mathcal{O}(n^3)$ flops. |
| [fn `lu_solve`](doc-example_libs_mathlib_src_numerics_h.md#fn-lu-solve-line-79) | [doc-example/libs/mathlib/src/numerics.h](doc-example_libs_mathlib_src_numerics_h.md) | 79 | Solve $Ax = b$ using a pre-computed LU factorisation.  Given the output of `lu_decompose`, solves $PLUx = b$ in two steps:  1. Forward substitution: $Ly = Pb$ — $\mathcal{O}(n^2)$ 2. Back substitution:    $Ux = y$  — $\mathcal{O}(n^2)$ |
| [fn `norm2`](doc-example_fortran_linalg_f90.md#fn-norm2-line-48) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 48 | Euclidean norm of a vector. |
| [fn `show`](doc-example_src_main_cpp.md#fn-show-line-14) | [doc-example/src/main.cpp](doc-example_src_main_cpp.md) | 14 | Print a labelled double value to stdout. |
| [fn `solve2`](doc-example_fortran_linalg_f90.md#fn-solve2-line-57) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 57 | Solve a 2×2 linear system Ax = b via Cramer's rule. |
| [fn `stats::algo::covariance`](doc-example_libs_stats_src_stats_h.md#fn-stats-algo-covariance-line-97) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 97 | Covariance of two equal-length samples.  $\text{Cov}(X,Y) = \frac{1}{n-1}\sum (x_i - \bar{x})(y_i - \bar{y})$ |
| [fn `stats::algo::linreg`](doc-example_libs_stats_src_stats_h.md#fn-stats-algo-linreg-line-84) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 84 | Ordinary least-squares linear regression.  Fits the model $y = a x + b$ by minimising $\sum(y_i - a x_i - b)^2$. |
| [fn `stats::mean`](doc-example_libs_stats_src_stats_h.md#fn-stats-mean-line-17) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 17 | Arithmetic mean of a sample. |
| [fn `stats::median`](doc-example_libs_stats_src_stats_h.md#fn-stats-median-line-65) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 65 | Median of the sample. |
| [fn `stats::pearson`](doc-example_libs_stats_src_stats_h.md#fn-stats-pearson-line-42) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 42 | Pearson correlation coefficient between two equal-length samples.  Returns a value in $[-1, 1]$.  Returns NaN when either sample has zero variance. |
| [fn `stats::percentile`](doc-example_libs_stats_src_stats_h.md#fn-stats-percentile-line-68) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 68 | Percentile via linear interpolation. |
| [fn `stats::stddev`](doc-example_libs_stats_src_stats_h.md#fn-stats-stddev-line-37) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 37 | Standard deviation of a sample. |
| [fn `stats::variance`](doc-example_libs_stats_src_stats_h.md#fn-stats-variance-line-26) | [doc-example/libs/stats/src/stats.h](doc-example_libs_stats_src_stats_h.md) | 26 | Sample variance (unbiased, Bessel-corrected).  Uses $s^2 = \frac{1}{n-1}\sum(x_i - \bar{x})^2$. |
| [fn `vec2_add`](doc-example_libs_mathlib_src_mathlib_h.md#fn-vec2-add-line-73) | [doc-example/libs/mathlib/src/mathlib.h](doc-example_libs_mathlib_src_mathlib_h.md) | 73 | Add two vectors component-wise. |
| [fn `vec2_length`](doc-example_libs_mathlib_src_mathlib_h.md#fn-vec2-length-line-68) | [doc-example/libs/mathlib/src/mathlib.h](doc-example_libs_mathlib_src_mathlib_h.md) | 68 | Compute the Euclidean length of v. |

## Modules

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [mod `linalg`](doc-example_fortran_linalg_f90.md#mod-linalg-line-1) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 1 | Basic linear algebra routines — Fortran with FORD-style comments. |

## Subroutines

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [sub `scale`](doc-example_fortran_linalg_f90.md#sub-scale-line-39) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 39 | Scale a vector by a scalar factor in-place. |

## Variables

| Symbol | File | Line | Summary |
|--------|------|-----:|---------|
| [var `default_lda`](doc-example_fortran_linalg_f90.md#var-default-lda-line-13) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 13 | Default leading dimension used for allocatable matrix layouts. |
| [var `dp`](doc-example_fortran_linalg_f90.md#var-dp-line-7) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 7 | Double-precision alias for clarity. |
| [var `pi`](doc-example_fortran_linalg_f90.md#var-pi-line-10) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 10 | Mathematical constant pi (double precision). |
| [var `singular_tol`](doc-example_fortran_linalg_f90.md#var-singular-tol-line-18) | [doc-example/fortran/linalg.f90](doc-example_fortran_linalg_f90.md) | 18 | Absolute tolerance used by `solve2` to detect singular matrices. |

