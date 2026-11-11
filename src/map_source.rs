use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::error::Error;
use std::fs;
use std::future::Future;
use std::path::PathBuf;
use std::pin::Pin;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gio, glib};
use serde_json::{Value, json};
use shumate::prelude::*;
use shumate::subclass::prelude::*;
use soup::prelude::*;

mod imp {
    use super::*;

    #[derive(Default)]
    pub struct TileStore {
        // The tile address, with `{z}`, `{x}` and `{y}` where the numbers go.
        pub address: RefCell<String>,
        pub session: soup::Session,
        pub downloading: RefCell<HashSet<(i32, i32, i32)>>,
        // How many tiles libshumate is waiting for, to draw them.
        pub wanted_now: Cell<u32>,
    }

    #[glib::object_subclass]
    impl ObjectSubclass for TileStore {
        const NAME: &'static str = "PhotoesTileStore";
        type Type = super::TileStore;
        type ParentType = shumate::DataSource;
    }

    impl ObjectImpl for TileStore {}

    impl DataSourceImpl for TileStore {
        fn tile_data_future(
            &self,
            x: i32,
            y: i32,
            zoom_level: i32,
        ) -> Pin<Box<dyn Future<Output = Result<glib::Bytes, glib::Error>> + 'static>> {
            let store = self.obj().clone();

            Box::pin(async move {
                let stored = gio::File::for_path(stored_at(x, y, zoom_level));
                if let Ok((tile, _)) = stored.load_bytes_future().await {
                    return Ok(tile);
                }

                let wanted_now = &store.imp().wanted_now;
                wanted_now.set(wanted_now.get() + 1);
                let tile = store.download(x, y, zoom_level).await;
                wanted_now.set(wanted_now.get() - 1);

                tile
            })
        }

        // libshumate asks for tiles through this. The crate's default version hands libshumate no data source,
        // and libshumate then fails every tile.
        fn start_request(
            &self,
            x: i32,
            y: i32,
            zoom_level: i32,
            _cancellable: Option<gio::Cancellable>,
        ) -> Option<shumate::DataSourceRequest> {
            let request = shumate::DataSourceRequest::new(x, y, zoom_level);

            let tile = self.tile_data_future(x, y, zoom_level);
            let answered = request.clone();
            glib::spawn_future_local(async move {
                match tile.await {
                    Ok(tile) => answered.emit_data(&tile, true),
                    Err(error) => answered.emit_error(&error),
                }
            });

            Some(request)
        }
    }
}

glib::wrapper! {
    // Gives libshumate its tiles from a folder the app owns, downloading the ones the folder lacks.
    pub struct TileStore(ObjectSubclass<imp::TileStore>) @extends shumate::DataSource;
}

fn stored_at(x: i32, y: i32, zoom: i32) -> PathBuf {
    glib::user_cache_dir().join(format!("photoes/tiles/{zoom}/{x}/{y}.pbf"))
}

impl TileStore {
    // Downloads the tile unless the store holds it or is already fetching it.
    // Tiles that libshumate is waiting to draw go first, so this waits until none are left.
    pub async fn keep(&self, x: i32, y: i32, zoom: i32) {
        while self.imp().wanted_now.get() > 0 {
            glib::timeout_future(Duration::from_millis(100)).await;
        }

        if stored_at(x, y, zoom).exists() || self.imp().downloading.borrow().contains(&(x, y, zoom))
        {
            return;
        }

        let _ = self.download(x, y, zoom).await;
    }

    async fn download(&self, x: i32, y: i32, zoom: i32) -> Result<glib::Bytes, glib::Error> {
        let store = self.imp();

        let address = store
            .address
            .borrow()
            .replace("{z}", &zoom.to_string())
            .replace("{x}", &x.to_string())
            .replace("{y}", &y.to_string());
        let request = soup::Message::new("GET", &address).map_err(|error| {
            glib::Error::new(gio::IOErrorEnum::InvalidArgument, &error.to_string())
        })?;

        store.downloading.borrow_mut().insert((x, y, zoom));
        let fetched = store
            .session
            .send_and_read_future(&request, glib::Priority::DEFAULT)
            .await;
        store.downloading.borrow_mut().remove(&(x, y, zoom));

        let tile = fetched?;

        if request.status() != soup::Status::Ok {
            let answer = format!("the tile server answered {:?}", request.status());

            return Err(glib::Error::new(gio::IOErrorEnum::Failed, &answer));
        }

        // A tile that cannot be stored is still drawn, and downloads again when next wanted.
        let stored_at = stored_at(x, y, zoom);
        if let Some(directory) = stored_at.parent() {
            let _ = fs::create_dir_all(directory);
        }
        let _ = gio::File::for_path(&stored_at)
            .replace_contents_future(tile.clone(), None, false, gio::FileCreateFlags::NONE)
            .await;

        Ok(tile)
    }
}

