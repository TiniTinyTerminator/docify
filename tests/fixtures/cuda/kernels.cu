/**
 * Add two vectors on the GPU.
 *
 * @param a left input vector
 * @param b right input vector
 * @param out output vector
 * @param n number of elements
 */
__global__ void vector_add(const float *a, const float *b, float *out, int n);

/// Clamp a value to the unit interval on device.
/// @param value input value
/// @return value clamped to [0, 1]
__device__ float clamp_unit(float value);

/**
 * Compute one block-local sum using shared memory.
 *
 * @param input global input buffer
 * @param partial one output value per block
 * @param n number of readable elements
 */
__global__ void reduce_sum(const float *input, float *partial, int n);

/**
 * Scale a vector in place from host or device code.
 *
 * @param data vector data
 * @param scale scale factor
 * @param n number of elements
 */
__host__ __device__ void scale_in_place(float *data, float scale, int n);
