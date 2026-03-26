use std::sync::LazyLock;

use regex::Regex;

use crate::output::FieldType;

static DATE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:date|signed\s+at|datum)[:_\s-]*$").unwrap()
});

static SIGNATURE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:signature|sign\s+here|sign|signez\s+ici|signer\s+ici|unterschrift|unterschreiben|unterzeichnen)[:_\s-]*$",
    )
    .unwrap()
});

static NUMBER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)(?:price|\$|€|total|quantity|prix|quantité|preis|summe|gesamt(?:betrag)?|menge|anzahl|stückzahl)[:_\s-]*$",
    )
    .unwrap()
});

/// Classify a text field's type based on the preceding text.
///
/// Only applies to fields with type == Text. Checks the preceding text against
/// regex patterns for date, signature, and number fields (in that order).
pub fn classify_field_type(preceding_text: &str, current_type: FieldType) -> FieldType {
    if current_type != FieldType::Text {
        return current_type;
    }

    if DATE_RE.is_match(preceding_text) {
        return FieldType::Date;
    }
    if SIGNATURE_RE.is_match(preceding_text) {
        return FieldType::Signature;
    }
    if NUMBER_RE.is_match(preceding_text) {
        return FieldType::Number;
    }

    FieldType::Text
}
