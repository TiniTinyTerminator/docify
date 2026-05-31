/**
 * @file geometry.ts
 * @brief 2-D and 3-D geometry primitives — TypeScript showcase for docify.
 */

/** An immutable point or free vector in two dimensions. */
export interface Vec2 {
    readonly x: number;
    readonly y: number;
}

/** An immutable point or free vector in three dimensions. */
export interface Vec3 {
    readonly x: number;
    readonly y: number;
    readonly z: number;
}

/**
 * Compute the Euclidean distance between two 2-D points.
 *
 * @param a - First point.
 * @param b - Second point.
 * @returns Non-negative distance ||a - b||.
 */
export function distance2(a: Vec2, b: Vec2): number {
    const dx = a.x - b.x;
    const dy = a.y - b.y;
    return Math.sqrt(dx * dx + dy * dy);
}

/**
 * Dot product of two 3-D vectors.
 *
 * @param u - First vector.
 * @param v - Second vector.
 * @returns Scalar u · v.
 */
export function dot3(u: Vec3, v: Vec3): number {
    return u.x * v.x + u.y * v.y + u.z * v.z;
}

/**
 * Cross product of two 3-D vectors.
 *
 * @param u - First vector.
 * @param v - Second vector.
 * @returns Vector perpendicular to both *u* and *v* with magnitude
 *          ||u|| * ||v|| * sin(θ).
 */
export function cross(u: Vec3, v: Vec3): Vec3 {
    return {
        x: u.y * v.z - u.z * v.y,
        y: u.z * v.x - u.x * v.z,
        z: u.x * v.y - u.y * v.x,
    };
}

/**
 * Axis-aligned bounding box in 2-D.
 *
 * Tracks the smallest rectangle that contains all added points.
 */
export class BoundingBox {
    private minX = Infinity;
    private minY = Infinity;
    private maxX = -Infinity;
    private maxY = -Infinity;

    /**
     * Expand the box to include *point*.
     *
     * @param point - Point to incorporate.
     */
    add(point: Vec2): void {
        if (point.x < this.minX) this.minX = point.x;
        if (point.y < this.minY) this.minY = point.y;
        if (point.x > this.maxX) this.maxX = point.x;
        if (point.y > this.maxY) this.maxY = point.y;
    }

    /**
     * Test whether *point* lies inside or on the boundary of the box.
     *
     * @param point - Point to test.
     * @returns `true` if the point is contained.
     */
    contains(point: Vec2): boolean {
        return point.x >= this.minX && point.x <= this.maxX
            && point.y >= this.minY && point.y <= this.maxY;
    }

    /** Area of the bounding box. Returns 0 if no points have been added. */
    get area(): number {
        if (!isFinite(this.minX)) return 0;
        return (this.maxX - this.minX) * (this.maxY - this.minY);
    }
}

/**
 * Linear interpolation between two 2-D points.
 *
 * @param a - Start point (t = 0).
 * @param b - End point   (t = 1).
 * @param t - Interpolation parameter. Values outside [0, 1] extrapolate.
 * @returns Point at position t along the segment from a to b.
 * @see distance2
 */
export function lerp2(a: Vec2, b: Vec2, t: number): Vec2 {
    return { x: a.x + (b.x - a.x) * t, y: a.y + (b.y - a.y) * t };
}
