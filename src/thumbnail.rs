use std::cell::RefCell;
use std::collections::HashMap;
use std::fs;
use std::os::unix::fs::DirBuilderExt;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::UNIX_EPOCH;

use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};

struct Held {
    texture: gdk::Texture,
    bytes: usize,
    // The value of `Kept::uses` when this texture was last asked for.
    last_use: u64,
}

struct Kept {
    textures: HashMap<PathBuf, Held>,
    bytes: usize,
    uses: u64,
}

// Holds this session's thumbnails in memory. Clones share one store.
#[derive(Clone)]
pub struct Store {
    kept: Rc<RefCell<Kept>>,
    decoders: Rc<glib::ThreadPool>,
}

pub struct Request {
    wanted: Arc<AtomicBool>,
}

impl Drop for Request {
    fn drop(&mut self) {
        self.wanted.store(false, Ordering::Relaxed);
    }
}

impl Store {
    pub fn new() -> Store {
        // Half the processors decode. The rest stay free for the interface and the map.
        // GLib's own task threads are not used, because map tiles read from disk on them and would queue behind every photo.
        let processors = thread::available_parallelism().map_or(1, |count| count.get());
        let decoders = glib::ThreadPool::shared(Some((processors as u32 / 2).max(1)));

        Store {
            decoders: Rc::new(decoders.expect("a thread pool")),
            kept: Rc::new(RefCell::new(Kept {
                textures: HashMap::new(),
                bytes: 0,
                uses: 0,
            })),
        }
    }

    // Dropping the returned request cancels it, and `show` is then never called.
    pub fn request(&self, path: PathBuf, show: impl FnOnce(&gdk::Texture) + 'static) -> Request {
        let wanted = Arc::new(AtomicBool::new(true));

        let mut kept = self.kept.borrow_mut();
        kept.uses += 1;
        let uses = kept.uses;
        let held = kept.textures.get_mut(&path).map(|held| {
            held.last_use = uses;
            held.texture.clone()
        });
        drop(kept);

        if let Some(texture) = held {
            show(&texture);
            return Request { wanted };
        }

        let wanted_before_decode = wanted.clone();
        let wanted_after_decode = wanted.clone();
        let kept = self.kept.clone();
        let decoders = self.decoders.clone();
        let decoded_path = path.clone();

        glib::spawn_future_local(async move {
            let decoding = decoders.push_future(move || {
                if !wanted_before_decode.load(Ordering::Relaxed) {
                    return None;
                }

                // The freedesktop thumbnail standard names the saved copy after the MD5 of the file's address.
                let address = gio::File::for_path(&path).uri();
                let name = glib::compute_checksum_for_string(glib::ChecksumType::Md5, &address)?;
                let directory = glib::user_cache_dir().join("thumbnails/x-large");
                let saved_at = directory.join(format!("{name}.png"));

                let modified = fs::metadata(&path).ok()?.modified().ok()?;
                let modified = modified.duration_since(UNIX_EPOCH).ok()?.as_secs();
                let modified = modified.to_string();

                let saved = Pixbuf::from_file(&saved_at).ok().filter(|saved| {
                    saved.option("tEXt::Thumb::MTime").as_deref() == Some(&modified)
                });

                let thumbnail = match saved {
                    Some(saved) => saved,
                    None => {
                        let (_, width, height) = Pixbuf::file_info(&path)?;

                        // The standard's "x-large" size fits in 512 x 512 px and is never enlarged.
                        let scaled = Pixbuf::from_file_at_scale(
                            &path,
                            width.min(512),
                            height.min(512),
                            true,
                        );
                        let upright = scaled.ok()?.apply_embedded_orientation()?;

                        let png = upright.save_to_bufferv(
                            "png",
                            &[
                                ("tEXt::Thumb::URI", address.as_str()),
                                ("tEXt::Thumb::MTime", modified.as_str()),
                            ],
                        );

                        // A thumbnail that cannot be saved is still shown, and decodes again next launch.
                        if let Ok(png) = png {
                            // The standard keeps thumbnails private to the user.
                            let _ = fs::DirBuilder::new()
                                .recursive(true)
                                .mode(0o700)
                                .create(&directory);
                            let _ = glib::file_set_contents_full(
                                &saved_at,
                                &png,
                                glib::FileSetContentsFlags::CONSISTENT,
                                0o600,
                            );
                        }

                        upright
                    }
                };

                let side = thumbnail.width().min(thumbnail.height());
                let square =
                    Pixbuf::new(thumbnail.colorspace(), thumbnail.has_alpha(), 8, side, side)?;
                thumbnail.copy_area(
                    (thumbnail.width() - side) / 2,
                    (thumbnail.height() - side) / 2,
                    side,
                    side,
                    &square,
                    0,
                    0,
                );

                Some((
                    square.read_pixel_bytes(),
                    side,
                    square.rowstride(),
                    square.has_alpha(),
                ))
            });

            let Ok(decoding) = decoding else {
                return;
            };

            let Ok(Some((pixels, side, stride, has_alpha))) = decoding.await else {
                return;
            };

            let format = if has_alpha {
                gdk::MemoryFormat::R8g8b8a8
            } else {
                gdk::MemoryFormat::R8g8b8
            };

            let texture = gdk::MemoryTexture::new(side, side, format, &pixels, stride as usize);
            let texture: gdk::Texture = texture.upcast();

            let mut kept = kept.borrow_mut();
            kept.uses += 1;
            let held = Held {
                texture: texture.clone(),
                bytes: pixels.len(),
                last_use: kept.uses,
            };
            kept.bytes += held.bytes;
            if let Some(replaced) = kept.textures.insert(decoded_path, held) {
                kept.bytes -= replaced.bytes;
            }

            // Past 256 MB, the textures unused for longest make room.
            while kept.bytes > 256 << 20 {
                let oldest = kept
                    .textures
                    .iter()
                    .min_by_key(|(_, held)| held.last_use)
                    .map(|(path, _)| path.clone());
                let Some(oldest) = oldest else {
                    break;
                };

                if let Some(dropped) = kept.textures.remove(&oldest) {
                    kept.bytes -= dropped.bytes;
                }
            }
            drop(kept);

            if !wanted_after_decode.load(Ordering::Relaxed) {
                return;
            }

            show(&texture);
        });

        Request { wanted }
    }
}
