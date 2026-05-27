/**
 * Fixed-capacity stack of integers backed by an array.
 *
 * <p>Elements are stored in LIFO order. The stack does not resize after
 * construction; callers must choose a capacity that suits their workload.
 */
public final class IntStack {

    private final int[] data;
    private int top;

    /**
     * Construct an empty stack with the given capacity.
     *
     * @param capacity Maximum number of elements.
     * @throws IllegalArgumentException if {@code capacity} is negative.
     */
    public IntStack(int capacity) {
        if (capacity < 0) throw new IllegalArgumentException("negative capacity");
        data = new int[capacity];
        top  = 0;
    }

    /**
     * Push a value onto the top of the stack.
     *
     * @param value The integer to push.
     * @throws IllegalStateException if the stack is full.
     */
    public void push(int value) {
        if (top == data.length) throw new IllegalStateException("stack full");
        data[top++] = value;
    }

    /**
     * Remove and return the element at the top of the stack.
     *
     * @return The popped value.
     * @throws IllegalStateException if the stack is empty.
     */
    public int pop() {
        if (top == 0) throw new IllegalStateException("stack empty");
        return data[--top];
    }

    /**
     * Return the element at the top without removing it.
     *
     * @return The top element.
     * @throws IllegalStateException if the stack is empty.
     */
    public int peek() {
        if (top == 0) throw new IllegalStateException("stack empty");
        return data[top - 1];
    }

    /**
     * Return the number of elements currently on the stack.
     *
     * @return Element count in [0, capacity].
     */
    public int size() {
        return top;
    }

    /**
     * Return {@code true} if the stack contains no elements.
     *
     * @return {@code true} if empty.
     */
    public boolean isEmpty() {
        return top == 0;
    }
}

/**
 * Contract for collections that support snapshot serialisation.
 *
 * <p>Implementing classes must produce a compact, portable byte representation
 * that can be used to restore the collection state later.
 */
public interface Snapshot {

    /**
     * Write the current state into {@code out} starting at {@code offset}.
     *
     * @param out    Destination byte array.
     * @param offset Starting byte offset in {@code out}.
     * @return Number of bytes written.
     * @throws IllegalArgumentException if {@code out} is too small.
     */
    int writeSnapshot(byte[] out, int offset);

    /**
     * Return the number of bytes required to snapshot this collection.
     *
     * @return Byte count, always positive.
     */
    int snapshotSize();
}

/**
 * Supported collection types.
 */
public enum CollectionKind {
    /** A resizable array. */
    LIST,
    /** A hash-based associative map. */
    MAP,
    /** A sorted map backed by a red-black tree. */
    SORTED_MAP,
    /** A first-in-first-out queue. */
    QUEUE
}

/**
 * Immutable view of stack usage statistics.
 *
 * @param size Current number of elements.
 * @param capacity Maximum number of elements.
 */
public record StackStats(int size, int capacity) {

    /**
     * Return the fraction of slots currently in use.
     *
     * @return A value in the range {@code [0.0, 1.0]}.
     */
    public double loadFactor() {
        return capacity == 0 ? 0.0 : (double) size / capacity;
    }
}

/**
 * Utility methods for working with primitive collection examples.
 */
public final class CollectionMath {

    /**
     * Clamp {@code value} into the inclusive range {@code [min, max]}.
     *
     * @param value Value to clamp.
     * @param min Lower bound.
     * @param max Upper bound.
     * @return The clamped value.
     */
    public static int clamp(int value, int min, int max) {
        return Math.max(min, Math.min(max, value));
    }
}
