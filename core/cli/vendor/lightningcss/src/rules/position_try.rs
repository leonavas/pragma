//! The `@position-try` rule.

use super::Location;
use crate::declaration::DeclarationBlock;
use crate::error::PrinterError;
use crate::printer::Printer;
use crate::traits::ToCss;
use crate::values::ident::DashedIdent;

/// A [@position-try](https://drafts.csswg.org/css-anchor-position-1/#position-try) rule.
#[derive(Debug, PartialEq, Clone)]
pub struct PositionTryRule<'i> {
  /// The name of the position-try fallback.
  pub name: DashedIdent<'i>,
  /// Declarations in the `@position-try` rule.
  pub declarations: DeclarationBlock<'i>,
  /// The location of the rule in the source file.
  pub loc: Location,
}

impl<'i> ToCss for PositionTryRule<'i> {
  fn to_css<W>(&self, dest: &mut Printer<W>) -> Result<(), PrinterError>
  where
    W: std::fmt::Write,
  {
    dest.write_str("@position-try ")?;
    self.name.to_css(dest)?;
    self.declarations.to_css_block(dest)
  }
}
