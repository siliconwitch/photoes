use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::HashMap;
use std::path::PathBuf;
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{gdk, glib};

use crate::library::Photo;
use crate::thumbnail;

pub struct DateView {
    pub widget: gtk::ScrolledWindow,
    paths: gtk::StringList,
}

impl DateView {
    pub fn show(&self, photos: &[Photo]) {
        let mut newest_first: Vec<&Photo> = photos.iter().collect();
        newest_first.sort_by_key(|photo| Reverse(photo.modified));

        let paths: Vec<_> = newest_first
            .iter()
            .map(|photo| photo.path.to_string_lossy())
            .collect();
        let paths: Vec<&str> = paths.iter().map(|path| path.as_ref()).collect();

        self.paths.splice(0, self.paths.n_items(), &paths);
    }
}

pub fn new() -> DateView {
    let paths = gtk::StringList::new(&[]);

    // Every tile holds a square texture, so a picture as wide as its column is also as tall.
    let placeholder = gdk::MemoryTexture::new(
        1,
        1,
        gdk::MemoryFormat::R8g8b8a8,
        &glib::Bytes::from_static(&[128, 128, 128, 40]),
        4,
    );

    let requests = Rc::new(RefCell::new(HashMap::new()));

    let tiles = gtk::SignalListItemFactory::new();

    tiles.connect_setup(|_, item| {
        let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
        item.set_child(Some(&gtk::Picture::new()));
    });

    tiles.connect_bind({
        let requests = requests.clone();
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
            let picture = item
                .child()
                .and_downcast::<gtk::Picture>()
                .expect("a picture");
            let path = item
                .item()
                .and_downcast::<gtk::StringObject>()
                .expect("a path");

            picture.set_paintable(Some(&placeholder));

            let request = thumbnail::request(PathBuf::from(path.string()), move |texture| {
                picture.set_paintable(Some(texture));
            });
            requests.borrow_mut().insert(item.clone(), request);
        }
    });

    tiles.connect_unbind({
        let requests = requests.clone();
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
            requests.borrow_mut().remove(item);
        }
    });

    let grid = gtk::GridView::builder()
        .model(&gtk::NoSelection::new(Some(paths.clone())))
        .factory(&tiles)
        .css_classes(["tiles"])
        .build();

    // Rows take their natural height, which for a square picture is the column width.
    grid.set_vscroll_policy(gtk::ScrollablePolicy::Natural);

    let widget = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();

    let tile_size = Rc::new(Cell::new(160.0));

    let arrange = Rc::new({
        let grid = grid.clone();
        let widget = widget.clone();
        let tile_size = tile_size.clone();
        move || {
            let columns = (f64::from(widget.width()) / tile_size.get())
                .round()
                .max(1.0) as u32;

            // A picture can shrink to nothing, so the grid always uses the maximum.
            grid.set_max_columns(columns);
        }
    });

    // The page size is the visible width. Changing columns must wait until the layout pass ends.
    widget.hadjustment().connect_page_size_notify({
        let arrange = arrange.clone();
        move |_| {
            let arrange = arrange.clone();
            glib::idle_add_local_once(move || arrange());
        }
    });

    let pinch = gtk::GestureZoom::new();
    pinch.set_propagation_phase(gtk::PropagationPhase::Capture);

    let tile_size_at_start = Rc::new(Cell::new(tile_size.get()));

    pinch.connect_begin({
        let tile_size = tile_size.clone();
        let tile_size_at_start = tile_size_at_start.clone();
        move |_, _| tile_size_at_start.set(tile_size.get())
    });

    pinch.connect_scale_changed(move |_, scale| {
        tile_size.set((tile_size_at_start.get() * scale).clamp(48.0, 512.0));
        arrange();
    });

    widget.add_controller(pinch);

    DateView { widget, paths }
}
