use super::*;
use prometheus::{Encoder, TextEncoder};

pub(crate) async fn metrics_handler(Extension(index): Extension<Arc<Index>>) -> ServerResult {
  task::block_in_place(|| {
    let Some(registry) = index.get_metric_registry() else {
      return Err(ServerError::Internal(anyhow!("registry are not found")));
    };
    let encoder = TextEncoder::new();
    let metric_families = registry.gather();
    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
      return Err(ServerError::Internal(e.into()));
    }
    Ok(
      String::from_utf8(buffer)
        .map_err(|_| ServerError::Internal(anyhow!("Failed to convert metrics to UTF-8")))?
        .into_response(),
    )
  })
}
