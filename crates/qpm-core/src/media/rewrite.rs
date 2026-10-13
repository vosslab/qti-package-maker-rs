use std::collections::BTreeMap;

use lol_html::{RewriteStrSettings, element, rewrite_str};

use super::MediaError;
use crate::{Item, ItemRenderView};

/// Rewrites only real `<img src>` attributes, leaving every other source byte untouched.
pub fn rewrite_html_srcs<F>(html: &str, src_map: F) -> Result<String, MediaError>
where
    F: Fn(&str) -> String,
{
    // ASVS 1.2.1: lol_html owns HTML-attribute serialization, including escaping a mapped value.
    rewrite_str(
        html,
        RewriteStrSettings::new().append_element_content_handler(element!("img[src]", |element| {
            let old = element
                .get_attribute("src")
                .expect("img[src] selector guarantees src");
            element.set_attribute("src", &src_map(&old))?;
            Ok(())
        })),
    )
    .map_err(|error| MediaError::Html(error.to_string()))
}

/// Builds a non-serializable writer view with only image sources rewritten.
///
/// The source [`Item`] stays immutable and its original CRC remains attached to the returned
/// view.  This keeps writer output rewriting at the output boundary (ASVS 1.1.2) rather than
/// storing an encoded or platform-specific representation in an item bank.
pub fn rewrite_item_media<F>(item: &Item, src_map: F) -> Result<ItemRenderView, MediaError>
where
    F: Fn(&str) -> String,
{
    let mut view = ItemRenderView::from_item(item);
    view.map_html_fields(|html| rewrite_html_srcs(html, &src_map))?;
    Ok(view)
}

/// Builds a presentation view with each real image element replaced by readable text.
///
/// The callback receives the exact authored source and optional alternative text. Its returned
/// value is inserted as text, never markup, so authored attributes cannot escape the writer view.
/// This is for formats such as the human-readable report which cannot retain an image element.
pub fn replace_item_images<F>(item: &Item, describe: F) -> Result<ItemRenderView, MediaError>
where
    F: Fn(&str, Option<&str>) -> String,
{
    let mut view = ItemRenderView::from_item(item);
    view.map_html_fields(|html| {
        rewrite_str(
            html,
            RewriteStrSettings::new().append_element_content_handler(element!(
                "img[src]",
                |element| {
                    let src = element
                        .get_attribute("src")
                        .expect("img[src] selector guarantees src");
                    let alt = element.get_attribute("alt");
                    element.replace(
                        &describe(&src, alt.as_deref()),
                        lol_html::html_content::ContentType::Text,
                    );
                    Ok(())
                }
            )),
        )
        .map_err(|error| MediaError::Html(error.to_string()))
    })?;
    Ok(view)
}

/// Rewrites a scalar/list/map field recursively while retaining its structure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FieldValue {
    String(String),
    List(Vec<FieldValue>),
    Map(BTreeMap<String, FieldValue>),
    Other,
}

pub fn rewrite_field_value<F>(value: &FieldValue, src_map: &F) -> Result<FieldValue, MediaError>
where
    F: Fn(&str) -> String,
{
    match value {
        FieldValue::String(value) => rewrite_html_srcs(value, src_map).map(FieldValue::String),
        FieldValue::List(values) => values
            .iter()
            .map(|value| rewrite_field_value(value, src_map))
            .collect::<Result<Vec<_>, _>>()
            .map(FieldValue::List),
        FieldValue::Map(values) => values
            .iter()
            .map(|(key, value)| Ok((key.clone(), rewrite_field_value(value, src_map)?)))
            .collect::<Result<BTreeMap<_, _>, MediaError>>()
            .map(FieldValue::Map),
        FieldValue::Other => Ok(FieldValue::Other),
    }
}

#[cfg(test)]
mod tests {
    use super::{replace_item_images, rewrite_html_srcs, rewrite_item_media};
    use crate::{Item, ItemBody};

    fn replace_old(src: &str) -> String {
        src.replace("old", "new")
    }

    #[test]
    fn rewrites_mixed_quotes_without_touching_neighbors() {
        let input = "before <IMG alt='x' src='old.png'> &amp; after";
        assert_eq!(
            rewrite_html_srcs(input, replace_old).expect("rewrite"),
            "before <IMG alt='x' src=\"new.png\"> &amp; after"
        );
    }

    #[test]
    fn excludes_data_src_and_lazy_src() {
        let input = "<img data-src='old.png' lazy-src='old2.png' src='old3.png'>";
        assert_eq!(
            rewrite_html_srcs(input, replace_old).expect("rewrite"),
            "<img data-src='old.png' lazy-src='old2.png' src=\"new3.png\">"
        );
    }

    #[test]
    fn excludes_img_shaped_script_text() {
        let input = "<script>const x = '<img src=\"old.png\">';</script><img src='old2.png'>";
        assert_eq!(
            rewrite_html_srcs(input, replace_old).expect("rewrite"),
            "<script>const x = '<img src=\"old.png\">';</script><img src=\"new2.png\">"
        );
    }

    #[test]
    fn preserves_unclosed_img_element_around_changed_src() {
        let input = "lead <img src='old.png' alt='unfinished'>";
        assert_eq!(
            rewrite_html_srcs(input, replace_old).expect("rewrite"),
            "lead <img src=\"new.png\" alt='unfinished'>"
        );
    }

    #[test]
    fn rewrites_multiple_images_in_document_order() {
        let input = "<img src='old-a.png'><td>x</td><img src=\"old-b.png?x=1\">";
        assert_eq!(
            rewrite_html_srcs(input, replace_old).expect("rewrite"),
            "<img src=\"new-a.png\"><td>x</td><img src=\"new-b.png?x=1\">"
        );
    }

    #[test]
    fn item_rewrite_returns_a_view_and_preserves_the_source_item() {
        let item = Item::new(
            "<img src='old.png'/>".to_owned(),
            ItemBody::Mc {
                choices: vec!["one".to_owned(), "two".to_owned()],
                answer: "one".to_owned(),
            },
        )
        .expect("valid item");
        let view = rewrite_item_media(&item, replace_old).expect("render view");
        assert_eq!(item.common().question_text, "<img src='old.png'/>");
        assert_eq!(view.common().question_text, "<img src=\"new.png\" />");
        assert_eq!(view.crc(), item.crc());
        assert_eq!(view.common().item_number, item.common().item_number);
    }

    #[test]
    fn image_replacement_reads_alt_as_data_and_keeps_source_item_immutable() {
        let item = Item::new(
            "Look <img src='figure.png' alt='a &amp; b'/>".to_owned(),
            ItemBody::Fib {
                answers: vec!["yes".to_owned()],
            },
        )
        .expect("valid item");
        let view = replace_item_images(&item, |src, alt| {
            format!("[{src}; {}]", alt.unwrap_or_default())
        })
        .expect("presentation view");
        assert_eq!(
            view.common().question_text,
            "Look [figure.png; a &amp;amp; b]"
        );
        assert!(item.common().question_text.contains("<img"));
        assert_eq!(view.crc(), item.crc());
    }
}
