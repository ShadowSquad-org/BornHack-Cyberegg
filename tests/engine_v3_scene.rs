//! BornPets v3 backgrounds, the engine part (plan 7, task 1).

mod state {
    use bornhack_aegg::game::bornpets_cfg::apply_overrides;
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::scene::{
        BackgroundState, FLOOR_ITEMS, Sky, background_state, floor_items, floor_lines, is_night,
        minutes_to_switch, sky, wake_view,
    };
    use bornhack_aegg::game::engine::thresholds::{BORNPETS_CFG_KEYS, Thresholds, validate};
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn active() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        s.fullness = (60 * P) as u16;
        s.hydration = (60 * P) as u16;
        s.energy = (80 * P) as u16;
        s.health = (95 * P) as u16;
        s.mood = (90 * P) as u16;
        s.fun = (60 * P) as u16;
        s.dirt = 0;
        s.booze = 0;
        s
    }

    #[test]
    fn night_is_nine_to_seven() {
        assert!(!is_night(1259));
        assert!(is_night(1260));
        assert!(is_night(0));
        assert!(is_night(419));
        assert!(!is_night(420));
    }

    #[test]
    fn an_unknown_clock_is_plain() {
        assert_eq!(sky(None), Sky::Plain);
        assert_eq!(sky(Some(720)), Sky::Day);
        assert_eq!(sky(Some(1380)), Sky::Night);
    }

    #[test]
    fn floor_items_build_up_in_order() {
        assert_eq!(floor_lines(), [40, 55, 70, 85, 95]);
        let at = |pct: i32| floor_items((pct * P) as u16);
        assert_eq!(at(39), 0);
        assert_eq!(at(40), 1);
        assert_eq!(at(54), 1);
        assert_eq!(at(55), 2);
        assert_eq!(at(70), 3);
        assert_eq!(at(85), 4);
        assert_eq!(at(95), 5);
        assert_eq!(at(100), 5);
        assert_eq!(FLOOR_ITEMS, 5);
    }

    #[test]
    fn switch_minutes_wrap_midnight() {
        assert_eq!(minutes_to_switch(None), None);
        assert_eq!(minutes_to_switch(Some(1250)), Some(10));
        assert_eq!(minutes_to_switch(Some(1260)), Some(600));
        assert_eq!(minutes_to_switch(Some(0)), Some(420));
        assert_eq!(minutes_to_switch(Some(420)), Some(840));
    }

    #[test]
    fn content_pet_gets_grass_and_a_problem_takes_it() {
        let mut s = active();
        let b = background_state(&s).expect("active pet draws a background");
        assert!(b.grass, "{:?}", s.pet_line());
        assert!(!b.poorly);
        s.fullness = (5 * P) as u16;
        assert!(!background_state(&s).unwrap().grass);
    }

    #[test]
    fn a_red_line_never_shows_grass() {
        // Danger line (filthy) is red.
        let mut s = active();
        s.dirt = (90 * P) as u16;
        assert!(s.pet_line().red());
        assert!(!background_state(&s).unwrap().grass);
        // Demo help line (health 30..34 %): red, and the status line says
        // Poorly, so these hold without the `!red` clause. The clause itself
        // is pinned in engine_v3_scene_cfg.rs.
        let mut s = active();
        s.demo.on = true;
        s.health = (32 * P) as u16;
        assert!(s.pet_line().red(), "{:?}", s.pet_line());
        assert!(!background_state(&s).unwrap().grass);
    }

    #[test]
    fn the_cross_needs_poorly_and_a_red_line() {
        let mut s = active();
        s.health = (20 * P) as u16;
        assert!(s.pet_line().red());
        assert!(background_state(&s).unwrap().poorly);
        let mut s = active();
        s.health = (32 * P) as u16;
        assert!(!background_state(&s).unwrap().poorly);
    }

    #[test]
    fn hibernating_and_eggs_draw_nothing() {
        let mut s = GameState::new_egg(7, PetKind(0));
        assert_eq!(s.phase, Phase::Hatching);
        assert_eq!(background_state(&s), None);
        s.phase = Phase::Active;
        assert!(background_state(&s).is_some());
        s.hibernating = true;
        assert_eq!(background_state(&s), None);
        assert_eq!(wake_view(&s), 0);
        s.hibernating = false;
        s.phase = Phase::Leaving;
        assert!(background_state(&s).is_some());
        s.phase = Phase::Gone;
        assert_eq!(background_state(&s), None);
    }

    #[test]
    fn pack_never_collides() {
        let mut seen = std::collections::HashSet::new();
        for sky in [Sky::Plain, Sky::Day, Sky::Night] {
            for floor in 0..=FLOOR_ITEMS as u8 {
                for grass in [false, true] {
                    for poorly in [false, true] {
                        let p = BackgroundState {
                            sky,
                            floor,
                            grass,
                            poorly,
                        }
                        .pack();
                        assert_ne!(p, 0);
                        assert!(seen.insert(p), "collision at {p}");
                    }
                }
            }
        }
        assert_eq!(seen.len(), 3 * 6 * 2 * 2);
    }

    #[test]
    fn floor_lines_must_rise() {
        let preset = Thresholds::CLASSIC;
        let mut t = preset;
        let ok = apply_overrides(&mut t, b"FLOOR_CAN_DIRT_PCT=50\n");
        assert_eq!(ok, 1);
        assert_eq!(t.FLOOR_CAN_DIRT_PCT, 50);
        let v = validate(&t, &preset);
        assert_eq!(
            [
                v.FLOOR_CRUMBS_DIRT_PCT,
                v.FLOOR_PEEL_DIRT_PCT,
                v.FLOOR_CAN_DIRT_PCT,
                v.FLOOR_SOCK_DIRT_PCT,
                v.FLOOR_PIZZA_DIRT_PCT
            ],
            [40, 55, 70, 85, 95]
        );
        let mut t = preset;
        apply_overrides(&mut t, b"BG_NIGHT_FROM_MIN=600\nBG_NIGHT_TO_MIN=600\n");
        let v = validate(&t, &preset);
        assert_eq!((v.BG_NIGHT_FROM_MIN, v.BG_NIGHT_TO_MIN), (1260, 420));
        assert!(
            BORNPETS_CFG_KEYS
                .iter()
                .any(|(k, _)| *k == "BG_NIGHT_FROM_MIN")
        );
    }
}
