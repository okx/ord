use super::*;

#[derive(Boilerplate)]
pub(crate) struct FAQsHtml {}

impl PageContent for FAQsHtml {
  fn title(&self) -> String {
    "Ordinals - FAQs".to_string()
  }
}