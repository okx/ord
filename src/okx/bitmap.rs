use super::*;
#[derive(Debug, Clone)]
pub struct BitmapMessage {}

pub trait BitmapMessageExtractor<'a, 'tx> {
  fn extract_bitmap_message(&self) -> Result<Option<BitmapMessage>>;
}

impl BitmapMessageExtractor<'_, '_> for OkxInscriptionEvent {
  fn extract_bitmap_message(&self) -> Result<Option<BitmapMessage>> {
    Ok(None)
  }
}

impl BitmapMessage {
  pub fn execute(&self, context: &mut TableContext, height: u32) -> Result<()> {
    Ok(())
  }
}
