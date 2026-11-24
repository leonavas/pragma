#![allow(non_snake_case)]
use super::{Property, PropertyId};
use crate::context::PropertyHandlerContext;
use crate::declaration::DeclarationList;
use crate::prefixes::Feature;
use crate::traits::{FallbackValues, IsCompatible, PropertyHandler};
use crate::vendor_prefix::VendorPrefix;

macro_rules! define_prefixes {
  (
    $($name: ident, )+
  ) => {
    #[derive(Default)]
    pub(crate) struct PrefixHandler {
      $(
        $name: Option<usize>,
      )+
    }

    impl<'i> PropertyHandler<'i> for PrefixHandler {
      fn handle_property(&mut self, property: &Property<'i>, dest: &mut DeclarationList<'i>, context: &mut PropertyHandlerContext) -> bool {
        match property {
          $(
            Property::$name(val, prefix) => {
              // Update the prefixes based on the targets.
              let new_prefixes = context.targets.prefixes(*prefix, Feature::$name);

              if let Some(i) = self.$name {
                if let Some(Property::$name(cur, prefixes)) = dest.get_mut(i) {
                  // If the value is the same, merge the prefixes.
                  if val == cur {
                    *prefixes = context.targets.prefixes(*prefixes | *prefix, Feature::$name);
                    return true
                  }

                  // If the new declaration covers every prefix of the previous one,
                  // the previous one is fully overridden: update it in place.
                  // Otherwise (e.g. `user-select: none; -webkit-user-select: text`),
                  // replacing would drop the declarations for the prefixes that the
                  // new one does not cover (the standard one), so append instead.
                  if new_prefixes.contains(*prefixes) {
                    *cur = val.clone();
                    *prefixes = new_prefixes;
                    return true
                  }
                }
              }

              // Store the index of the property, so we can update it later.
              self.$name = Some(dest.len());
              dest.push(Property::$name(val.clone(), new_prefixes))
            }
          )+
          _ => return false
        }

        true
      }

      fn finalize(&mut self, _: &mut DeclarationList, _: &mut PropertyHandlerContext) {}
    }
  };
}

define_prefixes! {
  TransformOrigin,
  TransformStyle,
  BackfaceVisibility,
  Perspective,
  PerspectiveOrigin,
  BoxSizing,
  TabSize,
  Hyphens,
  TextAlignLast,
  TextDecorationSkipInk,
  TextOverflow,
  UserSelect,
  Appearance,
  ClipPath,
  BoxDecorationBreak,
  TextSizeAdjust,
  PrintColorAdjust,
}

macro_rules! define_fallbacks {
  (
    $($name: ident$(($p: ident))?, )+
  ) => {
    pastey::paste! {
      #[derive(Default)]
      pub(crate) struct FallbackHandler {
        $(
          [<$name:snake>]: Option<usize>
        ),+
      }
    }

    impl<'i> PropertyHandler<'i> for FallbackHandler {
      fn handle_property(&mut self, property: &Property<'i>, dest: &mut DeclarationList<'i>, context: &mut PropertyHandlerContext<'i, '_>) -> bool {
        match property {
          $(
            Property::$name(val $(, mut $p)?) => {
              let mut val = val.clone();
              $(
                $p = context.targets.prefixes($p, Feature::$name);
              )?
              if pastey::paste! { self.[<$name:snake>] }.is_none() {
                let fallbacks = val.get_fallbacks(context.targets);
                #[allow(unused_variables)]
                let has_fallbacks = !fallbacks.is_empty();
                for fallback in fallbacks {
                  dest.push(Property::$name(fallback $(, $p)?))
                }

                $(
                  if has_fallbacks && $p.contains(VendorPrefix::None) {
                    $p = VendorPrefix::None;
                  }
                )?
              }

              if pastey::paste! { self.[<$name:snake>] }.is_none() || matches!(context.targets.browsers, Some(targets) if !val.is_compatible(targets)) {
                pastey::paste! { self.[<$name:snake>] = Some(dest.len()) };
                dest.push(Property::$name(val $(, $p)?));
              } else if let Some(index) = pastey::paste! { self.[<$name:snake>] } {
                let new_decl = Property::$name(val.clone() $(, $p)?);
                let cur_prefix = dest[index].property_id().prefix();
                let new_prefix = new_decl.property_id().prefix();
                if new_prefix.contains(cur_prefix) {
                  // The new declaration covers every prefix of the previous one
                  // (always true for unprefixed properties): override it.
                  dest[index] = new_decl;
                } else {
                  // Different vendor prefixes, e.g. `backdrop-filter: x; -webkit-backdrop-filter: x`.
                  // Never replace the previous declaration here, or the prefixes it covers
                  // (usually the standard one) would be lost.
                  let mut same_value = new_decl.clone();
                  same_value.set_prefix(cur_prefix);
                  if same_value == dest[index] {
                    // Same value: merge the prefixes into the previous declaration.
                    $(
                      $p = context.targets.prefixes(cur_prefix | $p, Feature::$name);
                    )?
                    dest[index] = Property::$name(val $(, $p)?);
                  } else {
                    // Different value: keep both, in source order.
                    pastey::paste! { self.[<$name:snake>] = Some(dest.len()) };
                    dest.push(new_decl);
                  }
                }
              }
            }
          )+
          Property::Unparsed(val) => {
            let (mut unparsed, index) = match val.property_id {
              $(
                PropertyId::$name$(($p))? => {
                  macro_rules! get_prefixed {
                    ($vp: ident) => {
                      if $vp.contains(VendorPrefix::None) {
                        val.get_prefixed(context.targets, Feature::$name)
                      } else {
                        val.clone()
                      }
                    };
                    () => {
                      val.clone()
                    };
                  }

                  let val = get_prefixed!($($p)?);
                  (val, pastey::paste! { &mut self.[<$name:snake>] })
                }
              )+
              _ => return false
            };

            // Unparsed properties are always "valid", meaning they always override previous declarations
            // that cover the same vendor prefixes. A declaration with other prefixes (e.g. the standard
            // one followed by a `-webkit-` one) must be kept.
            context.add_unparsed_fallbacks(&mut unparsed);
            let mut push = true;
            if let Some(i) = *index {
              let cur_prefix = dest[i].property_id().prefix();
              let new_prefix = unparsed.property_id.prefix();
              if new_prefix.contains(cur_prefix) {
                dest[i] = Property::Unparsed(unparsed.clone());
                push = false;
              } else if let Property::Unparsed(cur) = &mut dest[i] {
                if cur.value == unparsed.value {
                  cur.property_id = cur.property_id.with_prefix(cur_prefix | new_prefix);
                  push = false;
                }
              }
            }
            if push {
              *index = Some(dest.len());
              dest.push(Property::Unparsed(unparsed));
            }
          }
          _ => return false
        }

        true
      }

      fn finalize(&mut self, _: &mut DeclarationList, _: &mut PropertyHandlerContext) {
        $(
          pastey::paste! { self.[<$name:snake>] = None };
        )+
      }
    }
  };
}

define_fallbacks! {
  Color,
  TextShadow,
  Filter(prefix),
  BackdropFilter(prefix),
  Fill,
  Stroke,
  CaretColor,
  Caret,
}
