//! Conversion functions from Stylo types to Parley types
use std::borrow::Cow;

use style::values::computed::Length;

use crate::document::{FontFaceSubset, FontFaceSubsetMap};
use crate::node::TextBrush;

// Module of type aliases so we can refer to stylo types with nicer names
pub(crate) mod stylo {
    pub(crate) use style::computed_values::text_wrap_mode::T as TextWrapMode;
    pub(crate) use style::computed_values::white_space_collapse::T as WhiteSpaceCollapse;
    pub(crate) use style::properties::ComputedValues;
    pub(crate) use style::values::computed::OverflowWrap;
    pub(crate) use style::values::computed::WordBreak;
    pub(crate) use style::values::computed::font::FontStretch;
    pub(crate) use style::values::computed::font::FontStyle;
    pub(crate) use style::values::computed::font::FontVariationSettings;
    pub(crate) use style::values::computed::font::FontWeight;
    pub(crate) use style::values::computed::font::GenericFontFamily;
    pub(crate) use style::values::computed::font::LineHeight;
    pub(crate) use style::values::computed::font::SingleFontFamily;
}

pub(crate) mod parley {
    pub(crate) use parley::FontVariation;
    pub(crate) use parley::fontique::QueryFamily;
    pub(crate) use parley::setting::*;
    pub(crate) use parley::style::*;
}

pub(crate) fn generic_font_family(input: stylo::GenericFontFamily) -> parley::GenericFamily {
    match input {
        stylo::GenericFontFamily::None => parley::GenericFamily::SansSerif,
        stylo::GenericFontFamily::Serif => parley::GenericFamily::Serif,
        stylo::GenericFontFamily::SansSerif => parley::GenericFamily::SansSerif,
        stylo::GenericFontFamily::Monospace => parley::GenericFamily::Monospace,
        stylo::GenericFontFamily::Cursive => parley::GenericFamily::Cursive,
        stylo::GenericFontFamily::Fantasy => parley::GenericFamily::Fantasy,
        stylo::GenericFontFamily::SystemUi => parley::GenericFamily::SystemUi,
    }
}

pub(crate) fn query_font_family(input: &stylo::SingleFontFamily) -> parley::QueryFamily<'_> {
    match input {
        stylo::SingleFontFamily::FamilyName(name) => {
            'ret: {
                let name = name.name.as_ref();

                // Legacy web compatibility
                #[cfg(target_vendor = "apple")]
                if name == "-apple-system" {
                    break 'ret parley::QueryFamily::Generic(parley::GenericFamily::SystemUi);
                }
                #[cfg(target_os = "macos")]
                if name == "BlinkMacSystemFont" {
                    break 'ret parley::QueryFamily::Generic(parley::GenericFamily::SystemUi);
                }

                break 'ret parley::QueryFamily::Named(name);
            }
        }
        stylo::SingleFontFamily::Generic(generic) => {
            parley::QueryFamily::Generic(self::generic_font_family(*generic))
        }
    }
}

pub(crate) fn font_weight(input: stylo::FontWeight) -> parley::FontWeight {
    parley::FontWeight::new(input.value())
}

pub(crate) fn font_width(input: stylo::FontStretch) -> parley::FontWidth {
    parley::FontWidth::from_percentage(input.0.to_float())
}

pub(crate) fn font_style(input: stylo::FontStyle) -> parley::FontStyle {
    match input {
        stylo::FontStyle::NORMAL => parley::FontStyle::Normal,
        stylo::FontStyle::ITALIC => parley::FontStyle::Italic,
        val => parley::FontStyle::Oblique(Some(val.oblique_degrees())),
    }
}

pub(crate) fn font_variations(input: &stylo::FontVariationSettings) -> Vec<parley::FontVariation> {
    input
        .0
        .iter()
        .map(|v| parley::FontVariation {
            tag: parley::Tag::from_bytes(v.tag.0.to_be_bytes()),
            value: v.value,
        })
        .collect()
}

