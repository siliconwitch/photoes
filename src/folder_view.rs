use std::cell::{Cell, RefCell};
use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::rc::Rc;

use gtk::prelude::*;
use gtk::{glib, pango};

use crate::library::Photo;
use crate::thumbnail;

// Icon sizes in pixels: small, medium and large list, then small, medium and large icons.
const ICON_SIZES: [i32; 6] = [24, 40, 64, 96, 144, 224];
const FIRST_ICON_LEVEL: usize = 3;

struct Shown {
    photos: Vec<PathBuf>,
    roots: Vec<PathBuf>,
    directory: Option<PathBuf>,
}

pub struct FolderView {
    pub widget: gtk::ScrolledWindow,
    shown: Rc<RefCell<Shown>>,
    list: Rc<dyn Fn()>,
}

impl FolderView {
    pub fn show(&self, photos: &[Photo]) {
        let roots: BTreeSet<PathBuf> = photos.iter().map(|photo| photo.root.clone()).collect();

        *self.shown.borrow_mut() = Shown {
            photos: photos.iter().map(|photo| photo.path.clone()).collect(),
            roots: roots.into_iter().collect(),
            directory: None,
        };

        (self.list)();
    }
}

pub fn new() -> FolderView {
    let shown = Rc::new(RefCell::new(Shown {
        photos: Vec::new(),
        roots: Vec::new(),
        directory: None,
    }));

    let entries = gtk::StringList::new(&[]);

    let list: Rc<dyn Fn()> = Rc::new({
        let shown = shown.clone();
        let entries = entries.clone();
        move || {
            let shown = shown.borrow();

            let mut listing = Vec::new();

            let Some(directory) = &shown.directory else {
                for root in &shown.roots {
                    listing.push(root.to_string_lossy());
                }
                let listing: Vec<&str> = listing.iter().map(|path| path.as_ref()).collect();
                entries.splice(0, entries.n_items(), &listing);
                return;
            };

            let mut folders = BTreeSet::new();
            let mut photos = BTreeSet::new();
            for photo in &shown.photos {
                let Ok(below) = photo.strip_prefix(directory) else {
                    continue;
                };

                let mut components = below.components();
                let Some(first) = components.next() else {
                    continue;
                };

                if components.next().is_some() {
                    folders.insert(directory.join(first));
                } else {
                    photos.insert(photo.clone());
                }
            }

            listing.push("..".into());
            for path in folders.iter().chain(photos.iter()) {
                listing.push(path.to_string_lossy());
            }
            let listing: Vec<&str> = listing.iter().map(|path| path.as_ref()).collect();
            entries.splice(0, entries.n_items(), &listing);
        }
    });

    let level = Rc::new(Cell::new(FIRST_ICON_LEVEL));
    let requests = Rc::new(RefCell::new(HashMap::new()));

    let rows = gtk::SignalListItemFactory::new();

    rows.connect_setup({
        let level = level.clone();
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");

            let icon = gtk::Image::builder()
                .pixel_size(ICON_SIZES[level.get()])
                .build();
            let name = gtk::Label::builder()
                .ellipsize(pango::EllipsizeMode::Middle)
                .build();

            let row = if level.get() < FIRST_ICON_LEVEL {
                name.set_xalign(0.0);
                gtk::Box::new(gtk::Orientation::Horizontal, 12)
            } else {
                gtk::Box::new(gtk::Orientation::Vertical, 6)
            };

            row.append(&icon);
            row.append(&name);
            item.set_child(Some(&row));
        }
    });

    rows.connect_bind({
        let requests = requests.clone();
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
            let row = item.child().expect("a row");
            let icon = row
                .first_child()
                .and_downcast::<gtk::Image>()
                .expect("an icon");
            let name = row
                .last_child()
                .and_downcast::<gtk::Label>()
                .expect("a name");
            let entry = item
                .item()
                .and_downcast::<gtk::StringObject>()
                .expect("an entry");
            let entry = entry.string();

            let path = Path::new(entry.as_str());
            if entry == ".." || path.is_dir() {
                let folder_name = path.file_name().unwrap_or(path.as_os_str());
                name.set_label(&folder_name.to_string_lossy());
                icon.set_icon_name(Some("folder"));
                return;
            }

            name.set_label(&path.file_name().unwrap_or_default().to_string_lossy());
            icon.set_icon_name(Some("image-x-generic-symbolic"));

            let request = thumbnail::request(path.to_path_buf(), move |texture| {
                icon.set_paintable(Some(texture));
            });
            requests.borrow_mut().insert(item.clone(), request);
        }
    });

    rows.connect_unbind({
        let requests = requests.clone();
        move |_, item| {
            let item = item.downcast_ref::<gtk::ListItem>().expect("a list item");
            requests.borrow_mut().remove(item);
        }
    });

    let grid = gtk::GridView::builder()
        .model(&gtk::NoSelection::new(Some(entries.clone())))
        .factory(&rows)
        .single_click_activate(true)
        .css_classes(["entries"])
        .build();

    grid.connect_activate({
        let shown = shown.clone();
        let list = list.clone();
        move |_, position| {
            let Some(entry) = entries.string(position) else {
                return;
            };

            let mut shown = shown.borrow_mut();
            if entry == ".." {
                let Some(directory) = shown.directory.take() else {
                    return;
                };

                if !shown.roots.contains(&directory) {
                    shown.directory = directory.parent().map(Path::to_path_buf);
                }
            } else if Path::new(entry.as_str()).is_dir() {
                shown.directory = Some(PathBuf::from(entry.as_str()));
            } else {
                return;
            }
            drop(shown);

            list();
        }
    });

    let widget = gtk::ScrolledWindow::builder()
        .hscrollbar_policy(gtk::PolicyType::Never)
        .child(&grid)
        .build();

    let arrange = Rc::new({
        let grid = grid.clone();
        let widget = widget.clone();
        let level = level.clone();
        move || {
            // A cell is its icon plus 20 px of padding and margin. 36 leaves a gap between icons.
            let columns = if level.get() < FIRST_ICON_LEVEL {
                1
            } else {
                (widget.width() / (ICON_SIZES[level.get()] + 36)).max(1) as u32
            };

            // The minimum stays at one column so the window can always shrink.
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

    let level_at_start = Rc::new(Cell::new(level.get()));

    pinch.connect_begin({
        let level = level.clone();
        let level_at_start = level_at_start.clone();
        move |_, _| level_at_start.set(level.get())
    });

    pinch.connect_scale_changed(move |_, scale| {
        // Doubling the finger distance moves three levels.
        let pinched = level_at_start.get() as f64 + scale.log2() * 3.0;
        let pinched = pinched.round().clamp(0.0, 5.0) as usize;
        if pinched == level.get() {
            return;
        }

        level.set(pinched);

        // Resetting the factory rebuilds every row at the new level.
        grid.set_factory(None::<&gtk::SignalListItemFactory>);
        grid.set_factory(Some(&rows));
        arrange();
    });

    widget.add_controller(pinch);

    FolderView {
        widget,
        shown,
        list,
    }
}
