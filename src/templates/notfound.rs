use super::*;

#[derive(Boilerplate)]
pub(crate) struct NotFoundHtml {}

impl PageContent for NotFoundHtml {
  fn title(&self) -> String {
    "Ordinals - 404".to_string()
  }
}