pub(crate) fn white_space_collapse(input: stylo::WhiteSpaceCollapse) -> parley::WhiteSpaceCollapse {
    match input {
        stylo::WhiteSpaceCollapse::Collapse => parley::WhiteSpaceCollapse::Collapse,
        stylo::WhiteSpaceCollapse::Preserve => parley::WhiteSpaceCollapse::Preserve,

        // TODO: Implement PreserveBreaks and BreakSpaces modes
        stylo::WhiteSpaceCollapse::PreserveBreaks => parley::WhiteSpaceCollapse::Preserve,
        stylo::WhiteSpaceCollapse::BreakSpaces => parley::WhiteSpaceCollapse::Preserve,
    }
}

pub(crate) fn style(
    span_id: usize,
    style: &stylo::ComputedValues,
    font_face_subsets: &FontFaceSubsetMap,
) -> parley::TextStyle<'static, 'static, TextBrush> {
    let font_styles = style.get_font();
    let itext_styles = style.get_inherited_text();

    // Convert font size and line height
    let font_size = font_styles.font_size.used_size.0.px();
    let line_height = match font_styles.line_height {
        stylo::LineHeight::Normal => parley::LineHeight::FontSizeRelative(1.2),
        stylo::LineHeight::Number(num) => parley::LineHeight::FontSizeRelative(num.0),
        stylo::LineHeight::Length(value) => parley::LineHeight::Absolute(value.0.px()),
    };

    let letter_spacing = itext_styles
        .letter_spacing
        .0
        .resolve(Length::new(font_size))
        .px();

    let word_spacing = itext_styles
        .word_spacing
        .resolve(Length::new(font_size))
        .px();

    // Convert Bold/Italic
    let font_weight = self::font_weight(font_styles.font_weight);
    let font_style = self::font_style(font_styles.font_style);
    let font_width = self::font_width(font_styles.font_stretch);
    let font_variations = self::font_variations(&font_styles.font_variation_settings);

    // Convert font family
    let families: Vec<_> = font_styles
        .font_family
        .families
        .list
        .iter()
        .map(|family| match family {
            stylo::SingleFontFamily::FamilyName(name) => {
                'ret: {
                    let name = name.name.as_ref();

                    // Legacy web compatibility
                    #[cfg(target_vendor = "apple")]
                    if name == "-apple-system" {
                        break 'ret parley::FontFamilyName::Generic(
                            parley::GenericFamily::SystemUi,
                        );
                    }
                    #[cfg(target_os = "macos")]
                    if name == "BlinkMacSystemFont" {
                        break 'ret parley::FontFamilyName::Generic(
                            parley::GenericFamily::SystemUi,
                        );
                    }

                    break 'ret parley::FontFamilyName::Named(Cow::Owned(name.to_string()));
                }
            }
            stylo::SingleFontFamily::Generic(generic) => {
                parley::FontFamilyName::Generic(self::generic_font_family(*generic))
            }
        })
        .collect();

    // Splice in the synthetic per-subset families for any `@font-face` family
    // referenced here, so the shaper can pick whichever `unicode-range` subset
    // covers each character (see `expand_font_face_subsets`).
    let families =
        expand_font_face_subsets(families, font_face_subsets, font_weight.value(), font_style);

    // Wrapping and breaking
    let word_break = match itext_styles.word_break {
        stylo::WordBreak::Normal => parley::WordBreak::Normal,
        stylo::WordBreak::BreakAll => parley::WordBreak::BreakAll,
        stylo::WordBreak::KeepAll => parley::WordBreak::KeepAll,
    };
    let overflow_wrap = match itext_styles.overflow_wrap {
        stylo::OverflowWrap::Normal => parley::OverflowWrap::Normal,
        stylo::OverflowWrap::BreakWord => parley::OverflowWrap::BreakWord,
        stylo::OverflowWrap::Anywhere => parley::OverflowWrap::Anywhere,
    };
    let text_wrap_mode = match itext_styles.text_wrap_mode {
        stylo::TextWrapMode::Wrap => parley::TextWrapMode::Wrap,
        stylo::TextWrapMode::Nowrap => parley::TextWrapMode::NoWrap,
    };

    parley::TextStyle {
        // font_family: parley::FontFamily::Single(FontFamilyName::Generic(GenericFamily::SystemUi)),
        font_family: parley::FontFamily::List(Cow::Owned(families)),
        font_size,
        font_width,
        font_style,
        font_weight,
        font_variations: parley::FontVariations::List(Cow::Owned(font_variations)),
        font_features: parley::FontFeatures::List(Cow::Borrowed(&[])),
        locale: Default::default(),
        line_height,
        word_spacing,
        letter_spacing,
        text_wrap_mode,
        overflow_wrap,
        word_break,

        // Contains NodeId
        brush: TextBrush::from_id(span_id),

        // We avoid sending these styles through Parley because they don't affect layout
        // and handling them separately allows us to update them without rebuilding the Parley layout.
        //
        // Instead of setting them here we pass the NodeId in the `brush` field and use that to read these
        // styles lazily when rendering.
        has_underline: Default::default(),
        underline_offset: Default::default(),
        underline_size: Default::default(),
        underline_brush: Default::default(),
        has_strikethrough: Default::default(),
        strikethrough_offset: Default::default(),
        strikethrough_size: Default::default(),
        strikethrough_brush: Default::default(),
    }
}

