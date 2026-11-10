use std::collections::HashSet;
use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::time::SystemTime;

use exif::{In, Tag, Value};
use gtk::gdk_pixbuf::Pixbuf;

use crate::config::Config;

#[derive(Clone)]
pub struct Photo {
    pub path: PathBuf,
    pub root: PathBuf,
    pub modified: SystemTime,
    pub unsorted: bool,
    pub location: Option<(f64, f64)>,
}

pub fn scan(config: &Config) -> Vec<Photo> {
    let mut extensions = HashSet::new();
    for format in Pixbuf::formats() {
        for extension in format.extensions() {
            extensions.insert(extension.to_lowercase());
        }
    }

    let mut photos = Vec::new();

    let library = config.library.iter().map(|root| (root, false));
    let inbox = config.inbox.iter().map(|root| (root, true));
    for (root, unsorted) in library.chain(inbox) {
        let mut pending = vec![root.clone()];
        while let Some(directory) = pending.pop() {
            let Ok(entries) = std::fs::read_dir(&directory) else {
                continue;
            };

            for entry in entries.flatten() {
                // A "." prefix marks the hidden original of an edit.
                if entry.file_name().to_string_lossy().starts_with('.') {
                    continue;
                }

                let Ok(file_type) = entry.file_type() else {
                    continue;
                };

                let path = entry.path();
                if file_type.is_dir() {
                    pending.push(path);
                    continue;
                }

                let extension = path.extension().unwrap_or_default().to_string_lossy();
                if !file_type.is_file() || !extensions.contains(&extension.to_lowercase()) {
                    continue;
                }

                let Ok(metadata) = entry.metadata() else {
                    continue;
                };

                let Ok(modified) = metadata.modified() else {
                    continue;
                };

                let exif = File::open(&path).ok().and_then(|file| {
                    exif::Reader::new()
                        .read_from_container(&mut BufReader::new(file))
                        .ok()
                });

                // Exif stores a coordinate as degrees, minutes and seconds plus a hemisphere letter.
                let degrees = |value: Tag, hemisphere: Tag, negative: &str| -> Option<f64> {
                    let exif = exif.as_ref()?;
                    let field = exif.get_field(value, In::PRIMARY)?;

                    let Value::Rational(parts) = &field.value else {
                        return None;
                    };

                    let [whole, minutes, seconds] = parts.as_slice() else {
                        return None;
                    };

                    let degrees =
                        whole.to_f64() + minutes.to_f64() / 60.0 + seconds.to_f64() / 3600.0;
                    if !degrees.is_finite() {
                        return None;
                    }

                    let hemisphere = exif.get_field(hemisphere, In::PRIMARY)?;
                    if hemisphere.display_value().to_string() == negative {
                        return Some(-degrees);
                    }

                    Some(degrees)
                };

                let latitude = degrees(Tag::GPSLatitude, Tag::GPSLatitudeRef, "S");
                let longitude = degrees(Tag::GPSLongitude, Tag::GPSLongitudeRef, "W");

                photos.push(Photo {
                    path,
                    root: root.clone(),
                    modified,
                    unsorted,
                    location: latitude.zip(longitude),
                });
            }
        }
    }

    photos
}
