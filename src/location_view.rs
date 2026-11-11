use std::cell::{Cell, RefCell};
use std::collections::{HashMap, VecDeque};
use std::f64::consts::PI;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use gtk::prelude::*;
use gtk::{gdk, glib};
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
    pub widget: gtk::Overlay,
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

pub fn new(thumbnails: &thumbnail::Store) -> LocationView {
    let widget = shumate::SimpleMap::builder()
        .show_zoom_buttons(false)
        .css_classes(["map"])
        .build();

    let scale = widget.scale().expect("a scale");
    scale.set_visible(false);

    let viewport = widget.viewport().expect("a viewport");
    viewport.set_zoom_level(2.0);

    let map = widget.map().expect("a map");
    let controllers = map.observe_controllers();
    for index in 0..controllers.n_items() {
        let controller = controllers.item(index).expect("a controller");

        // The map only pans and zooms. libshumate has no switch for its rotate gesture.
        if let Some(rotate) = controller.downcast_ref::<gtk::GestureRotate>() {
            rotate.set_propagation_phase(gtk::PropagationPhase::None);
        }

        // libshumate glides the zoom when a scroll ends. That glide would start after every two-finger pan
        // and hold the map still against the next gesture for 200 ms.
        if let Some(scroll) = controller.downcast_ref::<gtk::EventControllerScroll>() {
            scroll.set_flags(gtk::EventControllerScrollFlags::VERTICAL);
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
        let thumbnails = thumbnails.clone();
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

                    let thumbnail = thumbnails.request(photo.path.clone(), move |texture| {
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

    // The zoom at which the world is as wide or as tall as the map, whichever comes first.
    let furthest_out: Rc<dyn Fn() -> f64> = Rc::new({
        let widget = widget.clone();
        let viewport = viewport.clone();
        move || {
            let Some(tiles) = viewport.reference_map_source() else {
                return f64::NEG_INFINITY;
            };

            let longest_side = widget.width().max(widget.height());
            (f64::from(longest_side) / f64::from(tiles.tile_size())).log2()
        }
    });

    // The view never leaves the world. Zoomed out further, or panned past a pole or the east or west edge,
    // the world repeats as a grid.
    let limit_view: Rc<dyn Fn()> = Rc::new({
        let widget = widget.clone();
        let viewport = viewport.clone();
        let map = map.clone();
        let furthest_out = furthest_out.clone();

        // Where the centre of the view last was, from 0 at the west edge of the world to 1 at the east edge.
        let last_across = Cell::new(0.5);
        move || {
            let Some(tiles) = viewport.reference_map_source() else {
                return;
            };

            if viewport.zoom_level() < furthest_out() {
                viewport.set_zoom_level(furthest_out());

                // libshumate's zoom animation would go on moving the map. It cannot be stopped from inside its own step.
                let map = map.clone();
                glib::idle_add_local_once(move || map.stop_go_to());
            }

            let zoom = viewport.zoom_level();
            let world = f64::from(tiles.tile_size()) * 2f64.powf(zoom);
            let x = tiles.x(zoom, viewport.longitude());
            let y = tiles.y(zoom, viewport.latitude());

            // The nearest the centre may come to each edge.
            let west = (f64::from(widget.width()) / 2.0).min(world / 2.0);
            let north = (f64::from(widget.height()) / 2.0).min(world / 2.0);

            // libshumate wraps a pan past the east or west edge round to the far side of the world.
            // The view stops at the edge it was nearer to before.
            let mut within_x = x;
            if x < west || x > world - west {
                within_x = if last_across.get() < 0.5 {
                    west
                } else {
                    world - west
                };
            }
            let within_y = y.clamp(north, world - north);
            last_across.set(within_x / world);

            if (within_x - x).abs() > 0.5 || (within_y - y).abs() > 0.5 {
                viewport.set_location(
                    tiles.latitude(zoom, within_y),
                    tiles.longitude(zoom, within_x),
                );
            }
        }
    });

    // Once the view has rested for a moment, the tiles a pan or a zoom would need next are downloaded into the store.
    // Opening the map is the user action that starts these downloads.
    // Every move of the view drops the tiles still waiting, because they were chosen for the view before.
    let store: Rc<RefCell<Option<map_source::TileStore>>> = Rc::new(RefCell::new(None));
    let rest: Rc<Cell<Option<glib::SourceId>>> = Rc::new(Cell::new(None));
    let ahead: Rc<RefCell<VecDeque<(i32, i32, i32)>>> = Rc::new(RefCell::new(VecDeque::new()));
    let downloads = Rc::new(Cell::new(0));

    let fetch_ahead: Rc<dyn Fn()> = Rc::new({
        let widget = widget.clone();
        let viewport = viewport.clone();
        let store = store.clone();
        move || {
            if let Some(waiting) = rest.take() {
                waiting.remove();
            }
            ahead.borrow_mut().clear();

            let widget = widget.clone();
            let viewport = viewport.clone();
            let store = store.clone();
            let rest_over = rest.clone();
            let ahead = ahead.clone();
            let downloads = downloads.clone();
            let waiting = glib::timeout_add_local_once(Duration::from_millis(300), move || {
                rest_over.take();

                let Some(store) = store.borrow().clone() else {
                    return;
                };
                let Some(tiles) = viewport.reference_map_source() else {
                    return;
                };

                if !widget.is_mapped() {
                    return;
                }

                // Half the view, as a share of the world's width.
                let world = f64::from(tiles.tile_size()) * 2f64.powf(viewport.zoom_level());
                let half_width = f64::from(widget.width()) / 2.0 / world;
                let half_height = f64::from(widget.height()) / 2.0 / world;
                let (centre_x, centre_y) = web_mercator(viewport.latitude(), viewport.longitude());

                let deepest = store.max_zoom_level() as i32;
                let level = (viewport.zoom_level() as i32).min(deepest);

                // First the view one level out, then the level in view with half a view more on every side,
                // then the view one level in. Zooming out uncovers the most unloaded map, so it goes first.
                let mut wanted = VecDeque::new();
                for (level, reach) in [(level - 1, 1.0), (level, 2.0), (level + 1, 1.0)] {
                    if level < 0 || level > deepest {
                        continue;
                    }

                    let across = 2f64.powi(level);
                    let first_column = ((centre_x - half_width * reach) * across).floor() as i32;
                    let last_column = ((centre_x + half_width * reach) * across).floor() as i32;
                    let first_row = ((centre_y - half_height * reach) * across).floor() as i32;
                    let last_row = ((centre_y + half_height * reach) * across).floor() as i32;

                    for column in first_column..=last_column {
                        for row in first_row.max(0)..=last_row.min(across as i32 - 1) {
                            let tile = (column.rem_euclid(across as i32), row, level);
                            if !wanted.contains(&tile) {
                                wanted.push_back(tile);
                            }
                        }
                    }
                }
                *ahead.borrow_mut() = wanted;

                // Six downloads run at a time, to go easy on a free tile server.
                while downloads.get() < 6 {
                    downloads.set(downloads.get() + 1);

                    let store = store.clone();
                    let ahead = ahead.clone();
                    let downloads = downloads.clone();
                    glib::spawn_future_local(async move {
                        loop {
                            let next = ahead.borrow_mut().pop_front();
                            let Some((x, y, level)) = next else {
                                break;
                            };

                            store.keep(x, y, level).await;
                        }

                        downloads.set(downloads.get() - 1);
                    });
                }
            });
            rest.set(Some(waiting));
        }
    });

    viewport.connect_zoom_level_notify({
        let pin = pin.clone();
        let limit_view = limit_view.clone();
        let fetch_ahead = fetch_ahead.clone();
        move |_| {
            limit_view();
            pin();
            fetch_ahead();
        }
    });
    viewport.connect_latitude_notify({
        let pin = pin.clone();
        let limit_view = limit_view.clone();
        let fetch_ahead = fetch_ahead.clone();
        move |_| {
            limit_view();
            pin();
            fetch_ahead();
        }
    });
    viewport.connect_longitude_notify({
        let pin = pin.clone();
        let limit_view = limit_view.clone();
        let fetch_ahead = fetch_ahead.clone();
        move |_| {
            limit_view();
            pin();
            fetch_ahead();
        }
    });

    // The map reports no size changes. A drawing area laid over it does.
    let size = gtk::DrawingArea::builder().can_target(false).build();
    size.connect_resize({
        let limit_view = limit_view.clone();
        move |_, _, _| {
            // The view cannot change during a layout pass.
            let limit_view = limit_view.clone();
            glib::idle_add_local_once(move || limit_view());
        }
    });

    let sized = gtk::Overlay::builder().child(&widget).build();
    sized.add_overlay(&size);

    let scroll = gtk::EventControllerScroll::new(gtk::EventControllerScrollFlags::BOTH_AXES);
    scroll.set_propagation_phase(gtk::PropagationPhase::Capture);
    scroll.connect_scroll({
        let viewport = viewport.clone();
        move |scroll, right, down| {
            let Some(tiles) = viewport.reference_map_source() else {
                return glib::Propagation::Proceed;
            };

            let zoom = viewport.zoom_level();

            // A mouse wheel zooms through libshumate. At the limit libshumate would still move the map.
            if scroll.unit() != gdk::ScrollUnit::Surface {
                if down > 0.0 && zoom <= furthest_out() {
                    return glib::Propagation::Stop;
                }

                return glib::Propagation::Proceed;
            }

            // Two fingers on a touchpad pan the map. libshumate alone would zoom.
            // GTK multiplies touchpad scrolling by 2.5, so the map does too and moves as far as the photo grid.
            let x = tiles.x(zoom, viewport.longitude()) + right * 2.5;
            let y = tiles.y(zoom, viewport.latitude()) + down * 2.5;
            viewport.set_location(tiles.latitude(zoom, y), tiles.longitude(zoom, x));

            glib::Propagation::Stop
        }
    });
    widget.add_controller(scroll);

    // Opening the Location tab is the user action that allows the tile download.
    let loading = Rc::new(Cell::new(false));
    widget.connect_map({
        let pin = pin.clone();
        let limit_view = limit_view.clone();
        move |widget| {
            pin();
            fetch_ahead();

            if widget.map_source().is_some() || loading.get() {
                return;
            }

            loading.set(true);

            let widget = widget.clone();
            let loading = loading.clone();
            let pin = pin.clone();
            let limit_view = limit_view.clone();
            let store = store.clone();
            let fetch_ahead = fetch_ahead.clone();
            glib::spawn_future_local(async move {
                let loaded = map_source::load(|tiles, tile_store| {
                    widget.set_map_source(Some(tiles));
                    store.replace(Some(tile_store.clone()));
                    limit_view();
                    pin();
                    fetch_ahead();

                    // libshumate can hold back a new source's first tiles until a layout in a later frame than its own retry.
                    let layer = widget.base_map_layer().expect("a map layer");
                    let frames = Cell::new(0);
                    widget.add_tick_callback(move |_, _| {
                        frames.set(frames.get() + 1);
                        if frames.get() < 2 {
                            return glib::ControlFlow::Continue;
                        }

                        layer.queue_allocate();
                        glib::ControlFlow::Break
                    });
                })
                .await;
                loading.set(false);

                if let Err(error) = loaded {
                    eprintln!("The map's tile address did not load: {error}");
                }
            });
        }
    });

    LocationView {
        widget: sized,
        located,
        pin,
    }
}
