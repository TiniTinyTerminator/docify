// csvkit — C# showcase for docify.
using System;
using System.Collections.Generic;
using System.IO;

namespace CsvKit
{
    /// <summary>
    /// Reads RFC-4180 CSV files row by row.
    /// </summary>
    /// <remarks>
    /// Handles quoted fields with embedded commas and newlines.
    /// The first row is always treated as a header.
    /// </remarks>
    public sealed class CsvReader : IDisposable
    {
        private readonly StreamReader _reader;
        private bool _disposed;

        /// <summary>The column names read from the header row.</summary>
        public IReadOnlyList<string> Headers { get; }

        /// <summary>
        /// Open a CSV file for reading.
        /// </summary>
        /// <param name="path">Path to the CSV file.</param>
        /// <exception cref="FileNotFoundException">
        /// Thrown when <paramref name="path"/> does not exist.
        /// </exception>
        public CsvReader(string path)
        {
            _reader = new StreamReader(path);
            var headerLine = _reader.ReadLine()
                ?? throw new InvalidDataException("CSV file is empty.");
            Headers = ParseRow(headerLine);
        }

        /// <summary>
        /// Read the next data row as a dictionary keyed by column name.
        /// </summary>
        /// <returns>
        /// A dictionary mapping each header to its field value, or
        /// <c>null</c> when the end of file is reached.
        /// </returns>
        public Dictionary<string, string>? ReadRow()
        {
            var line = _reader.ReadLine();
            if (line is null) return null;
            var fields = ParseRow(line);
            var row = new Dictionary<string, string>(Headers.Count);
            for (int i = 0; i < Headers.Count; i++)
                row[Headers[i]] = i < fields.Count ? fields[i] : string.Empty;
            return row;
        }

        /// <summary>Read all remaining rows into a list.</summary>
        /// <returns>List of row dictionaries; empty if at end of file.</returns>
        public List<Dictionary<string, string>> ReadAll()
        {
            var rows = new List<Dictionary<string, string>>();
            Dictionary<string, string>? row;
            while ((row = ReadRow()) is not null)
                rows.Add(row);
            return rows;
        }

        /// <inheritdoc />
        public void Dispose()
        {
            if (!_disposed) { _reader.Dispose(); _disposed = true; }
        }

        private static List<string> ParseRow(string line)
        {
            var fields = new List<string>();
            int i = 0;
            while (i <= line.Length)
            {
                if (i == line.Length) { fields.Add(string.Empty); break; }
                if (line[i] == '"')
                {
                    i++;
                    var sb = new System.Text.StringBuilder();
                    while (i < line.Length)
                    {
                        if (line[i] == '"' && i + 1 < line.Length && line[i + 1] == '"')
                        { sb.Append('"'); i += 2; }
                        else if (line[i] == '"') { i++; break; }
                        else { sb.Append(line[i++]); }
                    }
                    fields.Add(sb.ToString());
                    if (i < line.Length && line[i] == ',') i++;
                }
                else
                {
                    int end = line.IndexOf(',', i);
                    if (end < 0) { fields.Add(line[i..]); break; }
                    fields.Add(line[i..end]);
                    i = end + 1;
                }
            }
            return fields;
        }
    }

    /// <summary>
    /// Fluent builder for constructing CSV strings programmatically.
    /// </summary>
    public sealed class CsvWriter
    {
        private readonly List<string> _headers;
        private readonly List<IReadOnlyList<string>> _rows = new();

        /// <summary>
        /// Initialise the writer with a fixed set of column names.
        /// </summary>
        /// <param name="headers">Column names, in order.</param>
        public CsvWriter(IEnumerable<string> headers)
        {
            _headers = new List<string>(headers);
        }

        /// <summary>
        /// Append a row of values.
        /// </summary>
        /// <param name="values">
        /// Field values in the same order as the headers.
        /// Missing trailing values default to empty string.
        /// </param>
        /// <returns>This writer, for chaining.</returns>
        public CsvWriter AddRow(IEnumerable<string> values)
        {
            _rows.Add(new List<string>(values));
            return this;
        }

        /// <summary>Render the accumulated rows to a CSV string.</summary>
        /// <returns>RFC-4180 CSV text including the header row.</returns>
        public override string ToString()
        {
            var sb = new System.Text.StringBuilder();
            sb.AppendLine(string.Join(",", _headers.Select(EscapeField)));
            foreach (var row in _rows)
                sb.AppendLine(string.Join(",", row.Select(EscapeField)));
            return sb.ToString();
        }

        private static string EscapeField(string field) =>
            field.Contains(',') || field.Contains('"') || field.Contains('\n')
                ? $"\"{field.Replace("\"", "\"\"")}\"" : field;
    }
}
