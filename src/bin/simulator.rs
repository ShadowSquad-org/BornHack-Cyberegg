extern crate embedded_graphics as eg;
extern crate embedded_graphics_simulator as simulator;

use hello_graphics::draw_graphics;

use eg::{pixelcolor::BinaryColor, prelude::*};
use simulator::{OutputSettings, SimulatorDisplay, SimulatorEvent, Window};

fn main() -> Result<(), core::convert::Infallible> {
    println!("Hello, world!");

    let mut display: SimulatorDisplay<BinaryColor> = SimulatorDisplay::new(Size::new(152, 152));
    let mut window = Window::new("Hello Graphics", &OutputSettings::default());

    draw_graphics(&mut display).unwrap();

    'running: loop {
        window.update(&display);

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                _ => {}
            }
        }
    }

    // All done nothing to see.
    Ok(())
}
