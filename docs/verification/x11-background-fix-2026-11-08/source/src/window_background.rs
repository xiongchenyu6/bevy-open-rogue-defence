//! Match uncovered X11 resize pixels to the renderer's clear color.
use bevy::{prelude::*, window::RawHandleWrapper};
use raw_window_handle::RawWindowHandle;
use x11rb::{
    connection::Connection,
    protocol::xproto::{ChangeWindowAttributesAux, ConnectionExt},
};

pub(crate) fn match_clear_color(
    windows: Query<&RawHandleWrapper, Added<RawHandleWrapper>>,
    clear: Res<ClearColor>,
) {
    for window in &windows {
        let RawWindowHandle::Xlib(handle) = window.get_window_handle() else {
            continue;
        };
        let result = set_background(handle.window as u32, clear.0);
        if let Err(error) = result {
            warn!("Could not match X11 resize background to clear color: {error}");
        }
    }
}

fn set_background(window: u32, color: Color) -> Result<(), Box<dyn std::error::Error>> {
    let (connection, _) = x11rb::connect(None)?;
    let color = color.to_srgba().to_u8_array();
    let colormap = connection.get_window_attributes(window)?.reply()?.colormap;
    let pixel = connection
        .alloc_color(
            colormap,
            u16::from(color[0]) * 257,
            u16::from(color[1]) * 257,
            u16::from(color[2]) * 257,
        )?
        .reply()?
        .pixel;
    connection
        .change_window_attributes(window, &ChangeWindowAttributesAux::new().background_pixel(pixel))?
        .check()?;
    connection.flush()?;
    Ok(())
}
