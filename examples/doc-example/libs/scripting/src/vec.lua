--- @file vec.lua
--- @brief Immutable 2-D vector library — Lua showcase for docify.
---
--- All operations return new vectors; none mutate their arguments.

local Vec = {}
Vec.__index = Vec

--- Create a new 2-D vector.
--- @param x number  Horizontal component.
--- @param y number  Vertical component.
--- @return Vec      New vector {x, y}.
function Vec.new(x, y)
    return setmetatable({ x = x, y = y }, Vec)
end

--- Add two vectors component-wise.
--- @param a Vec  Left operand.
--- @param b Vec  Right operand.
--- @return Vec   a + b
function Vec.__add(a, b)
    return Vec.new(a.x + b.x, a.y + b.y)
end

--- Subtract vector b from a.
--- @param a Vec  Left operand.
--- @param b Vec  Right operand.
--- @return Vec   a - b
function Vec.__sub(a, b)
    return Vec.new(a.x - b.x, a.y - b.y)
end

--- Scale a vector by a scalar.
--- @param v     Vec     Vector to scale.
--- @param s     number  Scale factor.
--- @return Vec  v * s
function Vec.scale(v, s)
    return Vec.new(v.x * s, v.y * s)
end

--- Dot product of two vectors.
--- @param a Vec    First vector.
--- @param b Vec    Second vector.
--- @return number  a · b
function Vec.dot(a, b)
    return a.x * b.x + a.y * b.y
end

--- Euclidean length of a vector.
--- @param v Vec    Input vector.
--- @return number  ||v||
function Vec.length(v)
    return math.sqrt(v.x * v.x + v.y * v.y)
end

--- Return a unit vector in the same direction as v.
--- Returns the zero vector when ||v|| is zero.
--- @param v Vec  Input vector.
--- @return Vec   v / ||v||, or {0,0} if v is zero.
--- @see Vec.length
function Vec.normalize(v)
    local len = Vec.length(v)
    if len == 0 then return Vec.new(0, 0) end
    return Vec.new(v.x / len, v.y / len)
end

--- Linear interpolation between two vectors.
--- @param a Vec     Start vector (t = 0).
--- @param b Vec     End vector   (t = 1).
--- @param t number  Blend factor in [0, 1].
--- @return Vec      Blended vector.
function Vec.lerp(a, b, t)
    return Vec.new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
end

--- Human-readable string representation.
--- @return string  e.g. "(1.00, 2.50)"
function Vec:__tostring()
    return string.format("(%.2f, %.2f)", self.x, self.y)
end

return Vec
