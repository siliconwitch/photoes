use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;

use gtk::glib;
use gtk::prelude::*;
use shumate::prelude::*;

use crate::library::Photo;
use crate::map_source;
use crate::thumbnail;

struct Located {
    path: PathBuf,
    latitude: f64,
    longitude: f64,
    // Web Mercator position, from 0 at the west and north edges to 1 at the east and south.
    x: f64,
    y: f64,
}

struct Pin {
    marker: shumate::Marker,
    count: gtk::Label,
    _thumbnail: thumbnail::Request,
}

pub struct LocationView {
    pub widget: shumate::SimpleMap,
    located: Rc<RefCell<Vec<Located>>>,
    pin: Rc<dyn Fn()>,
}

impl LocationView {
    pub fn show(&self, photos: &[Photo]) {
        *self.located.borrow_mut() = photos
            .iter()
            .filter_map(|photo| {
                let (latitude, longitude) = photo.location?;
                let (x, y) = web_mercator(latitude, longitude);

                Some(Located {
                    path: photo.path.clone(),
                    latitude,
                    longitude,
                    x,
                    y,
                })
            })
            .collect();

        (self.pin)();
    }
}

fn web_mercator(latitude: f64, longitude: f64) -> (f64, f64) {
    let x = (longitude + 180.0) / 360.0;
    let y = (1.0 - latitude.to_radians().tan().asinh() / PI) / 2.0;

    (x, y)
}

pub fn new() -> LocationView {
    let widget = shumate::SimpleMap::new();
    widget.set_show_zoom_buttons(false);

    let scale = widget.scale().expect("a scale");
    scale.set_visible(false);

    // Vector tiles are 512 px. Below zoom 1 the world is smaller than the window and repeats as a grid of maps.
    let viewport = widget.viewport().expect("a viewport");
    viewport.set_min_zoom_level(1);
    viewport.set_zoom_level(2.0);

    // The map only pans and zooms. libshumate has no switch for its rotate gesture.
    let map = widget.map().expect("a map");
    let controllers = map.observe_controllers();
    for index in 0..controllers.n_items() {
        let rotate = controllers.item(index).and_downcast::<gtk::GestureRotate>();
        if let Some(rotate) = rotate {
            rotate.set_propagation_phase(gtk::PropagationPhase::None);
        }
    }

    let layer = shumate::MarkerLayer::new(&viewport);
    widget.add_overlay_layer(&layer);

    let located: Rc<RefCell<Vec<Located>>> = Rc::new(RefCell::new(Vec::new()));
    let pins: RefCell<HashMap<PathBuf, Pin>> = RefCell::new(HashMap::new());

    let pin: Rc<dyn Fn()> = Rc::new({
        let widget = widget.clone();
        let viewport = viewport.clone();
        let located = located.clone();
        move || {
            // A hidden map decodes no thumbnails.
            if !widget.is_mapped() {
                return;
            }

            // libshumate cannot place a marker before the map has its tiles.
            let Some(tiles) = viewport.reference_map_source() else {
                return;
            };

            let located = located.borrow();
            let mut pins = pins.borrow_mut();

            // Photos in the same 96 px square of the map share one pin, which shows the first of them.
            let world = f64::from(tiles.tile_size()) * 2f64.powf(viewport.zoom_level().floor());
            let (centre_x, centre_y) = web_mercator(viewport.latitude(), viewport.longitude());

            let mut clusters: HashMap<(i64, i64), (&Located, u32)> = HashMap::new();
            for photo in located.iter() {
                // 2048 px either side of the centre covers a 4K window.
                let off_screen = ((photo.x - centre_x) * world).abs() > 2048.0
                    || ((photo.y - centre_y) * world).abs() > 2048.0;
                if off_screen {
                    continue;
                }

                let square = (
                    (photo.x * world / 96.0) as i64,
                    (photo.y * world / 96.0) as i64,
                );
                clusters.entry(square).or_insert((photo, 0)).1 += 1;
            }

            let shown: HashMap<&PathBuf, u32> = clusters
                .values()
                .map(|(photo, count)| (&photo.path, *count))
                .collect();

            pins.retain(|path, pin| {
                if !shown.contains_key(path) {
                    layer.remove_marker(&pin.marker);
                }

                shown.contains_key(path)
            });

            for (photo, count) in clusters.values() {
                let pin = pins.entry(photo.path.clone()).or_insert_with(|| {
                    let picture = gtk::Image::builder()
                        .pixel_size(56)
                        .overflow(gtk::Overflow::Hidden)
                        .css_classes(["pin"])
                        .build();
                    let count = gtk::Label::builder()
                        .halign(gtk::Align::End)
                        .valign(gtk::Align::Start)
                        .css_classes(["count"])
                        .build();

                    let face = gtk::Overlay::builder().child(&picture).build();
                    face.add_overlay(&count);

                    let marker = shumate::Marker::new();
                    marker.set_child(Some(&face));
                    marker.set_location(photo.latitude, photo.longitude);
                    layer.add_marker(&marker);

                    let thumbnail = thumbnail::request(photo.path.clone(), move |texture| {
                        picture.set_paintable(Some(texture));
                    });

                    Pin {
                        marker,
                        count,
                        _thumbnail: thumbnail,
                    }
                });

                pin.count.set_label(&count.to_string());
                pin.count.set_visible(*count > 1);
            }
        }
    });

    viewport.connect_zoom_level_notify({
        let pin = pin.clone();
        move |_| pin()
    });
    viewport.connect_latitude_notify({
        let pin = pin.clone();
        move |_| pin()
    });
    viewport.connect_longitude_notify({
        let pin = pin.clone();
        move |_| pin()
    });
    // Opening the Location tab is the user action that allows the tile download.
    let loading = Rc::new(Cell::new(false));
    widget.connect_map({
        let pin = pin.clone();
        move |widget| {
            pin();

            if widget.map_source().is_some() || loading.get() {
                return;
            }

            loading.set(true);

            let widget = widget.clone();
            let loading = loading.clone();
            let pin = pin.clone();
            glib::spawn_future_local(async move {
                let tiles = map_source::load().await;
                loading.set(false);

                match tiles {
                    Ok(tiles) => {
                        widget.set_map_source(Some(&tiles));
                        pin();
                    }
                    Err(error) => eprintln!("Map tiles did not load: {error}"),
                }
            });
        }
    });

    LocationView {
        widget,
        located,
        pin,
    }
}
