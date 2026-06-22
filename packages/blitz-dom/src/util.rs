use crate::node::{Node, NodeData};
use color::{AlphaColor, Srgb};
use std::borrow::Cow;
use style::color::AbsoluteColor;

pub type Color = AlphaColor<Srgb>;

/// Decode raw font bytes, decompressing WOFF/WOFF2 if the `woff` feature is enabled.
/// Returns the original slice unchanged for TTF/OTF input, and also on decompression
/// failure. With the `woff` feature disabled, all input passes through unchanged.
pub fn decode_font_bytes(bytes: &[u8]) -> Cow<'_, [u8]> {
    if bytes.len() < 4 {
        return Cow::Borrowed(bytes);
    }
    match &bytes[0..4] {
        #[cfg(feature = "woff")]
        b"wOFF" => wuff::decompress_woff1(bytes)
            .map(Cow::Owned)
            .unwrap_or_else(|_| {
                #[cfg(feature = "tracing")]
                tracing::warn!("Failed to decompress woff1 font");
                Cow::Borrowed(bytes)
            }),
        #[cfg(feature = "woff")]
        b"wOF2" => wuff::decompress_woff2(bytes)
            .map(Cow::Owned)
            .unwrap_or_else(|_| {
                #[cfg(feature = "tracing")]
                tracing::warn!("Failed to decompress woff2 font");
                Cow::Borrowed(bytes)
            }),
        _ => Cow::Borrowed(bytes),
    }
}

#[cfg(feature = "svg")]
use std::sync::{Arc, LazyLock};
#[cfg(feature = "svg")]
use usvg::fontdb;
#[cfg(feature = "svg")]
pub(crate) static FONT_DB: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    Arc::new(db)
});

/// Which kind of CSS image layer list (`background-image` or `mask-image`) to
/// flush from style to dedicated storage on the node.
#[derive(Clone, Copy, Debug)]
pub enum ImageLayerKind {
    Background,
    Mask,
}

