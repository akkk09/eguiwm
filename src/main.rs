#![allow(irrefutable_let_patterns)]

mod grabs;
mod handlers;
mod input;
mod layout;
mod state;
mod winit;

use smithay::reexports::{calloop::EventLoop, wayland_server::Display};
use state::Eguiwm;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_logging();

    let mut event_loop: EventLoop<Eguiwm> = EventLoop::try_new()?;
    let display: Display<Eguiwm> = Display::new()?;
    let mut state = Eguiwm::new(&mut event_loop, display);

    // Development backend: run nested so a compositor crash cannot strand the user
    // on a broken native session.
    winit::init_winit(&mut event_loop, &mut state)?;

    // Child clients should connect to this nested compositor, not the host.
    unsafe { std::env::set_var("WAYLAND_DISPLAY", &state.socket_name) };

    if let Some(command) = std::env::args().skip(1).collect::<Vec<_>>().windows(2)
        .find(|args| args[0] == "--command")
        .map(|args| args[1].clone())
    {
        std::process::Command::new(command).spawn()?;
    }

    event_loop.run(None, &mut state, move |_| {})?;
    Ok(())
}

fn init_logging() {
    if let Ok(filter) = tracing_subscriber::EnvFilter::try_from_default_env() {
        tracing_subscriber::fmt().with_env_filter(filter).init();
    } else {
        tracing_subscriber::fmt().init();
    }
}