/// Expand a resolved CSS font-family list so that, for every named family with
/// registered `@font-face` `unicode-range` subsets (see
/// [`BaseDocument::font_face_subsets`](crate::document::BaseDocument)), the
/// synthetic per-subset families are spliced in immediately *before* that
/// family.
///
/// parley's shaper walks the font stack per cluster and selects the first family
/// whose font covers the cluster's characters, so listing each subset as its own
/// stack entry lets it choose whichever subset's `unicode-range` covers each
/// character — instead of collapsing the whole CSS family to a single face and
/// rendering everything outside that face as `.notdef`.
fn expand_font_face_subsets(
    families: Vec<parley::FontFamilyName<'static>>,
    subsets: &FontFaceSubsetMap,
    weight: f32,
    style: parley::FontStyle,
) -> Vec<parley::FontFamilyName<'static>> {
    if subsets.is_empty() {
        return families;
    }
    let mut expanded = Vec::with_capacity(families.len());
    for family in families {
        if let parley::FontFamilyName::Named(name) = &family {
            if let Some(group) = subsets.get(&name.to_lowercase()) {
                for subset_family in select_covering_subsets(group, weight, style) {
                    expanded.push(parley::FontFamilyName::Named(Cow::Owned(
                        subset_family.to_owned(),
                    )));
                }
            }
        }
        expanded.push(family);
    }
    expanded
}

