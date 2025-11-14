use {
  crate::metrics,
  axum::{
    extract::MatchedPath,
    http::{Request, Response},
  },
  pin_project::pin_project,
  std::{
    fmt::Display,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
    time::Instant,
  },
  tower::{Layer, Service},
};

/// Layer that adds metrics recording and tracing to requests
///
/// This middleware records:
/// - Request count by method and status code
/// - Request duration (response time) by method and status code
/// - Tracing spans for distributed tracing
///
/// ## High Cardinality Prevention
///
/// Uses route templates (e.g., `/api/v1/ord/id/:id`) instead of actual paths
/// (e.g., `/api/v1/ord/id/abc123...`) to prevent metric label explosion.
#[derive(Clone, Copy, Debug)]
pub struct MetricsLayer;

impl<S> Layer<S> for MetricsLayer {
  type Service = MetricsService<S>;

  fn layer(&self, inner: S) -> Self::Service {
    MetricsService { inner }
  }
}

/// Service that records request metrics
#[derive(Clone, Debug)]
pub struct MetricsService<S> {
  inner: S,
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for MetricsService<S>
where
  S: Service<Request<ReqBody>, Response = Response<ResBody>>,
  S::Error: Display,
{
  type Response = S::Response;
  type Error = S::Error;
  type Future = MetricsFuture<S::Future>;

  fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
    self.inner.poll_ready(cx)
  }

  fn call(&mut self, request: Request<ReqBody>) -> Self::Future {
    let method = request.method().clone();

    // Get matched path (route template) to avoid high cardinality
    // e.g., "/api/v1/ord/id/:id/inscription" instead of "/api/v1/ord/id/abc123.../inscription"
    let path = request
      .extensions()
      .get::<MatchedPath>()
      .map(|matched| matched.as_str().to_string())
      .unwrap_or_else(|| request.uri().path().to_string());

    let start = Instant::now();

    // Create tracing span
    let span = tracing::info_span!(
      "api_request",
      method = %method,
      path = %path,
      status_code = tracing::field::Empty,
    );

    MetricsFuture {
      inner: self.inner.call(request),
      method,
      path,
      start,
      span,
    }
  }
}

/// Future that completes the request and records metrics
#[pin_project]
pub struct MetricsFuture<F> {
  #[pin]
  inner: F,
  method: axum::http::Method,
  path: String,
  start: Instant,
  span: tracing::Span,
}

impl<F, ResBody, E> Future for MetricsFuture<F>
where
  F: Future<Output = Result<Response<ResBody>, E>>,
  E: Display,
{
  type Output = F::Output;

  fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
    let this = self.project();
    let _enter = this.span.enter();

    match this.inner.poll(cx) {
      Poll::Ready(result) => {
        let duration = this.start.elapsed();

        match &result {
          Ok(response) => {
            let status = response.status();

            // Record status in span
            this.span.record("status_code", status.as_u16());

            // Log request
            tracing::info!(
              method = %this.method,
              path = %this.path,
              status = status.as_u16(),
              duration_ms = duration.as_millis(),
              "API request completed"
            );

            // Record Prometheus metrics
            metrics::record_api_request(
              this.method.as_str(),
              &this.path,
              status.as_u16(),
              duration.as_secs_f64(),
            );
          }
          Err(err) => {
            tracing::error!(
              method = %this.method,
              path = %this.path,
              error = %err,
              duration_ms = duration.as_millis(),
              "API request failed"
            );
          }
        }

        Poll::Ready(result)
      }
      Poll::Pending => Poll::Pending,
    }
  }
}

/// Create metrics layer for axum router
pub fn metrics_layer() -> MetricsLayer {
  MetricsLayer
}
