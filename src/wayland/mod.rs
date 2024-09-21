use crate::control::{Controller, Eyes, GuiContext};
use std::process::Command;
use std::str;

#[derive(Debug, Clone, Copy)]
struct Selection {
    tl: [i32; 2],
    dim: [i32; 2],
}
pub struct WaylandContext {
    selection: Selection,
}
pub struct WaylandController {
    selection: Selection,
}
pub struct WaylandEyes {
    selection: Selection,
}

impl GuiContext for WaylandContext {
    type Controller = WaylandController;
    type Eyes = WaylandEyes;

    fn from_window_name(_: &str) -> eyre::Result<Self> {
        // Use slurp to select a region during context initialization
        let slurp_output = Command::new("slurp").output()?;
        let region = str::from_utf8(&slurp_output.stdout)?.trim();

        // Parse the region string (assuming format "x,y width,height")
        let parts: Vec<&str> = region.split_whitespace().collect();
        let tl_parts: Vec<i32> = parts[0]
            .split(',')
            .map(|s| s.parse().unwrap_or(0))
            .collect();
        let dim_parts: Vec<i32> = parts[1]
            .split(',')
            .map(|s| s.parse().unwrap_or(0))
            .collect();

        let selection = Selection {
            tl: [tl_parts[0], tl_parts[1]],
            dim: [dim_parts[0], dim_parts[1]],
        };

        Ok(WaylandContext { selection })
    }

    fn controller(&self) -> eyre::Result<Self::Controller> {
        Ok(WaylandController {
            selection: self.selection,
        })
    }

    fn eyes(&self) -> eyre::Result<Self::Eyes> {
        Ok(WaylandEyes {
            selection: self.selection,
        })
    }
}

impl WaylandController {
    fn adjust_coords(&self, coords: [i32; 2]) -> [i32; 2] {
        [
            coords[0] + self.selection.tl[0],
            coords[1] + self.selection.tl[1],
        ]
    }

    fn move_mouse(&self, adjusted_coords: [i32; 2]) -> eyre::Result<()> {
        Command::new("ydotool")
            .args([
                "mousemove",
                &adjusted_coords[0].to_string(),
                &adjusted_coords[1].to_string(),
            ])
            .output()?;
        Ok(())
    }

    fn perform_click(&self) -> eyre::Result<()> {
        Command::new("ydotool").args(["click", "1"]).output()?;
        Ok(())
    }

    pub fn cast_hook(&self) -> eyre::Result<()> {
        Command::new("ydotool")
            .args(["key", "`"])
            .output()?;
        Ok(())
    }
}

impl Controller for WaylandController {
    fn run(
        self,
        recv: std::sync::mpsc::Receiver<crate::control::ToController>,
    ) -> eyre::Result<()> {
        loop {
            match recv.recv()? {
                crate::control::ToController::MoveMouse(coords) => {
                    let adjusted_coords = self.adjust_coords(coords);
                    self.move_mouse(adjusted_coords)?;
                }
                crate::control::ToController::PerformClick(coords) => {
                    let adjusted_coords = self.adjust_coords(coords);
                    self.move_mouse(adjusted_coords)?;
                    self.perform_click()?;
                }
                crate::control::ToController::CastHook => {
                    self.cast_hook()?;
                }
            }
        }
    }
}

impl Eyes for WaylandEyes {
    fn run(self, send: std::sync::mpsc::SyncSender<crate::control::ToBrain>) -> eyre::Result<()> {
        loop {
            let grim_output = Command::new("grim")
                .args([
                    "-g",
                    &format!(
                        "{}x{}+{}+{}",
                        self.selection.dim[0],
                        self.selection.dim[1],
                        self.selection.tl[0],
                        self.selection.tl[1]
                    ),
                    "-",
                ])
                .output()?;
            let image = image::load_from_memory(&grim_output.stdout)?.to_rgb8();

            send.send(crate::control::ToBrain::NextFrame(image))?;
        }
    }
}
