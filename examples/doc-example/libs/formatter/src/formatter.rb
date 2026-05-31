# formatter.rb — Ruby showcase for docify.
# Provides text formatting and table-rendering utilities.

module Formatter
  # Format a floating-point number as a fixed-width string.
  #
  # @param value  [Float]   Number to format.
  # @param width  [Integer] Minimum total character width (default 10).
  # @param digits [Integer] Decimal places (default 3).
  # @return [String] Right-aligned formatted string.
  #
  # @example
  #   Formatter.float_field(3.14159, 8, 2)  # => "    3.14"
  def self.float_field(value, width = 10, digits = 3)
    format("%#{width}.#{digits}f", value)
  end

  # Wrap *text* to at most *max_width* characters per line.
  #
  # Breaks on whitespace boundaries; words longer than *max_width* are
  # placed on their own line without breaking.
  #
  # @param text      [String]  Text to wrap.
  # @param max_width [Integer] Maximum line width in characters.
  # @return [String] Wrapped text with newline separators.
  def self.word_wrap(text, max_width = 72)
    words = text.split
    lines = []
    current = +""
    words.each do |word|
      if current.empty?
        current << word
      elsif current.length + 1 + word.length <= max_width
        current << " " << word
      else
        lines << current
        current = +word
      end
    end
    lines << current unless current.empty?
    lines.join("\n")
  end

  # Render a two-dimensional array as a plain-text ASCII table.
  #
  # The first element of *rows* is treated as the header row.
  # Column widths are computed from the widest entry in each column.
  #
  # @param rows    [Array<Array<String>>] Table data; first row is header.
  # @param padding [Integer] Horizontal padding on each side of a cell.
  # @return [String] Formatted table as a multi-line string.
  # @see word_wrap
  def self.ascii_table(rows, padding = 1)
    return "" if rows.empty?
    cols = rows.first.length
    widths = Array.new(cols, 0)
    rows.each do |row|
      row.each_with_index { |cell, c| widths[c] = [widths[c], cell.to_s.length].max }
    end
    pad = " " * padding
    sep = "+" + widths.map { |w| "-" * (w + padding * 2) }.join("+") + "+"
    lines = [sep]
    rows.each_with_index do |row, i|
      cells = row.each_with_index.map { |cell, c| "#{pad}#{cell.to_s.ljust(widths[c])}#{pad}" }
      lines << "|" + cells.join("|") + "|"
      lines << sep if i == 0
    end
    lines << sep
    lines.join("\n")
  end

  # Strip ANSI colour escape sequences from a string.
  #
  # @param str [String] Potentially colour-coded terminal string.
  # @return [String] Plain string without escape codes.
  def self.strip_ansi(str)
    str.gsub(/\e\[[0-9;]*m/, "")
  end
end
