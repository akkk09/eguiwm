use std::{ffi::OsString, io::Read, os::unix::net::UnixListener, path::PathBuf, sync::Arc};

use smithay::{
    desktop::{PopupManager, Space, Window, WindowSurfaceType},
    input::{Seat, SeatState},
    reexports::{
        calloop::{EventLoop, Interest, LoopSignal, Mode, PostAction, generic::Generic},
        wayland_server::{
            Display, DisplayHandle,
            backend::{ClientData, ClientId, DisconnectReason},
            protocol::wl_surface::WlSurface,
        },
    },
    utils::{Logical, Point},
    wayland::{
        compositor::{CompositorClientState, CompositorState},
        output::OutputManagerState,
        selection::data_device::DataDeviceState,
        shell::xdg::XdgShellState,
        shm::ShmState,
        socket::ListeningSocketSource,
    },
};

pub struct Eguiwm {
    pub start_time: std::time::Instant,
    pub socket_name: OsString,
    pub display_handle: DisplayHandle,

    pub space: Space<Window>,
    pub loop_signal: LoopSignal,
    pub workspaces: Vec<Vec<Window>>,
    pub active_workspace: usize,
    pub panel_window: Option<Window>,
    pub control_listener: UnixListener,
    pub control_socket: PathBuf,

    // Smithay State
    pub compositor_state: CompositorState,
    pub xdg_shell_state: XdgShellState,
    pub shm_state: ShmState,
    pub output_manager_state: OutputManagerState,
    pub seat_state: SeatState<Eguiwm>,
    pub data_device_state: DataDeviceState,
    pub popups: PopupManager,

    pub seat: Seat<Self>,
}

impl Eguiwm {
    pub fn new(event_loop: &mut EventLoop<Self>, display: Display<Self>) -> Self {
        let start_time = std::time::Instant::now();

        let dh = display.handle();

        // Here we initialize implementations of some wayland protocols
        // Some of them require us to implement traits on the Eguiwm state,
        // you can find those implementations in the `crate::handlers` module

        // Initialize protocols needed for displaying windows
        let compositor_state = CompositorState::new::<Self>(&dh);
        let xdg_shell_state = XdgShellState::new::<Self>(&dh);
        let shm_state = ShmState::new::<Self>(&dh, vec![]);
        let popups = PopupManager::default();

        let output_manager_state = OutputManagerState::new_with_xdg_output::<Self>(&dh);

        // Data device is responsible for clipboard and drag-and-drop
        let data_device_state = DataDeviceState::new::<Self>(&dh);

        // A seat is a group of keyboards, pointer and touch devices.
        // A seat typically has a pointer and maintains a keyboard focus and a pointer focus.
        let mut seat_state = SeatState::new();
        let mut seat: Seat<Self> = seat_state.new_wl_seat(&dh, "winit");

        // Notify clients that we have a keyboard, for the sake of the example we assume that keyboard is always present.
        // You may want to track keyboard hot-plug in real compositor.
        seat.add_keyboard(Default::default(), 200, 25).unwrap();

        // Notify clients that we have a pointer (mouse)
        // Here we assume that there is always pointer plugged in
        seat.add_pointer();

        // A space represents a two-dimensional plane. Windows and Outputs can be mapped onto it.
        //
        // Windows get a position and stacking order through mapping.
        // Outputs become views of a part of the Space and can be rendered via Space::render_output.
        let space = Space::default();

        // Setup a wayland socket that will be used to accept clients
        let socket_name = Self::init_wayland_listener(display, event_loop);

        // A tiny local IPC socket lets the panel request workspace changes.
        let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        let control_socket = runtime_dir.join(format!("eguiwm-control-{}.sock", std::process::id()));
        let _ = std::fs::remove_file(&control_socket);
        let control_listener = UnixListener::bind(&control_socket)
            .expect("failed to create eguiwm control socket");
        control_listener.set_nonblocking(true)
            .expect("failed to make eguiwm control socket nonblocking");

        // Get the loop signal, used to stop the event loop
        let loop_signal = event_loop.get_signal();

        Self {
            start_time,
            display_handle: dh,

            space,
            loop_signal,
            workspaces: (0..4).map(|_| Vec::new()).collect(),
            active_workspace: 0,
            panel_window: None,
            control_listener,
            control_socket,
            socket_name,

            compositor_state,
            xdg_shell_state,
            shm_state,
            output_manager_state,
            seat_state,
            data_device_state,
            popups,
            seat,
        }
    }