/// From one CSS family's registered subsets, pick the group to offer the shaper:
/// the subsets matching the requested slant (italic/oblique vs upright), and
/// among those the ones at the weight closest to `weight`. Every subset sharing
/// that closest weight is returned, because a single CSS family is split across
/// many disjoint-`unicode-range` files at the same weight and style.
fn select_covering_subsets(
    group: &[FontFaceSubset],
    weight: f32,
    style: parley::FontStyle,
) -> Vec<&str> {
    let want_slanted = !matches!(style, parley::FontStyle::Normal);
    let mut candidates: Vec<&FontFaceSubset> = group
        .iter()
        .filter(|s| !matches!(s.style, parley::FontStyle::Normal) == want_slanted)
        .collect();
    // If no subset matches the requested slant, offer every subset rather than
    // dropping characters to `.notdef` — parley can synthesize the slant.
    if candidates.is_empty() {
        candidates = group.iter().collect();
    }
    let best = candidates
        .iter()
        .map(|s| (s.weight - weight).abs())
        .fold(f32::INFINITY, f32::min);
    candidates
        .into_iter()
        .filter(|s| (s.weight - weight).abs() == best)
        .map(|s| s.family.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subset(family: &str, weight: f32, style: parley::FontStyle) -> FontFaceSubset {
        FontFaceSubset {
            family: family.to_string(),
            weight,
            style,
        }
    }

    fn family_names(families: &[parley::FontFamilyName<'static>]) -> Vec<String> {
        families
            .iter()
            .map(|f| match f {
                parley::FontFamilyName::Named(name) => name.to_string(),
                parley::FontFamilyName::Generic(_) => "<generic>".to_string(),
            })
            .collect()
    }

    #[test]
    fn offers_every_subset_at_the_closest_weight() {
        // Two disjoint `unicode-range` subsets at weight 400, plus a 700 set.
        let group = [
            subset("a", 400.0, parley::FontStyle::Normal),
            subset("b", 400.0, parley::FontStyle::Normal),
            subset("c", 700.0, parley::FontStyle::Normal),
        ];
        // A regular run gets both 400 subsets together, never the 700 face.
        assert_eq!(
            select_covering_subsets(&group, 400.0, parley::FontStyle::Normal),
            ["a", "b"]
        );
        // A bold run gets the 700 face.
        assert_eq!(
            select_covering_subsets(&group, 700.0, parley::FontStyle::Normal),
            ["c"]
        );
        // 500 is closer to 400 than to 700.
        assert_eq!(
            select_covering_subsets(&group, 500.0, parley::FontStyle::Normal),
            ["a", "b"]
        );
    }

    #[test]
    fn prefers_subsets_matching_the_requested_slant() {
        let group = [
            subset("up", 400.0, parley::FontStyle::Normal),
            subset("it", 400.0, parley::FontStyle::Italic),
        ];
        assert_eq!(
            select_covering_subsets(&group, 400.0, parley::FontStyle::Normal),
            ["up"]
        );
        assert_eq!(
            select_covering_subsets(&group, 400.0, parley::FontStyle::Italic),
            ["it"]
        );
        // Oblique is slanted, so it matches the italic face rather than upright.
        assert_eq!(
            select_covering_subsets(&group, 400.0, parley::FontStyle::Oblique(Some(14.0))),
            ["it"]
        );
    }

    #[test]
    fn offers_all_subsets_when_no_slant_matches() {
        let group = [
            subset("a", 400.0, parley::FontStyle::Normal),
            subset("b", 400.0, parley::FontStyle::Normal),
        ];
        // Italic requested but only upright subsets exist: offer them all and
        // let parley synthesize the slant, rather than dropping to `.notdef`.
        assert_eq!(
            select_covering_subsets(&group, 400.0, parley::FontStyle::Italic),
            ["a", "b"]
        );
    }

    #[test]
    fn splices_subset_families_ahead_of_their_css_family() {
        let mut subsets = FontFaceSubsetMap::new();
        subsets.insert(
            "inter".to_string(),
            vec![
                subset("\u{1}ff-1", 400.0, parley::FontStyle::Normal),
                subset("\u{1}ff-2", 400.0, parley::FontStyle::Normal),
            ],
        );
        // Lookup is case-insensitive ("Inter" -> "inter"); generic families and
        // families without subsets pass through untouched.
        let families = vec![
            parley::FontFamilyName::Named(Cow::Borrowed("Inter")),
            parley::FontFamilyName::Named(Cow::Borrowed("Helvetica")),
            parley::FontFamilyName::Generic(parley::GenericFamily::SansSerif),
        ];
        let out = expand_font_face_subsets(families, &subsets, 400.0, parley::FontStyle::Normal);
        assert_eq!(
            family_names(&out),
            ["\u{1}ff-1", "\u{1}ff-2", "Inter", "Helvetica", "<generic>"]
        );
    }

    #[test]
    fn expansion_is_a_no_op_without_registered_subsets() {
        let subsets = FontFaceSubsetMap::new();
        let families = vec![parley::FontFamilyName::Named(Cow::Borrowed("Inter"))];
        let out = expand_font_face_subsets(families, &subsets, 400.0, parley::FontStyle::Normal);
        assert_eq!(family_names(&out), ["Inter"]);
    }
}
