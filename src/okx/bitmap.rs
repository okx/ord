use super::*;
#[derive(Debug, Clone)]
pub struct BitmapOperation {}

pub trait BitmapMessageExtractor<'a, 'tx> {
  fn extract_bitmap_message(&self) -> Result<Option<BitmapOperation>>;
}

impl BitmapMessageExtractor<'_, '_> for OkxInscriptionEvent {
  fn extract_bitmap_message(&self) -> Result<Option<BitmapOperation>> {
    Ok(None)
  }
}

impl BitmapOperation {
  pub(crate) fn execute(&self, context: &mut TableContext, height: u32) -> Result<()> {
    Ok(())
  }
}
