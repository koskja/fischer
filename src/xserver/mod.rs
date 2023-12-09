use crate::control::{Controller, Eyes};

pub struct XController;
pub struct XEyes;
impl Controller for XController {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        todo!()
    }

    fn run(self, recv: std::sync::mpsc::Receiver<crate::control::ToController>) -> eyre::Result<()> {
        todo!()
    }
}
impl Eyes for XEyes {
    fn from_window_name(name: &str) -> eyre::Result<Self> {
        todo!()
    }

    fn run(self, send: std::sync::mpsc::SyncSender<crate::control::ToBrain>) -> eyre::Result<()> {
        todo!()
    }
}