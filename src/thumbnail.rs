use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{gdk, gio, glib};

pub struct Request {
    wanted: Arc<AtomicBool>,
}

impl Drop for Request {
    fn drop(&mut self) {
        self.wanted.store(false, Ordering::Relaxed);
    }
}

// Dropping the returned request cancels it, and `show` is then never called.
pub fn request(path: PathBuf, show: impl FnOnce(&gdk::Texture) + 'static) -> Request {
    let wanted = Arc::new(AtomicBool::new(true));
    let wanted_before_decode = wanted.clone();
    let wanted_after_decode = wanted.clone();

    glib::spawn_future_local(async move {
        let decoded = gio::spawn_blocking(move || {
            if !wanted_before_decode.load(Ordering::Relaxed) {
                return None;
            }

            let (_, width, height) = Pixbuf::file_info(&path)?;

            // The shorter side becomes 256 px, so the centre crop below is 256 px square.
            let scaled = if width >= height {
                Pixbuf::from_file_at_scale(&path, -1, 256, true)
            } else {
                Pixbuf::from_file_at_scale(&path, 256, -1, true)
            };

            let upright = scaled.ok()?.apply_embedded_orientation()?;

            let side = upright.width().min(upright.height());
            let square = Pixbuf::new(upright.colorspace(), upright.has_alpha(), 8, side, side)?;
            upright.copy_area(
                (upright.width() - side) / 2,
                (upright.height() - side) / 2,
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
        })
        .await;

        let Ok(Some((pixels, side, stride, has_alpha))) = decoded else {
            return;
        };

        if !wanted_after_decode.load(Ordering::Relaxed) {
            return;
        }

        let format = if has_alpha {
            gdk::MemoryFormat::R8g8b8a8
        } else {
            gdk::MemoryFormat::R8g8b8
        };

        let texture = gdk::MemoryTexture::new(side, side, format, &pixels, stride as usize);
        show(texture.upcast_ref());
    });

    Request { wanted }
}
