use crate::okx::datastore::cache::CacheTableIndex;
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
pub struct TraceNode {
  pub trace_type: CacheTableIndex,
  pub key: Vec<u8>,
}
