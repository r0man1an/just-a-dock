use gtk4::gdk;

pub fn build_icon_image(display: &gdk::Display, icon_name: Option<&str>, render_size: i32) -> gtk4::Image {
    if let Some(name) = icon_name {
        if name.starts_with('/') {
            let file = gtk4::gio::File::for_path(name);
            if let Ok(texture) = gdk::Texture::from_file(&file) {
                let image = gtk4::Image::from_paintable(Some(&texture));
                image.set_pixel_size(render_size);
                return image;
            }
        }

        let theme = gtk4::IconTheme::for_display(display);
        for candidate in icon_candidates(name) {
            if !theme.has_icon(&candidate) {
                continue;
            }
            let paintable = theme.lookup_icon(
                &candidate,
                &[],
                render_size,
                1,
                gtk4::TextDirection::None,
                gtk4::IconLookupFlags::empty(),
            );
            let image = gtk4::Image::from_paintable(Some(&paintable));
            image.set_pixel_size(render_size);
            return image;
        }
    }

    let image = gtk4::Image::from_icon_name("application-x-executable");
    image.set_pixel_size(render_size);
    image
}

pub fn build_gicon_image(display: &gdk::Display, gicon: &gtk4::gio::Icon, render_size: i32) -> gtk4::Image {
    let theme = gtk4::IconTheme::for_display(display);
    let paintable = theme.lookup_by_gicon(
        gicon,
        render_size,
        1,
        gtk4::TextDirection::None,
        gtk4::IconLookupFlags::empty(),
    );
    let image = gtk4::Image::from_paintable(Some(&paintable));
    image.set_pixel_size(render_size);
    image
}

fn icon_candidates(name: &str) -> Vec<String> {
    let mut candidates = vec![name.to_string()];

    let no_ext = name
        .strip_suffix(".png")
        .or_else(|| name.strip_suffix(".svg"))
        .or_else(|| name.strip_suffix(".xpm"))
        .unwrap_or(name);
    if no_ext != name {
        candidates.push(no_ext.to_string());
    }

    let lower = no_ext.to_lowercase();
    if !candidates.iter().any(|c| c == &lower) {
        candidates.push(lower);
    }

    candidates
}
