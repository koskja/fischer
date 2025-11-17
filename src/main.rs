#![feature(let_chains)]
mod control;
mod recog;
mod util;

#[cfg(any(
    all(feature = "windows", feature = "xserver"),
    all(feature = "windows", feature = "wayland"),
    all(feature = "windows", feature = "macos"),
    all(feature = "xserver", feature = "wayland"),
    all(feature = "xserver", feature = "macos"),
    all(feature = "wayland", feature = "macos")
))]
compile_error!("multiple window managers are incompatible");

#[cfg(feature = "macos")]
mod macos;
#[cfg(feature = "wayland")]
mod wayland;
#[cfg(feature = "windows")]
mod win32;
#[cfg(feature = "xserver")]
mod xserver;

use control::{Controller, Eyes, GuiContext, MAX_FRAMES_IN_FLIGHT};
use recog::Brain;
use std::{
    io::Write,
    sync::mpsc::sync_channel,
    thread::{spawn, JoinHandle},
};

type ResultJoinHandle = JoinHandle<eyre::Result<()>>;
pub struct Handles {
    pub brain: ResultJoinHandle,
    pub eyes: ResultJoinHandle,
    pub controller: ResultJoinHandle,
}

fn _launch<C: GuiContext + 'static>(window_name: &str) -> eyre::Result<Handles> {
    let context = <C as GuiContext>::from_window_name(window_name)?;
    let eyes = context.eyes()?;
    let controller = context.controller()?;
    let brain = Brain::new();

    let (frames_tx, frames_rx) = sync_channel(MAX_FRAMES_IN_FLIGHT);
    let (ack_tx, ack_rx) = sync_channel(MAX_FRAMES_IN_FLIGHT);
    let (ctl_tx, ctl_rx) = sync_channel(2);
    let eyes = spawn(move || eyes.run(frames_tx, ack_rx));
    let brain = spawn(move || brain.run(frames_rx, ctl_tx, ack_tx));
    let controller = spawn(move || controller.run(ctl_rx));
    Ok(Handles {
        brain,
        eyes,
        controller,
    })
}

pub fn launch(window_name: &str) -> eyre::Result<Handles> {
    #[cfg(feature = "windows")]
    return _launch::<win32::Win32Context>(window_name);
    #[cfg(feature = "xserver")]
    return _launch::<xserver::XContext>(window_name);
    #[cfg(feature = "wayland")]
    return _launch::<wayland::WaylandContext>(window_name);
    #[cfg(feature = "macos")]
    return _launch::<macos::MacosContext>(window_name);
    #[cfg(all(
        not(feature = "windows"),
        not(feature = "xserver"),
        not(feature = "wayland"),
        not(feature = "macos")
    ))]
    panic!("no features selected")
}

fn main() -> eyre::Result<()> {
    #[cfg(feature = "macos")]
    macos::request_permissions()?;

    let handles = launch("World of Warcraft")?;
    for _ in 0.. {
        if handles.brain.is_finished()
            || handles.controller.is_finished()
            || handles.eyes.is_finished()
        {
            break;
        }
        std::io::stdout().flush()?;
    }
    Ok(())
}
