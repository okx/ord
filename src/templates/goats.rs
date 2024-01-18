use super::*;

#[derive(Boilerplate)]
pub(crate) struct GoatsHtml {}

impl PageContent for GoatsHtml {
  fn title(&self) -> String {
    "Ordinals - Goats".to_string()
  }
}