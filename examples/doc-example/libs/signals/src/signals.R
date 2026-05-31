#' Signal analysis utilities — R showcase for docify.
#' @description
#' A collection of functions for basic time-series and signal processing
#' using base R only (no external packages required).

#' Compute the simple moving average of a numeric vector.
#'
#' @param x      Numeric vector of signal values.
#' @param window Positive integer window length.
#' @return Numeric vector of length \code{length(x) - window + 1}.
#' @examples
#' moving_avg(c(1, 2, 3, 4, 5), 3)  # 2.0 3.0 4.0
moving_avg <- function(x, window) {
  if (window < 1 || window > length(x))
    stop("window out of range")
  n <- length(x) - window + 1
  vapply(seq_len(n), function(i) mean(x[i:(i + window - 1)]), numeric(1))
}

#' Rescale a vector to the range [0, 1].
#'
#' @param x Numeric vector.
#' @return Numeric vector with values in [0, 1].
#'   Returns a zero vector when all values are identical.
#' @seealso \code{\link{moving_avg}}
normalize <- function(x) {
  lo <- min(x); hi <- max(x)
  if (hi == lo) return(rep(0, length(x)))
  (x - lo) / (hi - lo)
}

#' Root-mean-square amplitude of a signal.
#'
#' @param x Numeric vector of signal samples.
#' @return Non-negative scalar RMS value.
rms <- function(x) sqrt(mean(x^2))

#' Lag-k autocorrelation of a stationary signal.
#'
#' Uses the unbiased estimator (n - k denominator).
#'
#' @param x Numeric vector of length n >= 2.
#' @param k Non-negative integer lag (default 1).
#' @return Autocorrelation coefficient in [-1, 1].
autocorr <- function(x, k = 1L) {
  n  <- length(x)
  mu <- mean(x)
  v  <- sum((x - mu)^2) / n
  if (v == 0) return(NA_real_)
  sum((x[seq_len(n - k)] - mu) * (x[seq_len(n - k) + k] - mu)) / ((n - k) * v)
}
