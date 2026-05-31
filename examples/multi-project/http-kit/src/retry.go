// Package httpkit provides composable HTTP middleware and retry logic.
package httpkit

import (
	"context"
	"errors"
	"math"
	"net/http"
	"time"
)

// RetryPolicy controls how a failed request is retried.
type RetryPolicy struct {
	// MaxAttempts is the maximum total number of attempts (1 = no retry).
	MaxAttempts int

	// BaseDelay is the initial back-off duration.
	BaseDelay time.Duration

	// MaxDelay caps the exponential back-off.
	MaxDelay time.Duration

	// Multiplier is the growth factor per attempt (default 2.0).
	Multiplier float64
}

// DefaultRetryPolicy returns a policy with three attempts and
// exponential back-off starting at 100 ms, capped at 5 s.
func DefaultRetryPolicy() RetryPolicy {
	return RetryPolicy{
		MaxAttempts: 3,
		BaseDelay:   100 * time.Millisecond,
		MaxDelay:    5 * time.Second,
		Multiplier:  2.0,
	}
}

// Delay returns the back-off duration before attempt number n (1-based).
func (p RetryPolicy) Delay(n int) time.Duration {
	if n <= 1 {
		return 0
	}
	mul := p.Multiplier
	if mul <= 0 {
		mul = 2.0
	}
	d := float64(p.BaseDelay) * math.Pow(mul, float64(n-2))
	if d > float64(p.MaxDelay) {
		d = float64(p.MaxDelay)
	}
	return time.Duration(d)
}

// RetryTransport is an http.RoundTripper that retries requests according
// to a RetryPolicy.
//
// Only network errors and 5xx responses trigger a retry; client errors
// (4xx) are returned immediately.
type RetryTransport struct {
	// Base is the underlying transport.  Defaults to http.DefaultTransport.
	Base http.RoundTripper

	// Policy governs retry behaviour.
	Policy RetryPolicy

	// ShouldRetry returns true when a response warrants a retry.
	// Defaults to retrying all 5xx status codes.
	ShouldRetry func(*http.Response, error) bool
}

// RoundTrip executes the request, retrying on transient failures as
// defined by the transport's Policy and ShouldRetry predicate.
func (t *RetryTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	base := t.Base
	if base == nil {
		base = http.DefaultTransport
	}
	shouldRetry := t.ShouldRetry
	if shouldRetry == nil {
		shouldRetry = func(r *http.Response, err error) bool {
			return err != nil || (r != nil && r.StatusCode >= 500)
		}
	}
	var (
		resp *http.Response
		err  error
	)
	for attempt := 1; attempt <= t.Policy.MaxAttempts; attempt++ {
		if d := t.Policy.Delay(attempt); d > 0 {
			select {
			case <-req.Context().Done():
				return nil, req.Context().Err()
			case <-time.After(d):
			}
		}
		clone := req.Clone(req.Context())
		resp, err = base.RoundTrip(clone)
		if !shouldRetry(resp, err) {
			break
		}
		if resp != nil {
			resp.Body.Close()
		}
	}
	return resp, err
}

// ErrMaxRetries is returned when all retry attempts are exhausted.
var ErrMaxRetries = errors.New("httpkit: maximum retries exceeded")

// Get is a convenience wrapper that performs a GET with context and
// returns the response body as a byte slice.
//
// The caller is responsible for closing resp.Body when err is nil.
func Get(ctx context.Context, url string) (*http.Response, error) {
	req, err := http.NewRequestWithContext(ctx, http.MethodGet, url, nil)
	if err != nil {
		return nil, err
	}
	return http.DefaultClient.Do(req)
}
