mod compositor;
mod xdg_shell;

use crate::Eguiwm;

use smithay::{
    input::{
        dnd::{DnDGrab, DndGrabHandler, GrabType, Source},
        pointer::Focus,
        Seat, SeatHandler, SeatState,
    },
    reexports::wayland_server::{protocol::wl_surface::WlSurface, Resource},
    utils::Serial,
    wayland::{
        output::OutputHandler,
        pointer_constraints::PointerConstraintsHandler,
        selection::{
            data_device::{set_data_device_focus, DataDeviceHandler, DataDeviceState, WaylandDndGrabHandler},
            SelectionHandler,
        },
    },
};

impl SeatHandler for Eguiwm {
    type KeyboardFocus = WlSurface;
    type PointerFocus = WlSurface;
    type TouchFocus = WlSurface;

    fn seat_state(&mut self) -> &mut SeatState<Eguiwm> {
        &mut self.seat_state
    }

    fn cursor_image(
        &mut self,
        _seat: &Seat<Self>,
        _image: smithay::input::pointer::CursorImageStatus,
    ) {
    }

    fn focus_changed(&mut self, seat: &Seat<Self>, focused: Option<&WlSurface>) {
        let dh = &self.display_handle;
        let client = focused.and_then(|surface| dh.get_client(surface.id()).ok());
        set_data_device_focus(dh, seat, client);
    }
}

impl PointerConstraintsHandler for Eguiwm {}

impl SelectionHandler for Eguiwm {
    type SelectionUserData = ();
}

impl DataDeviceHandler for Eguiwm {
    fn data_device_state(&mut self) -> &mut DataDeviceState {
        &mut self.data_device_state
    }
}

impl DndGrabHandler for Eguiwm {}

impl WaylandDndGrabHandler for Eguiwm {
    fn dnd_requested<S: Source>(
        &mut self,
        source: S,
        _icon: Option<WlSurface>,
        seat: Seat<Self>,
        serial: Serial,
        type_: GrabType,
    ) {
        match type_ {
            GrabType::Pointer => {
                let pointer = seat.get_pointer().unwrap();
                let start_data = pointer.grab_start_data().unwrap();
                let grab = DnDGrab::new_pointer(&self.display_handle, start_data, source, seat);
                pointer.set_grab(self, grab, serial, Focus::Keep);
            }
            GrabType::Touch => {
                // eguiwm lacks touch handling.
                source.cancel();
            }
        }
    }
}

impl OutputHandler for Eguiwm {}

smithay::delegate_dispatch2!(Eguiwm);
