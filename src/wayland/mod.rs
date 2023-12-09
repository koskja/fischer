use crate::control::{Controller, Eyes};


pub struct WaylandController;
pub struct WaylandEyes;
impl Controller for WaylandController {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        todo!()
    }

    fn run(self, recv: std::sync::mpsc::Receiver<crate::control::ToController>) -> eyre::Result<()> {
        todo!()
    }
}
impl Eyes for WaylandEyes {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        todo!()
    }

    fn run(self, send: std::sync::mpsc::SyncSender<crate::control::ToBrain>) -> eyre::Result<()> {
        todo!()
    }
}