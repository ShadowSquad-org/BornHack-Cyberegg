#![no_std]
#![no_main]

use hello_graphics::{
    draw_graphics,
    fw::epd::{EpdBus, EpdConfig152x152 as EpdConfig, EpdGfx, init_epd, init_epd_bus},
};

use embassy_executor::Spawner;
use embassy_nrf::gpio::{Level, Output, OutputDrive};
use embassy_time::Timer;
use ssd1680::graphics::WHITE;
use static_cell::StaticCell;
use {defmt_rtt as _, panic_probe as _};

// Display dimensions, for this display always the same
// const ROWS: u16 = 152;
// const COLS: u8 = 152;

// Code is for NRF52840
// Example to port: https://github.com/mbv/esp32-ssd1680/blob/main/src/main.rs

// Pin assignments SSD1680 EDP

#[embassy_executor::main]
async fn main(_spawner: Spawner) {
    let p = embassy_nrf::init(Default::default());

    // EPD display buffers
    let dimension = EpdConfig::to_dimensions();
    const BUF_SIZE: usize = EpdConfig::BUF_SIZE;
    static BLACK_BUF: StaticCell<[u8; BUF_SIZE]> = StaticCell::new();
    static RED_BUF: StaticCell<[u8; BUF_SIZE]> = StaticCell::new();
    let black_buffer = BLACK_BUF.init([0; BUF_SIZE]);
    let red_buffer = RED_BUF.init([0; BUF_SIZE]);

    // Pin assignments

    // LED (Low active)
    let mut led_red = Output::new(p.P1_07, Level::High, OutputDrive::Standard);
    let mut _led_green = Output::new(p.P1_15, Level::High, OutputDrive::Standard);
    let mut _led_blue = Output::new(p.P0_02, Level::High, OutputDrive::Standard);
    // LEDs will be tested later

    let busy_pin = p.P0_14;
    let resetn_pin = p.P0_11;
    let dc_pin = p.P0_12;
    let csn_pin = p.P1_09;
    // let sck_pin = Output::new(p.P0_08, Level::Low, OutputDrive::Standard);
    let sck_pin = p.P0_08;
    let mosi_pin = p.P0_27;

    let test_str = "test";

    defmt::info!("{}", test_str);
    defmt::info!("{}", test_str);

    static BUS_CELL: StaticCell<EpdBus> = StaticCell::new();
    let bus = BUS_CELL.init(init_epd_bus(p.SPI3, sck_pin, mosi_pin));

    let mut display: EpdGfx<'_> = init_epd(
        bus,
        busy_pin,
        resetn_pin,
        dc_pin,
        csn_pin,
        dimension,
        black_buffer,
        red_buffer,
    )
    .unwrap();

    let _ = display.reset().await;
    display.clear(WHITE);

    draw_graphics(&mut display).unwrap();

    loop {
        led_red.set_low();
        Timer::after_millis(50).await;
        led_red.set_high();
        Timer::after_millis(4950).await;

        let _ = display.reset().await;
        let _ = display.update().await;
        defmt::info!("Updated EPD");
        let _ = display.deep_sleep().await.unwrap();
    }
}
