/**
 * @file interp.h
 * @brief Interpolation and curve evaluation — native-math public API.
 */
#pragma once
#include <array>
#include <vector>

namespace nm {

/**
 * @brief Linear interpolation between two scalars.
 *
 * @param a  Start value (t = 0).
 * @param b  End value   (t = 1).
 * @param t  Blend factor.  Values outside [0, 1] extrapolate.
 * @return   a + t*(b - a)
 */
inline double lerp(double a, double b, double t) noexcept {
    return a + t * (b - a);
}

/**
 * @brief Smooth Hermite interpolation (Ken Perlin's smoothstep).
 *
 * Remaps t ∈ [0, 1] with zero first derivatives at the endpoints,
 * producing an S-shaped curve.  Input is clamped to [0, 1].
 *
 * @param t  Blend factor.
 * @return   6t⁵ − 15t⁴ + 10t³
 * @see lerp
 */
double smoothstep(double t) noexcept;

/**
 * @brief Evaluate a cubic Bézier curve at parameter t.
 *
 * @param p0  Start point (t = 0).
 * @param p1  First control point.
 * @param p2  Second control point.
 * @param p3  End point (t = 1).
 * @param t   Curve parameter in [0, 1].
 * @return    Point on the curve at t.
 */
std::array<double, 2> bezier3(
    std::array<double, 2> p0,
    std::array<double, 2> p1,
    std::array<double, 2> p2,
    std::array<double, 2> p3,
    double t) noexcept;

/**
 * @brief Catmull-Rom spline through a sequence of control points.
 *
 * Generates `segments * (count-1)` uniformly-spaced samples along the
 * spline.  Requires at least 2 control points.
 *
 * @param pts      Control points (x, y pairs).
 * @param segments Number of line segments between consecutive knots.
 * @return         Sampled points in order.
 * @warning Behaviour is undefined when `pts` has fewer than 2 elements.
 */
std::vector<std::array<double, 2>>
catmull_rom(const std::vector<std::array<double, 2>>& pts, int segments = 16);

/**
 * @brief Clamp a value to the closed interval [lo, hi].
 *
 * @param v   Value to clamp.
 * @param lo  Lower bound.
 * @param hi  Upper bound.  Must satisfy hi >= lo.
 * @return    v clamped to [lo, hi].
 */
template <typename T>
constexpr T clamp(T v, T lo, T hi) noexcept { return v < lo ? lo : v > hi ? hi : v; }

/**
 * @brief Remap a value from one range to another.
 *
 * Linearly maps v from [in_lo, in_hi] → [out_lo, out_hi].
 * Does not clamp the output.
 *
 * @param v      Input value.
 * @param in_lo  Input range lower bound.
 * @param in_hi  Input range upper bound.
 * @param out_lo Output range lower bound.
 * @param out_hi Output range upper bound.
 * @return       Remapped value.
 * @see lerp
 */
double remap(double v, double in_lo, double in_hi,
             double out_lo, double out_hi) noexcept;

} // namespace nm
