<?php

declare(strict_types=1);

namespace Example\PhpUtils;

/**
 * Immutable wrapper around an array that provides a fluent query API.
 *
 * All transformation methods return a new Collection; the original is
 * never mutated.  The underlying array is always re-indexed after each
 * operation.
 */
class Collection
{
    /** @var list<mixed> */
    private readonly array $items;

    /**
     * @param list<mixed> $items Initial items.
     */
    public function __construct(array $items = [])
    {
        $this->items = array_values($items);
    }

    /**
     * Create a Collection from any iterable.
     *
     * @param iterable<mixed> $source Values to wrap.
     * @return static
     */
    public static function from(iterable $source): static
    {
        return new static(iterator_to_array($source, false));
    }

    /**
     * Return a new Collection containing only items for which
     * $predicate returns true.
     *
     * @param callable(mixed): bool $predicate Filter function.
     * @return static
     */
    public function filter(callable $predicate): static
    {
        return new static(array_values(array_filter($this->items, $predicate)));
    }

    /**
     * Return a new Collection with $callback applied to every item.
     *
     * @param callable(mixed): mixed $callback Transformation.
     * @return static
     */
    public function map(callable $callback): static
    {
        return new static(array_map($callback, $this->items));
    }

    /**
     * Reduce the collection to a single value.
     *
     * @param callable(mixed, mixed): mixed $callback Reducer function.
     * @param mixed                         $initial  Starting accumulator.
     * @return mixed The final accumulated value.
     */
    public function reduce(callable $callback, mixed $initial = null): mixed
    {
        return array_reduce($this->items, $callback, $initial);
    }

    /**
     * Return the first item matching $predicate, or $default if none match.
     *
     * @param callable(mixed): bool $predicate Search predicate.
     * @param mixed                 $default   Fallback value.
     * @return mixed
     */
    public function first(callable $predicate, mixed $default = null): mixed
    {
        foreach ($this->items as $item) {
            if ($predicate($item)) return $item;
        }
        return $default;
    }

    /**
     * Return a new Collection sorted by the return value of $key.
     *
     * @param callable(mixed): mixed $key Comparison key extractor.
     * @return static
     */
    public function sortBy(callable $key): static
    {
        $items = $this->items;
        usort($items, fn($a, $b) => $key($a) <=> $key($b));
        return new static($items);
    }

    /** Number of items in the collection. */
    public function count(): int
    {
        return count($this->items);
    }

    /** Convert to a plain PHP array. */
    public function toArray(): array
    {
        return $this->items;
    }
}
