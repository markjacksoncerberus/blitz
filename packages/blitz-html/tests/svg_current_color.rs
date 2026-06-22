//! SVG images using `fill="currentColor"` must tint to the element's computed
//! CSS `color`, not usvg's default black. Regression test for icons (Lucide,
//! Heroicons, Feather, …) rendering as black boxes.
//!
//! The `blitz-paint` dev-dependency enables the `svg` feature (and, via feature
//! unification, `blitz-dom/svg`), so the SVG paint path is always available here.

use anyrender::render_to_buffer;
use anyrender_vello_cpu::VelloCpuImageRenderer;
use blitz_dom::DocumentConfig;
use blitz_dom::node::{ImageData, SpecialElementData, SvgImageData};
use blitz_dom::util::parse_svg;
use blitz_html::{HtmlDocument, HtmlProvider};
use blitz_paint::paint_scene;
use blitz_traits::shell::{ColorScheme, Viewport};
use std::sync::Arc;

/// A 10x10 SVG that fills its whole area with `currentColor`.
const ICON: &[u8] = br#"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="currentColor"/></svg>"#;

/// Render a 100x100 `<img>` with the given inline style, injecting `ICON` as its
/// (already "loaded") SVG image, and return the center pixel. Page is white.
fn center_pixel(img_style: &str) -> [u8; 3] {
    let html = format!(
        r#"<html><body style="margin:0; background:#ffffff;">
            <img id="icon" style="{img_style}">
        </body></html>"#
    );
    let mut doc = HtmlDocument::from_html(
        &html,
        DocumentConfig {
            viewport: Some(Viewport::new(100, 100, 1.0, ColorScheme::Light)),
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            ..Default::default()
        },
    );
    doc.resolve(0.0);

    let icon_id = doc.query_selector("#icon").unwrap().expect("#icon");
    {
        let tree = Arc::new(parse_svg(ICON).unwrap());
        let svg = SvgImageData::new(tree, Arc::from(ICON));
        let node = doc.get_node_mut(icon_id).unwrap();
        node.element_data_mut().unwrap().special_data =
            SpecialElementData::Image(Box::new(ImageData::Svg(svg)));
    }
    doc.resolve(0.0);

    let buffer = render_to_buffer::<VelloCpuImageRenderer, _>(
        |scene| paint_scene(scene, doc.as_mut(), 1.0, 100, 100, 0, 0),
        100,
        100,
    );
    let idx = (50 * 100 + 50) * 4;
    [buffer[idx], buffer[idx + 1], buffer[idx + 2]]
}

#[test]
fn current_color_icon_tints_to_css_color() {
    let px = center_pixel("width:100px; height:100px; color:#00ff00;");
    assert_eq!(px, [0, 255, 0], "currentColor icon must tint to CSS color");
}

#[test]
fn current_color_icon_is_not_black() {
    // The bug: without color resolution, the icon paints solid black on a red
    // request. Confirm a red request actually renders red.
    let px = center_pixel("width:100px; height:100px; color:#ff0000;");
    assert_eq!(px, [255, 0, 0], "currentColor icon must not fall back to black");
}

#[test]
fn current_color_inherits_from_ancestor() {
    // `color` inherits, so an icon with no color of its own picks up the body's.
    let html = r#"<html><body style="margin:0; background:#ffffff; color:#0000ff;">
            <img id="icon" style="width:100px; height:100px;">
        </body></html>"#;
    let mut doc = HtmlDocument::from_html(
        html,
        DocumentConfig {
            viewport: Some(Viewport::new(100, 100, 1.0, ColorScheme::Light)),
            html_parser_provider: Some(Arc::new(HtmlProvider) as _),
            ..Default::default()
        },
    );
    doc.resolve(0.0);
    let icon_id = doc.query_selector("#icon").unwrap().expect("#icon");
    {
        let tree = Arc::new(parse_svg(ICON).unwrap());
        let svg = SvgImageData::new(tree, Arc::from(ICON));
        doc.get_node_mut(icon_id)
            .unwrap()
            .element_data_mut()
            .unwrap()
            .special_data = SpecialElementData::Image(Box::new(ImageData::Svg(svg)));
    }
    doc.resolve(0.0);
    let buffer = render_to_buffer::<VelloCpuImageRenderer, _>(
        |scene| paint_scene(scene, doc.as_mut(), 1.0, 100, 100, 0, 0),
        100,
        100,
    );
    let idx = (50 * 100 + 50) * 4;
    assert_eq!(
        [buffer[idx], buffer[idx + 1], buffer[idx + 2]],
        [0, 0, 255],
        "currentColor must resolve through inherited color"
    );
}
