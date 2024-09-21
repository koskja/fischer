use std::sync::mpsc::{Receiver, SyncSender};

use image::RgbImage;

pub enum ToBrain {
    NextFrame(RgbImage),
}
pub enum ToController {
    /// Move the mouse to a position relative to the target window
    MoveMouse([i32; 2]),
    /// Perform a mouse click at a position relative to the target window
    PerformClick([i32; 2]),
    /// Sends a message that the BACKTICK key was pressed
    CastHook,
}
pub trait GuiContext: Sized + Send + Sync {
    type Controller: Controller;
    type Eyes: Eyes;
    fn from_window_name(name: &str) -> eyre::Result<Self>;
    fn controller(&self) -> eyre::Result<Self::Controller>;
    fn eyes(&self) -> eyre::Result<Self::Eyes>;
}
pub trait Controller: Sized + Send + Sync {
    fn run(self, recv: Receiver<ToController>) -> eyre::Result<()>;
}
pub trait Eyes: Sized + Send + Sync {
    fn run(self, send: SyncSender<ToBrain>) -> eyre::Result<()>;
}
