use crate::{Eguiwm, grabs::resize_grab, state::ClientState};
use smithay::{
    backend::renderer::utils::on_commit_buffer_handler,
    reexports::wayland_server::{
        Client,
        protocol::{wl_buffer, wl_surface::WlSurface},
    },
    wayland::{
        buffer::BufferHandler,
        compositor::{
            CompositorClientState, CompositorHandler, CompositorState, get_parent, is_sync_subsurface,
        },
        shm::{ShmHandler, ShmState},
    },
};

use super::xdg_shell;

impl CompositorHandler for Eguiwm {
    fn compositor_state(&mut self) -> &mut CompositorState {
        &mut self.compositor_state
    }

    fn client_compositor_state<'a>(&self, client: &'a Client) -> &'a CompositorClientState {
        &client.get_data::<ClientState>().unwrap().compositor_state
    }

    fn commit(&mut self, surface: &WlSurface) {
        on_commit_buffer_handler::<Self>(surface);
        if !is_sync_subsurface(surface) {
            let mut root = surface.clone();
            while let Some(parent) = get_parent(&root) {
                root = parent;
            }
            if let Some(window) = self
                .space
                .elements()
                .find(|w| w.toplevel().unwrap().wl_surface() == &root)
                .cloned()
            {
                // Clients set app_id after creating the toplevel, so identify the panel
                // again on its first surface commit as well as in new_toplevel().
                let is_panel = window
                    .toplevel()
                    .unwrap()
                    .current_state()
                    .app_id
                    .as_deref()
                    == Some("eguiwm-panel");
                if is_panel {
                    for workspace in &mut self.workspaces {
                        workspace.retain(|candidate| {
                            candidate.toplevel().unwrap().wl_surface()
                                != window.toplevel().unwrap().wl_surface()
                        });
                    }
                    if let Some(output) = self.space.outputs().next().cloned() {
                        if let Some(area) = self.space.output_geometry(&output) {
                            let toplevel = window.toplevel().unwrap();
                            toplevel.with_pending_state(|state| {
                                state.size = Some((area.size.w, 42).into());
                            });
                            toplevel.send_pending_configure();
                        }
                    }
                    self.space.map_element(window.clone(), (0, 0), true);
                    self.panel_window = Some(window);
                } else {
                    window.on_commit();
                }
            }
        };

        xdg_shell::handle_commit(&mut self.popups, &self.space, surface);
        resize_grab::handle_commit(&mut self.space, surface);
    }
}

impl BufferHandler for Eguiwm {
    fn buffer_destroyed(&mut self, _buffer: &wl_buffer::WlBuffer) {}
}

impl ShmHandler for Eguiwm {
    fn shm_state(&self) -> &ShmState {
        &self.shm_state
    }
}
