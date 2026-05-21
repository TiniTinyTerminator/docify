package geom

import "math"

// Vec2 is a 2-D vector with float64 components.
type Vec2 struct {
	X, Y float64
}

// Add returns the component-wise sum of v and w.
func (v Vec2) Add(w Vec2) Vec2 {
	return Vec2{v.X + w.X, v.Y + w.Y}
}

// Scale returns v multiplied by scalar s.
func (v Vec2) Scale(s float64) Vec2 {
	return Vec2{v.X * s, v.Y * s}
}

// Dot returns the dot product of v and w.
func (v Vec2) Dot(w Vec2) float64 {
	return v.X*w.X + v.Y*w.Y
}

// Length returns the Euclidean length of v.
func (v Vec2) Length() float64 {
	return math.Sqrt(v.Dot(v))
}

// Normalise returns a unit vector in the direction of v.
// Returns the zero vector when the length of v is zero.
func Normalise(v Vec2) Vec2 {
	l := v.Length()
	if l == 0 {
		return Vec2{}
	}
	return v.Scale(1 / l)
}

// Lerp linearly interpolates from a to b by factor t.
// t is clamped to [0, 1] before use.
func Lerp(a, b Vec2, t float64) Vec2 {
	if t < 0 {
		t = 0
	}
	if t > 1 {
		t = 1
	}
	return a.Scale(1 - t).Add(b.Scale(t))
}

// Shape is implemented by any closed planar region.
type Shape interface {
	Area() float64
	Perimeter() float64
}

// Rect is an axis-aligned rectangle at position (X, Y) with width W and height H.
type Rect struct {
	X, Y float64
	W, H float64
}

// Area returns the area of r.
func (r Rect) Area() float64 { return r.W * r.H }

// Perimeter returns the perimeter of r.
func (r Rect) Perimeter() float64 { return 2 * (r.W + r.H) }
