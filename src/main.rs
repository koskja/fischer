#![feature(let_chains)]
mod bitmap;
mod control;
mod recog;
mod wshim;
pub use wshim::*;

use recog::Brain;
use std::{
    io::Write,
    sync::mpsc::sync_channel,
    thread::spawn,
};

fn main() -> eyre::Result<()> {
    let eyes = Eyes::new("World of Warcraft").unwrap();
    let controller = eyes.make_hands();
    let brain = Brain::new();
    let (s1, r1) = sync_channel(2);
    let (s2, r2) = sync_channel(2);
    let a = spawn(move || eyes.run(s1));
    let b = spawn(move || brain.run(r1, s2));
    let c = spawn(move || controller.run(r2));
    for _ in 0.. {
        if a.is_finished() || b.is_finished() || c.is_finished() {
            break;
        }
        std::io::stdout().flush()?;
    }
    Ok(())
}
