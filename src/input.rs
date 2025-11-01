use std::time::Duration;

use anyhow::Result;
use enigo::{Enigo, MouseButton, MouseControllable};

pub fn click_at(x: i32, y: i32) -> Result<()> {
    let mut enigo = Enigo::new();
    enigo.mouse_move_to(x, y);
    std::thread::sleep(Duration::from_millis(30));
    enigo.mouse_click(MouseButton::Left);
    Ok(())
}