impl ImageLayerKind {
    pub fn image_type(self, idx: usize) -> ImageType {
        match self {
            Self::Background => ImageType::Background(idx),
            Self::Mask => ImageType::Mask(idx),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub enum ImageType {
    Image,
    Background(usize),
    Mask(usize),
}

/// A point
#[derive(Clone, Debug, Copy, Eq, PartialEq)]
pub struct Point<T> {
    /// The x coordinate
    pub x: T,
    /// The y coordinate
    pub y: T,
}

impl Point<f64> {
    pub const ZERO: Self = Point { x: 0.0, y: 0.0 };
}

// Debug print an RcDom
pub fn walk_tree(indent: usize, node: &Node) {
    // Skip all-whitespace text nodes entirely
    if let NodeData::Text(data) = &node.data {
        if data.content.chars().all(|c| c.is_ascii_whitespace()) {
            return;
        }
    }

    print!("{}", " ".repeat(indent));
    let id = node.id;
    match &node.data {
        NodeData::Document => println!("#Document {id}"),

        NodeData::Text(data) => {
            if data.content.chars().all(|c| c.is_ascii_whitespace()) {
                println!("{id} #text: <whitespace>");
            } else {
                let content = data.content.trim();
                if content.len() > 10 {
                    println!(
                        "#text {id}: {}...",
                        content
                            .split_at(content.char_indices().take(10).last().unwrap().0)
                            .0
                            .escape_default()
                    )
                } else {
                    println!("#text {id}: {}", data.content.trim().escape_default())
                }
            }
        }

        NodeData::Comment => println!("<!-- COMMENT {id} -->"),

        NodeData::AnonymousBlock(_) => println!("{id} AnonymousBlock"),

        NodeData::Element(data) => {
            print!("<{} {id}", data.name.local);
            for attr in data.attrs.iter() {
                print!(" {}=\"{}\"", attr.name.local, attr.value);
            }
            if !node.children.is_empty() {
                println!(">");
            } else {
                println!("/>");
            }
        } // NodeData::Doctype {
          //     ref name,
          //     ref public_id,
          //     ref system_id,
          // } => println!("<!DOCTYPE {} \"{}\" \"{}\">", name, public_id, system_id),
          // NodeData::ProcessingInstruction { .. } => unreachable!(),
    }

    if !node.children.is_empty() {
        for child_id in node.children.iter() {
            walk_tree(indent + 2, node.with(*child_id));
        }

        if let NodeData::Element(data) = &node.data {
            println!("{}</{}>", " ".repeat(indent), data.name.local);
        }
    }
}

/// Parse an SVG document, leaving `currentColor` at its usvg default (black).
///
/// Use this when no element context is available (intrinsic sizing, or inline
/// `<svg>` which already substitutes `currentColor` during DOM serialization —
/// see `Node::write_outer_html`).
#[cfg(feature = "svg")]
pub fn parse_svg(source: &[u8]) -> Result<usvg::Tree, usvg::Error> {
    parse_svg_inner(source, None)
}

/// Parse an SVG document, resolving `currentColor` against `current_color`.
///
/// usvg 0.46 exposes no direct "current color" option. It resolves
/// `currentColor` from the nearest ancestor's `color` presentation attribute
/// (defaulting to black), so we inject `svg { color: ... }` through usvg's
/// stylesheet hook. usvg applies that as the root's `color` attribute, and
/// because `color` is inheritable in SVG, every `currentColor` below it
/// resolves to the element's computed CSS `color` — matching how browsers tint
/// `currentColor` icons.
#[cfg(feature = "svg")]
pub fn parse_svg_with_current_color(
    source: &[u8],
    current_color: AbsoluteColor,
) -> Result<usvg::Tree, usvg::Error> {
    parse_svg_inner(source, Some(current_color))
}

#[cfg(feature = "svg")]
fn parse_svg_inner(
    source: &[u8],
    current_color: Option<AbsoluteColor>,
) -> Result<usvg::Tree, usvg::Error> {
    let style_sheet = current_color.map(|color| {
        // Convert to sRGB and clamp: wide-gamut colors can map to out-of-[0,1]
        // components, and we want a plain, parseable `rgba()` for usvg.
        let [r, g, b, a] = color.as_color_color().components;
        let channel = |c: f32| (c.clamp(0.0, 1.0) * 255.0).round() as u8;
        format!(
            "svg{{color:rgba({},{},{},{})}}",
            channel(r),
            channel(g),
            channel(b),
            a.clamp(0.0, 1.0),
        )
    });

    let options = usvg::Options {
        fontdb: Arc::clone(&*FONT_DB),
        style_sheet,
        ..Default::default()
    };

    let tree = usvg::Tree::from_data(source, &options)?;
    Ok(tree)
}

pub trait ToColorColor {
    /// Converts a color into the `AlphaColor<Srgb>` type from the `color` crate
    fn as_color_color(&self) -> Color;
}
impl ToColorColor for AbsoluteColor {
    fn as_color_color(&self) -> Color {
        Color::new(
            *self
                .to_color_space(style::color::ColorSpace::Srgb)
                .raw_components(),
        )
    }
}

#[cfg(all(test, feature = "svg"))]
pub(crate) mod svg_tests {
    use super::{parse_svg, parse_svg_with_current_color};
    use style::color::AbsoluteColor;

    /// A filled rect using `currentColor` — the canonical icon pattern.
    const FILL_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="currentColor"/></svg>"#;
    /// A stroked path using `currentColor`, no fill.
    const STROKE_SVG: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><path d="M0 0 L10 10" fill="none" stroke="currentColor" stroke-width="2"/></svg>"#;

    fn first_path(tree: &usvg::Tree) -> &usvg::Path {
        fn find(node: &usvg::Node) -> Option<&usvg::Path> {
            match node {
                usvg::Node::Path(path) => Some(path),
                usvg::Node::Group(group) => group.children().iter().find_map(find),
                _ => None,
            }
        }
        tree.root()
            .children()
            .iter()
            .find_map(find)
            .expect("expected a path")
    }

    fn paint_color(paint: &usvg::Paint) -> usvg::Color {
        match paint {
            usvg::Paint::Color(color) => *color,
            _ => panic!("expected a solid color paint"),
        }
    }

    /// The first filled path's solid color — exposed for the `element` tests.
    pub(crate) fn first_fill_color(tree: &usvg::Tree) -> usvg::Color {
        paint_color(first_path(tree).fill().expect("expected a fill").paint())
    }

    #[test]
    fn current_color_defaults_to_black() {
        let color = first_fill_color(&parse_svg(FILL_SVG).unwrap());
        assert_eq!((color.red, color.green, color.blue), (0, 0, 0));
    }

    #[test]
    fn current_color_resolves_fill() {
        let red = AbsoluteColor::srgb_legacy(255, 0, 0, 1.0);
        let color = first_fill_color(&parse_svg_with_current_color(FILL_SVG, red).unwrap());
        assert_eq!((color.red, color.green, color.blue), (255, 0, 0));
    }

    #[test]
    fn current_color_resolves_stroke() {
        let green = AbsoluteColor::srgb_legacy(0, 128, 0, 1.0);
        let tree = parse_svg_with_current_color(STROKE_SVG, green).unwrap();
        let stroke = first_path(&tree).stroke().expect("expected a stroke");
        let color = paint_color(stroke.paint());
        assert_eq!((color.red, color.green, color.blue), (0, 128, 0));
    }

    #[test]
    fn current_color_alpha_becomes_fill_opacity() {
        // `currentColor` carries its alpha into the paint's opacity.
        let translucent = AbsoluteColor::srgb_legacy(255, 0, 0, 0.5);
        let tree = parse_svg_with_current_color(FILL_SVG, translucent).unwrap();
        let fill = first_path(&tree).fill().expect("expected a fill");
        let color = paint_color(fill.paint());
        assert_eq!((color.red, color.green, color.blue), (255, 0, 0));
        assert!((fill.opacity().get() - 0.5).abs() < 0.01);
    }
}

/// Creates an markup5ever::QualName.
/// Given a local name and an optional namespace
#[macro_export]
macro_rules! qual_name {
    ($local:tt $(, $ns:ident)?) => {
        $crate::QualName {
            prefix: None,
            ns: $crate::ns!($($ns)?),
            local: $crate::local_name!($local),
        }
    };
}
