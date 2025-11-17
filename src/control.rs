use std::sync::mpsc::{Receiver, SyncSender, TryRecvError};

use image::RgbImage;

pub const MAX_FRAMES_IN_FLIGHT: usize = 5;

pub enum ToBrain {
    NextFrame(RgbImage),
}
pub enum ToEyes {
    FrameProcessed,
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
    fn run(self, send: SyncSender<ToBrain>, recv: Receiver<ToEyes>) -> eyre::Result<()>;
}

pub struct FrameBudget {
    inflight: usize,
    recv: Receiver<ToEyes>,
}

impl FrameBudget {
    pub fn new(recv: Receiver<ToEyes>) -> Self {
        Self { inflight: 0, recv }
    }

    pub fn wait_for_slot(&mut self) -> eyre::Result<()> {
        while self.inflight >= MAX_FRAMES_IN_FLIGHT {
            self.wait_for_completion()?;
        }
        Ok(())
    }

    pub fn frame_sent(&mut self) -> eyre::Result<()> {
        self.inflight += 1;
        self.drain_completions()
    }

    fn wait_for_completion(&mut self) -> eyre::Result<()> {
        match self.recv.recv() {
            Ok(ToEyes::FrameProcessed) => {
                self.inflight = self.inflight.saturating_sub(1);
                Ok(())
            }
            Err(_) => Err(eyre::eyre!(
                "brain disconnected before acknowledging processed frames"
            )),
        }
    }

    fn drain_completions(&mut self) -> eyre::Result<()> {
        loop {
            match self.recv.try_recv() {
                Ok(ToEyes::FrameProcessed) => {
                    self.inflight = self.inflight.saturating_sub(1);
                }
                Err(TryRecvError::Empty) => return Ok(()),
                Err(TryRecvError::Disconnected) => {
                    return Err(eyre::eyre!(
                        "brain disconnected before acknowledging processed frames"
                    ))
                }
            }
        }
    }
}