// `show` runs at once with the tile index saved by the last launch.
// It runs again if the fetched index names a different tile address.
pub async fn load(
    show: impl Fn(&shumate::VectorRenderer, &TileStore),
) -> Result<(), Box<dyn Error>> {
    let directory = glib::user_cache_dir().join("photoes");
    let saved_at = directory.join("tile-index.json");

    // Past 2 GB, the tiles downloaded longest ago are deleted. Deleting the oldest also keeps the map from going stale.
    let stored_in = directory.join("tiles");
    gio::spawn_blocking(move || {
        let mut tiles = Vec::new();
        let mut pending = vec![stored_in];
        while let Some(folder) = pending.pop() {
            let Ok(entries) = fs::read_dir(&folder) else {
                continue;
            };

            for entry in entries.flatten() {
                let Ok(details) = entry.metadata() else {
                    continue;
                };

                if details.is_dir() {
                    pending.push(entry.path());
                } else if let Ok(downloaded) = details.modified() {
                    tiles.push((downloaded, details.len(), entry.path()));
                }
            }
        }

        let mut stored: u64 = tiles.iter().map(|(_, size, _)| size).sum();
        tiles.sort();
        for (_, size, tile) in tiles {
            if stored <= 2 << 30 {
                break;
            }

            if fs::remove_file(&tile).is_ok() {
                stored -= size;
            }
        }
    });

    let saved: Option<Value> = fs::read(&saved_at)
        .ok()
        .and_then(|index| serde_json::from_slice(&index).ok());

    let mut shown_address = None;
    if let Some(index) = &saved
        && let Ok((tiles, store)) = renderer(index)
    {
        show(&tiles, &store);
        shown_address = Some(&index["tiles"]);
    }

    // OpenFreeMap changes its tile address weekly and publishes the current one here.
    let request = soup::Message::new("GET", "https://tiles.openfreemap.org/planet")?;
    let fetched = soup::Session::new()
        .send_and_read_future(&request, glib::Priority::DEFAULT)
        .await?;
    let index: Value = serde_json::from_slice(&fetched)?;

    if shown_address == Some(&index["tiles"]) {
        return Ok(());
    }

    let (tiles, store) = renderer(&index)?;
    show(&tiles, &store);

    fs::create_dir_all(&directory)?;
    fs::write(&saved_at, &fetched)?;

    Ok(())
}

fn renderer(index: &Value) -> Result<(shumate::VectorRenderer, TileStore), Box<dyn Error>> {
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

    // Only these labels are drawn, because libshumate builds a widget for each one and the full set makes zooming stutter.
    // The style already ranks them: countries, cities, towns, main roads, streets, then water.
    layers.retain_mut(|layer| {
        if layer["type"] != "symbol" {
            return true;
        }

        let water = match layer["id"].as_str() {
            Some(
                "label_country_1" | "label_country_2" | "label_country_3" | "highway-name-major"
                | "highway-name-minor",
            ) => return true,

            // Names appear in the order someone zooming in looks for them: the country, then the city, then the town.
            // The style shows cities from zoom 3 and towns from zoom 6, while many countries are still in view.
            Some("label_city" | "label_city_capital") => {
                layer["minzoom"] = json!(7);
                return true;
            }
            Some("label_town") => {
                layer["minzoom"] = json!(9);
                return true;
            }
            Some("water_name_point_label" | "water_name_line_label") => "lake",
            Some("waterway_line_label") => "river",
            _ => return false,
        };

        // Of the water labels, lakes and rivers stay. Seas, oceans, bays, streams and canals go.
        layer["filter"] = json!(["all", layer["filter"], ["==", ["get", "class"], water]]);

        true
    });

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

    // The store replaces libshumate's own downloader, whose tile store is fixed at 100 MB and shared with other apps.
    let address = index["tiles"][0]
        .as_str()
        .ok_or("tile index has no address")?;
    let store: TileStore = glib::Object::new();
    store.imp().address.replace(address.to_string());
    store.imp().session.set_user_agent("photoes");
    store.imp().session.set_timeout(30);
    store.set_max_zoom_level(index["maxzoom"].as_u64().unwrap_or(14) as u32);
    tiles.set_data_source("openmaptiles", &store);

    Ok((tiles, store))
}
