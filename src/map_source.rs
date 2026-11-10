use gtk::glib;
use serde_json::{Value, json};
use shumate::prelude::*;
use soup::prelude::*;

pub async fn load() -> Result<shumate::VectorRenderer, Box<dyn std::error::Error>> {
    // OpenFreeMap changes its tile address weekly and publishes the current one here.
    let request = soup::Message::new("GET", "https://tiles.openfreemap.org/planet")?;
    let index = soup::Session::new()
        .send_and_read_future(&request, glib::Priority::DEFAULT)
        .await?;
    let index: Value = serde_json::from_slice(&index)?;

    let mut style: Value = serde_json::from_str(include_str!("map_style.json"))?;

    // libshumate cannot follow the style's link to the tile index, so the tile address goes in directly.
    style["sources"] = json!({
        "openmaptiles": {
            "type": "vector",
            "tiles": index["tiles"],
            "minzoom": 0,
            "maxzoom": index["maxzoom"],
        }
    });

    // Labels use the system language, then any Latin-script name, then the local name.
    let mut label = vec![json!("coalesce")];
    let languages = glib::language_names();
    let language = languages
        .iter()
        .find(|name| name.len() <= 3 && name.chars().all(|letter| letter.is_ascii_lowercase()));
    if let Some(language) = language {
        label.push(json!(["get", format!("name:{language}")]));
    }
    label.push(json!(["get", "name:latin"]));
    label.push(json!(["get", "name"]));

    let layers = style["layers"]
        .as_array_mut()
        .ok_or("style has no layers")?;
    for layer in layers {
        let Some(text) = layer.pointer_mut("/layout/text-field") else {
            continue;
        };

        if text.to_string().contains("name") {
            *text = Value::Array(label.clone());
        }
    }

    // The style writes `["linear", 1]`. libshumate accepts only `["linear"]`.
    let mut pending = vec![&mut style];
    while let Some(value) = pending.pop() {
        match value {
            Value::Array(items) => {
                if items.len() == 2 && items[0] == "linear" {
                    items.truncate(1);
                }
                pending.extend(items.iter_mut());
            }
            Value::Object(members) => pending.extend(members.values_mut()),
            _ => {}
        }
    }

    let tiles = shumate::VectorRenderer::new("openfreemap", &style.to_string())?;
    tiles.set_license("OpenFreeMap © OpenMapTiles Data from OpenStreetMap");

    Ok(tiles)
}
