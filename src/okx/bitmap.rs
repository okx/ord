use super::*;
#[derive(Debug, Clone)]
pub struct BitmapMessage {}

pub trait BitmapMessageExtractor<'a, 'tx> {
  fn extract_bitmap_message(&self) -> Result<Option<BitmapMessage>>;
}

impl<'a, 'tx> BitmapMessageExtractor<'a, 'tx> for OkxInscriptionEvent {
  fn extract_bitmap_message(&self) -> Result<Option<BitmapMessage>> {
    Ok(None)
  }
}

impl BitmapMessage {
  pub fn execute(&self, context: &mut TableContext, height: u32) -> Result<()> {
    Ok(())
  }
}