    fn init_wayland_listener(display: Display<Eguiwm>, event_loop: &mut EventLoop<Self>) -> OsString {
        // Creates a new listening socket, automatically choosing the next available `wayland` socket name.
        let listening_socket = ListeningSocketSource::new_auto().unwrap();

        // Get the name of the listening socket.
        // Clients will connect to this socket.
        let socket_name = listening_socket.socket_name().to_os_string();

        let loop_handle = event_loop.handle();

        loop_handle
            .insert_source(listening_socket, move |client_stream, _, state| {
                // Inside the callback, you should insert the client into the display.
                //
                // You may also associate some data with the client when inserting the client.
                state
                    .display_handle
                    .insert_client(client_stream, Arc::new(ClientState::default()))
                    .unwrap();
            })
            .expect("Failed to init the wayland event source.");

        // You also need to add the display itself to the event loop, so that client events will be processed by wayland-server.
        loop_handle
            .insert_source(
                Generic::new(display, Interest::READ, Mode::Level),
                |_, display, state| {
                    // Safety: we don't drop the display
                    unsafe {
                        display.get_mut().dispatch_clients(state).unwrap();
                    }
                    Ok(PostAction::Continue)
                },
            )
            .unwrap();

        socket_name
    }

    /// Poll the panel's local control socket without blocking the compositor loop.
    pub fn poll_control_commands(&mut self) {
        loop {
            let Ok((mut stream, _)) = self.control_listener.accept() else {
                break;
            };
            let mut command = String::new();
            if stream.read_to_string(&mut command).is_ok() {
                if let Some(index) = command.trim().strip_prefix("workspace:")
                    .and_then(|value| value.parse::<usize>().ok())
                    .filter(|index| *index < self.workspaces.len())
                {
                    self.switch_workspace(index);
                }
            }
        }
    }

    pub fn switch_workspace(&mut self, index: usize) {
        if index >= self.workspaces.len() || index == self.active_workspace {
            return;
        }
        let old_windows = self.workspaces[self.active_workspace].clone();
        for window in &old_windows {
            self.space.unmap_elem(window);
        }
        self.active_workspace = index;
        self.reflow_active_workspace();
    }

    pub fn reflow_active_workspace(&mut self) {
        let Some(output) = self.space.outputs().next().cloned() else {
            return;
        };
        let Some(mut area) = self.space.output_geometry(&output) else {
            return;
        };
        // Keep the top 42 logical pixels clear for the native taskbar.
        area.loc.y += 42;
        area.size.h = (area.size.h - 42).max(1);

        let windows = self.workspaces[self.active_workspace].clone();
        let rects = crate::layout::tile_rects(area, windows.len());
        for (window, rect) in windows.iter().zip(rects) {
            self.space.map_element(window.clone(), rect.loc, true);
            if let Some(toplevel) = window.toplevel() {
                toplevel.with_pending_state(|state| state.size = Some(rect.size));
                toplevel.send_pending_configure();
            }
        }
    }

    pub fn surface_under(&self, pos: Point<f64, Logical>) -> Option<(WlSurface, Point<f64, Logical>)> {
        self.space.element_under(pos).and_then(|(window, location)| {
            window
                .surface_under(pos - location.to_f64(), WindowSurfaceType::ALL)
                .map(|(s, p)| (s, (p + location).to_f64()))
        })
    }
}

/// Data associated with a wayland client that connects to Eguiwm.
/// One instance of this type per client.
#[derive(Default)]
pub struct ClientState {
    pub compositor_state: CompositorClientState,
}

impl ClientData for ClientState {
    fn initialized(&self, _client_id: ClientId) {}
    fn disconnected(&self, _client_id: ClientId, _reason: DisconnectReason) {}
}
