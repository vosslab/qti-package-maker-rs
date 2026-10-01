use super::{BLACK, Color};

pub(super) fn color(value: &str) -> Option<Color> {
    let value = value.trim().to_ascii_lowercase();
    if let Some(components) = value
        .strip_prefix("rgb(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let components: Vec<_> = components.split(',').map(str::trim).collect();
        let [red, green, blue] = components.as_slice() else {
            return None;
        };
        return Some(Color(
            color_channel(red)?,
            color_channel(green)?,
            color_channel(blue)?,
            255,
        ));
    }
    if let Some(components) = value
        .strip_prefix("rgba(")
        .and_then(|value| value.strip_suffix(')'))
    {
        let components: Vec<_> = components.split(',').map(str::trim).collect();
        let [red, green, blue, alpha] = components.as_slice() else {
            return None;
        };
        return Some(Color(
            color_channel(red)?,
            color_channel(green)?,
            color_channel(blue)?,
            alpha_channel(alpha)?,
        ));
    }
    let named = match value.as_str() {
        "black" => Some(BLACK),
        "blue" => Some(Color(0, 0, 255, 255)),
        "cornflowerblue" => Some(Color(100, 149, 237, 255)),
        "coral" => Some(Color(255, 127, 80, 255)),
        "crimson" => Some(Color(220, 20, 60, 255)),
        "darkgray" => Some(Color(169, 169, 169, 255)),
        "darkgreen" => Some(Color(0, 100, 0, 255)),
        "darkred" => Some(Color(139, 0, 0, 255)),
        "deepskyblue" => Some(Color(0, 191, 255, 255)),
        "gainsboro" => Some(Color(220, 220, 220, 255)),
        "gray" | "grey" => Some(Color(128, 128, 128, 255)),
        "white" => Some(Color(255, 255, 255, 255)),
        "silver" => Some(Color(192, 192, 192, 255)),
        "lightgray" => Some(Color(211, 211, 211, 255)),
        "lightsalmon" => Some(Color(255, 160, 122, 255)),
        "lightskyblue" => Some(Color(135, 206, 250, 255)),
        "red" => Some(Color(255, 0, 0, 255)),
        "salmon" => Some(Color(250, 128, 114, 255)),
        "royalblue" => Some(Color(65, 105, 225, 255)),
        "slategray" => Some(Color(112, 128, 144, 255)),
        "snow" => Some(Color(255, 250, 250, 255)),
        "yellow" => Some(Color(255, 255, 0, 255)),
        "orange" => Some(Color(255, 165, 0, 255)),
        "tomato" => Some(Color(255, 99, 71, 255)),
        "transparent" => Some(Color(0, 0, 0, 0)),
        _ => None,
    };
    if named.is_some() {
        return named;
    }
    let hex = value.strip_prefix('#')?;
    let byte = |s: &str| u8::from_str_radix(s, 16).ok();
    match hex.len() {
        3 => Some(Color(
            byte(&hex[0..1])? * 17,
            byte(&hex[1..2])? * 17,
            byte(&hex[2..3])? * 17,
            255,
        )),
        6 => Some(Color(
            byte(&hex[0..2])?,
            byte(&hex[2..4])?,
            byte(&hex[4..6])?,
            255,
        )),
        8 => Some(Color(
            byte(&hex[0..2])?,
            byte(&hex[2..4])?,
            byte(&hex[4..6])?,
            byte(&hex[6..8])?,
        )),
        _ => None,
    }
}

pub(super) fn bare_hex_color(value: &str) -> bool {
    let value = value.trim();
    value.len() == 6
        && !value.starts_with('#')
        && value.as_bytes().iter().all(u8::is_ascii_hexdigit)
}

fn color_channel(value: &str) -> Option<u8> {
    let channel = value.parse::<u16>().ok()?;
    u8::try_from(channel).ok()
}

fn alpha_channel(value: &str) -> Option<u8> {
    let alpha = value.parse::<f32>().ok()?;
    if !(0.0..=1.0).contains(&alpha) {
        return None;
    }
    Some((alpha * 255.0).round() as u8)
}
