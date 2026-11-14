//! A player CFG can make the demo help line red while the status line still says Content (MOOD_HEALTH_FLOOR_PCT=30): no grass then. The threshold table is write-once per process, so this has its own
//! binary.

use bornhack_aegg::game::engine::needs::P;
use bornhack_aegg::game::engine::scene::background_state;
use bornhack_aegg::game::engine::sim;
use bornhack_aegg::game::engine::status::{Word, status_line};
use bornhack_aegg::game::engine::thresholds::{BORNPETS_CFG_KEYS, Mode, Thresholds, validate};
use bornhack_aegg::game::engine::{GameState, PetKind, PetLine, Phase};

fn install() {
    let mut c = Thresholds::CLASSIC;
    let set = BORNPETS_CFG_KEYS
        .iter()
        .find(|(k, _)| *k == "MOOD_HEALTH_FLOOR_PCT")
        .unwrap()
        .1;
    set(&mut c, 30);
    sim::install_preset(validate(&c, &Thresholds::CLASSIC), Mode::Classic, true);
}

fn demo_pet(health_pct: i32) -> GameState {
    let mut s = GameState::new_egg(7, PetKind(0));
    s.phase = Phase::Active;
    s.fullness = (60 * P) as u16;
    s.hydration = (60 * P) as u16;
    s.energy = (80 * P) as u16;
    s.mood = (90 * P) as u16;
    s.fun = (60 * P) as u16;
    s.dirt = 0;
    s.booze = 0;
    s.demo.on = true;
    s.health = (health_pct * P) as u16;
    s
}

#[test]
fn a_red_help_line_over_a_content_status_shows_no_grass() {
    install();
    let s = demo_pet(32);
    let line = status_line(&s);
    assert_eq!(
        (line.first, line.second, line.danger),
        (Word::Content, None, false)
    );
    assert_eq!(s.pet_line(), PetLine::Help);
    assert!(!background_state(&s).unwrap().grass);
}
