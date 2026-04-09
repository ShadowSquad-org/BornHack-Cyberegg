#![cfg(feature = "simulator")]
extern crate embedded_graphics as eg;
extern crate embedded_graphics_simulator as simulator;

use hello_graphics::{
    DISPLAY_STATE, DisplayState, TriColor, draw_graphics,
    game::input::{GameBtn, dispatch},
    with_display_state_mut,
};

use eg::pixelcolor::Rgb888;
use eg::prelude::*;
use embedded_graphics_simulator::{
    OutputSettings, SimulatorDisplay, SimulatorEvent, Window, sdl2::Keycode,
};

/// Adapter that translates TriColor draw calls to an Rgb888 SimulatorDisplay.
struct TriColorDisplay<'a>(&'a mut SimulatorDisplay<Rgb888>);

impl DrawTarget for TriColorDisplay<'_> {
    type Color = TriColor;
    type Error = core::convert::Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Self::Error>
    where
        I: IntoIterator<Item = Pixel<TriColor>>,
    {
        self.0
            .draw_iter(
                pixels
                    .into_iter()
                    .map(|Pixel(p, c)| Pixel(p, Rgb888::from(c))),
            )
            .unwrap();
        Ok(())
    }
}

impl OriginDimensions for TriColorDisplay<'_> {
    fn size(&self) -> Size {
        self.0.size()
    }
}

fn main() -> Result<(), core::convert::Infallible> {
    let mut display: SimulatorDisplay<Rgb888> = SimulatorDisplay::new(Size::new(152, 152));
    let mut window = Window::new("BornPets simulator", &OutputSettings::default());

    let health_str = "sim";
    let bat_prc: u8 = 85;

    let mut need_redraw = true;

    'running: loop {
        if need_redraw {
            display.clear(Rgb888::new(255, 255, 255)).unwrap();
            draw_graphics(&mut TriColorDisplay(&mut display), health_str, &bat_prc).unwrap();
            need_redraw = false;
        }

        window.update(&mut display);

        for event in window.events() {
            match event {
                SimulatorEvent::Quit => break 'running,
                SimulatorEvent::KeyDown { keycode, .. } => {
                    let active =
                        hello_graphics::with_display_state!(|s: &std::cell::Ref<'_, DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>>| {
                            s.active_screen()
                        });
                    match keycode {
                        Keycode::Escape => break 'running,
                        Keycode::Up => {
                            if active == 0 {
                                dispatch(GameBtn::Up);
                            } else {
                                with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| s.menu_up());
                            }
                            need_redraw = true;
                        }
                        Keycode::Down => {
                            if active == 0 {
                                dispatch(GameBtn::Down);
                            } else {
                                with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| s.menu_down());
                            }
                            need_redraw = true;
                        }
                        Keycode::Left => {
                            if active == 0 {
                                dispatch(GameBtn::Left);
                            } else {
                                with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| s.screen_left());
                            }
                            need_redraw = true;
                        }
                        Keycode::Right => {
                            if active == 0 {
                                let consumed = dispatch(GameBtn::Right);
                                if !consumed {
                                    with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| {
                                        s.screen_right()
                                    });
                                }
                            } else {
                                with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| s.screen_right());
                            }
                            need_redraw = true;
                        }
                        Keycode::Return => {
                            if active == 0 {
                                dispatch(GameBtn::Fire);
                            } else {
                                with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| s.fire());
                            }
                            need_redraw = true;
                        }
                        Keycode::Backspace => {
                            if active == 0 {
                                dispatch(GameBtn::Cancel);
                            } else {
                                with_display_state_mut!(|s: &mut DisplayState<{ hello_graphics::menu::SCREEN_COUNT }>| s.on_cancel());
                            }
                            need_redraw = true;
                        }
                        Keycode::E => {
                            if active == 0 {
                                dispatch(GameBtn::Execute);
                            }
                            need_redraw = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }

    Ok(())
}
