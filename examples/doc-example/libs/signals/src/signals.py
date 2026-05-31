"""
Signal processing utilities — Python showcase for docify.
"""

from typing import Sequence


def moving_average(xs: Sequence[float], window: int) -> list[float]:
    """Compute the simple moving average of a sequence.

    Args:
        xs:     Input values. Must not be empty.
        window: Number of samples to average. Must be >= 1 and <= len(xs).

    Returns:
        List of length ``len(xs) - window + 1`` containing the moving
        average values.

    Raises:
        ValueError: If *window* is less than 1 or larger than *xs*.

    Example:
        >>> moving_average([1, 2, 3, 4, 5], 3)
        [2.0, 3.0, 4.0]
    """
    if window < 1:
        raise ValueError(f"window must be >= 1, got {window}")
    if window > len(xs):
        raise ValueError(f"window ({window}) exceeds sequence length ({len(xs)})")
    return [sum(xs[i:i + window]) / window for i in range(len(xs) - window + 1)]


def normalize(xs: Sequence[float]) -> list[float]:
    """Rescale values to the range [0, 1].

    Maps ``min(xs)`` → 0 and ``max(xs)`` → 1 via linear interpolation.
    Returns a list of zeros when all values are identical.

    Args:
        xs: Non-empty sequence of real numbers.

    Returns:
        Normalized values in [0, 1].

    See Also:
        moving_average
    """
    lo, hi = min(xs), max(xs)
    span = hi - lo
    if span == 0:
        return [0.0] * len(xs)
    return [(x - lo) / span for x in xs]


def rms(xs: Sequence[float]) -> float:
    """Root-mean-square amplitude of a signal.

    Args:
        xs: Non-empty input sequence.

    Returns:
        sqrt(mean(x_i^2))
    """
    return (sum(x * x for x in xs) / len(xs)) ** 0.5


class RingBuffer:
    """Fixed-capacity circular buffer for streaming samples.

    Older samples are overwritten once the buffer is full, making this
    suitable for real-time windowing without dynamic allocation.

    Args:
        capacity: Maximum number of samples to hold. Must be >= 1.
    """

    def __init__(self, capacity: int) -> None:
        if capacity < 1:
            raise ValueError("capacity must be >= 1")
        self._buf: list[float] = [0.0] * capacity
        self._cap = capacity
        self._head = 0
        self._size = 0

    def push(self, value: float) -> None:
        """Append a sample, overwriting the oldest entry when full.

        Args:
            value: Sample to append.
        """
        self._buf[self._head] = value
        self._head = (self._head + 1) % self._cap
        if self._size < self._cap:
            self._size += 1

    def to_list(self) -> list[float]:
        """Return buffered samples in chronological order.

        Returns:
            Oldest-first list of up to *capacity* samples.
        """
        if self._size < self._cap:
            return self._buf[:self._size]
        tail = self._head
        return self._buf[tail:] + self._buf[:tail]

    @property
    def full(self) -> bool:
        """True when the buffer has reached capacity."""
        return self._size == self._cap
