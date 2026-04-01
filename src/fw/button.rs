use crate::{DISPLAY_STATE, update_health};
use crate::fw::game::nav::{nav_down, nav_left, nav_right, nav_up, NavResult};
use embassy_nrf::gpio::Input;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, watch::Sender, watch::Watch};

macro_rules! update_button_health {
    ($pin:expr, $field:ident) => {
        if $pin.is_low() {
            update_health!(|f| f.buttons.$field.seen_low = true);
        } else {
            update_health!(|f| f.buttons.$field.seen_high = true);
        }
    };
}

pub static BTN_WATCH: Watch<CriticalSectionRawMutex, u8, 2> = Watch::new();

pub async fn run_buttons(
    mut btn_can: Input<'_>,
    mut btn_exe: Input<'_>,
    mut joy_up: Input<'_>,
    mut joy_down: Input<'_>,
    mut joy_left: Input<'_>,
    mut joy_right: Input<'_>,
    mut joy_fire: Input<'_>,
) {
    let btn_sender: Sender<CriticalSectionRawMutex, u8, 2> = BTN_WATCH.sender();
    loop {
        let (_btn, index) = embassy_futures::select::select_array([
            btn_can.wait_for_any_edge(),
            btn_exe.wait_for_any_edge(),
            joy_up.wait_for_any_edge(),
            joy_down.wait_for_any_edge(),
            joy_left.wait_for_any_edge(),
            joy_right.wait_for_any_edge(),
            joy_fire.wait_for_any_edge(),
        ])
        .await;

        // Handle the specific button that was pressed (active low)
        match index {
            0 => {
                defmt::info!("Cancel button {}", btn_can.is_low());
                if btn_can.is_low() {
                    DISPLAY_STATE.lock(|f| f.borrow_mut().on_cancel());
                    btn_sender.send(index as u8);
                }
                update_button_health!(btn_can, cancel);
            }
            1 => {
                defmt::info!("Execute button pressed");
                if btn_exe.is_low() {
                    btn_sender.send(index as u8);
                }
                update_button_health!(btn_exe, execute);
            }
            2 => {
                if joy_up.is_low() {
                    let on_game = DISPLAY_STATE.lock(|f| f.borrow().active_screen()) == 0;
                    if on_game { nav_up(); } else { DISPLAY_STATE.lock(|f| f.borrow_mut().menu_up()); }
                    btn_sender.send(index as u8);
                }
                defmt::info!("Joystick up");
                update_button_health!(joy_up, up);
            }
            3 => {
                if joy_down.is_low() {
                    let on_game = DISPLAY_STATE.lock(|f| f.borrow().active_screen()) == 0;
                    if on_game { nav_down(); } else { DISPLAY_STATE.lock(|f| f.borrow_mut().menu_down()); }
                    btn_sender.send(index as u8);
                }
                defmt::info!("Joystick down");
                update_button_health!(joy_down, down);
            }
            4 => {
                if joy_left.is_low() {
                    let on_game = DISPLAY_STATE.lock(|f| f.borrow().active_screen()) == 0;
                    if on_game { nav_left(); } else { DISPLAY_STATE.lock(|f| f.borrow_mut().screen_left()); }
                    btn_sender.send(index as u8);
                }
                defmt::info!("Joystick left");
                update_button_health!(joy_left, left);
            }
            5 => {
                if joy_right.is_low() {
                    let on_game = DISPLAY_STATE.lock(|f| f.borrow().active_screen()) == 0;
                    if on_game {
                        if matches!(nav_right(), NavResult::NextScreen) {
                            DISPLAY_STATE.lock(|f| f.borrow_mut().screen_right());
                        }
                    } else {
                        DISPLAY_STATE.lock(|f| f.borrow_mut().screen_right());
                    }
                    btn_sender.send(index as u8);
                }
                defmt::info!("Joystick right");
                update_button_health!(joy_right, right);
            }
            6 => {
                if joy_fire.is_low() {
                    DISPLAY_STATE.lock(|f| f.borrow_mut().fire());
                    btn_sender.send(index as u8);
                }
                defmt::info!("Joystick fire: {}", joy_fire.is_low());
                update_button_health!(joy_fire, fire);
            }
            _ => unreachable!(),
        }
    }
}
