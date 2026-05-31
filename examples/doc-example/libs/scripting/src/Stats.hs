-- | Pure statistical helpers — Haskell showcase for docify.
--
-- All functions operate on lists of 'Double' values and are strict in
-- their arguments.  Functions that require a non-empty input signal
-- failure via 'Maybe'.
module Stats
    ( mean
    , variance
    , stddev
    , median
    , quantile
    , pearson
    ) where

import Data.List (sort)

-- | Arithmetic mean of a non-empty list.
--
-- Returns 'Nothing' for an empty list.
mean :: [Double] -> Maybe Double
mean [] = Nothing
mean xs = Just (sum xs / fromIntegral (length xs))

-- | Sample variance (Bessel-corrected, n − 1 denominator).
--
-- Returns 'Nothing' when the list has fewer than two elements.
--
-- See also: 'stddev'
variance :: [Double] -> Maybe Double
variance xs
    | length xs < 2 = Nothing
    | otherwise     = do
        m <- mean xs
        let diffs = map (\x -> (x - m) ^ (2 :: Int)) xs
        Just (sum diffs / fromIntegral (length xs - 1))

-- | Sample standard deviation.
--
-- Returns 'Nothing' for lists shorter than two elements.
--
-- See also: 'variance'
stddev :: [Double] -> Maybe Double
stddev = fmap sqrt . variance

-- | Median of a non-empty list.
--
-- Uses linear interpolation for even-length lists.
median :: [Double] -> Maybe Double
median [] = Nothing
median xs =
    let s = sort xs
        n = length s
        mid = n `div` 2
    in  if odd n
            then Just (s !! mid)
            else Just ((s !! (mid - 1) + s !! mid) / 2)

-- | Empirical quantile via linear interpolation.
--
-- @q@ must be in [0, 1].  Returns 'Nothing' for an empty list.
quantile :: Double -> [Double] -> Maybe Double
quantile _ [] = Nothing
quantile q xs =
    let s   = sort xs
        n   = length s
        pos = q * fromIntegral (n - 1)
        lo  = floor pos :: Int
        hi  = min (lo + 1) (n - 1)
        frac = pos - fromIntegral lo
    in  Just (s !! lo * (1 - frac) + s !! hi * frac)

-- | Pearson correlation coefficient of two equal-length lists.
--
-- Returns 'Nothing' if either input has zero variance or if the lists
-- have fewer than two elements.
pearson :: [Double] -> [Double] -> Maybe Double
pearson xs ys
    | length xs /= length ys = Nothing
    | otherwise = do
        mx <- mean xs
        my <- mean ys
        sx <- stddev xs
        sy <- stddev ys
        let n    = fromIntegral (length xs)
            cov  = sum (zipWith (\x y -> (x - mx) * (y - my)) xs ys) / (n - 1)
        if sx == 0 || sy == 0 then Nothing
        else Just (cov / (sx * sy))
