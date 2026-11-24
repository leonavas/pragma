//! The `@namespace` rule.

use super::Location;
use crate::error::PrinterError;
use crate::printer::Printer;
use crate::traits::ToCss;
use crate::values::ident::Ident;
use crate::values::string::CSSString;

/// A [@namespace](https://drafts.csswg.org/css-namespaces/#declaration) rule.
#[derive(Debug, PartialEq, Clone)]
pub struct NamespaceRule<'i> {
  /// An optional namespace prefix to declare, or `None` to declare the default namespace.
  pub prefix: Option<Ident<'i>>,
  /// The url of the namespace.
  pub url: CSSString<'i>,
  /// The location of the rule in the source file.
  pub loc: Location,
}

impl<'i> ToCss for NamespaceRule<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str("@namespace ")?;
    if let Some(prefix) = &self.prefix {
      prefix.to_css(dest)?;
      dest.write_char(' ')?;
    }

    self.url.to_css(dest)?;
    dest.write_char(';')
  }
}
