#![no_std]
#![no_main]

pub mod fw;

use core::result::{Result, Result::Ok};
use embedded_graphics::{
    mono_font::{MonoTextStyle, ascii::FONT_10X20},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, PrimitiveStyle},
    text::{Alignment, Baseline, Text, TextStyleBuilder},
};

pub const FOREGROUND_COLOR: BinaryColor = BinaryColor::Off;

/// Draw your graphics to any display that implements DrawTarget
pub fn draw_graphics<D>(display: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = BinaryColor>,
{
    // Clear the display, all white
    let _ = display.clear(BinaryColor::On);
    let centered = TextStyleBuilder::new()
        .baseline(Baseline::Middle)
        .alignment(Alignment::Center)
        .build();

    let position = Point::new(76, 76);
    Circle::with_center(position, 125)
        .into_styled(PrimitiveStyle::with_fill(FOREGROUND_COLOR))
        .draw(display)?;

    // Put text "HELLO GRAPHICS" on the display, centered in white
    let text_style = MonoTextStyle::new(&FONT_10X20, BinaryColor::On);
    let text = Text::with_text_style(
        "HELLO",
        display.bounding_box().center(),
        text_style,
        centered,
    );
    text.draw(display)?;

    Ok(())
}
