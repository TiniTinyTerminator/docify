"""
Minimal columnar data frame — analysis toolkit showcase.
"""

from __future__ import annotations
from typing import Any, Callable, Iterator


class DataFrame:
    """Lightweight in-memory columnar data store.

    Stores data as a dict of equal-length lists.  Column order is
    preserved (insertion order of the initial dict).

    Args:
        data: Mapping of column name → list of values.  All lists must
              have the same length.

    Raises:
        ValueError: When column lengths differ.

    Example:
        >>> df = DataFrame({"x": [1, 2, 3], "y": [4, 5, 6]})
        >>> df.shape
        (3, 2)
    """

    def __init__(self, data: dict[str, list[Any]]) -> None:
        lengths = {len(v) for v in data.values()}
        if len(lengths) > 1:
            raise ValueError("All columns must have the same length")
        self._data: dict[str, list[Any]] = {k: list(v) for k, v in data.items()}

    @property
    def shape(self) -> tuple[int, int]:
        """Number of rows and columns as ``(nrows, ncols)``."""
        ncols = len(self._data)
        nrows = len(next(iter(self._data.values()))) if ncols else 0
        return nrows, ncols

    @property
    def columns(self) -> list[str]:
        """Column names in order."""
        return list(self._data)

    def __len__(self) -> int:
        return self.shape[0]

    def __iter__(self) -> Iterator[dict[str, Any]]:
        """Iterate over rows as dicts."""
        nrows = self.shape[0]
        for i in range(nrows):
            yield {col: vals[i] for col, vals in self._data.items()}

    def select(self, *columns: str) -> "DataFrame":
        """Return a new DataFrame with only the specified columns.

        Args:
            *columns: Column names to keep.

        Returns:
            DataFrame containing only the requested columns.

        Raises:
            KeyError: When a column name is not present.
        """
        return DataFrame({col: self._data[col] for col in columns})

    def filter(self, predicate: Callable[[dict[str, Any]], bool]) -> "DataFrame":
        """Return rows for which *predicate* returns True.

        Args:
            predicate: Callable that receives a row dict and returns bool.

        Returns:
            New DataFrame containing only the matching rows.
        """
        rows = [row for row in self if predicate(row)]
        if not rows:
            return DataFrame({col: [] for col in self.columns})
        return DataFrame({col: [r[col] for r in rows] for col in self.columns})

    def apply(self, column: str, func: Callable[[Any], Any]) -> "DataFrame":
        """Apply *func* element-wise to *column*, returning a new DataFrame.

        Args:
            column:  Name of the column to transform.
            func:    Transformation applied to each element.

        Returns:
            New DataFrame with the transformed column.

        Raises:
            KeyError: When *column* is not present.
        """
        new_data = dict(self._data)
        new_data[column] = [func(v) for v in self._data[column]]
        return DataFrame(new_data)

    def add_column(self, name: str, values: list[Any]) -> "DataFrame":
        """Return a new DataFrame with an extra column appended.

        Args:
            name:   Column name.  Must not already exist.
            values: Values — must match the current row count.

        Returns:
            New DataFrame with the additional column.

        Raises:
            ValueError: When *name* already exists or lengths differ.
        """
        if name in self._data:
            raise ValueError(f"Column '{name}' already exists")
        return DataFrame({**self._data, name: values})


def read_csv(path: str, delimiter: str = ",") -> DataFrame:
    """Read a delimited text file into a DataFrame.

    The first line is treated as the header row.

    Args:
        path:      Path to the CSV file.
        delimiter: Field separator character (default comma).

    Returns:
        DataFrame whose columns correspond to the header fields.

    Raises:
        FileNotFoundError: When *path* does not exist.
        ValueError:        When the file has no rows beyond the header.
    """
    with open(path, encoding="utf-8") as fh:
        lines = [l.rstrip("\n") for l in fh]
    if not lines:
        raise ValueError(f"{path!r} is empty")
    headers = lines[0].split(delimiter)
    data: dict[str, list[Any]] = {h: [] for h in headers}
    for line in lines[1:]:
        for h, v in zip(headers, line.split(delimiter)):
            data[h].append(v)
    return DataFrame(data)
