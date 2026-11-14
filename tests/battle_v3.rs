//! BornPets v3 battles (plan 5). Task 1: the snapshot.

mod snapshot {
    use bornhack_aegg::game::bornpets_cfg::apply_overrides;
    use bornhack_aegg::game::engine::combat::{
        Body, Factor, Flags, WIRE_FOCUS, WIRE_GUARD, WIRE_HP, WIRE_POWER, WIRE_SPEED, WIRE_STAMINA,
        derive, derive_raw, felt, felt_with,
    };
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::thresholds::Thresholds;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    /// The prototype builds (`battle::builds`) as `Body`, traits 50.
    pub(super) fn builds() -> [(&'static str, Body); 20] {
        let b = Body::REFERENCE;
        let fit = Body {
            fit: 90,
            energy: 75,
            mood: 85,
            health: 95,
            ..b
        };
        let sumo = Body {
            weight: 80,
            fit: 80,
            energy: 75,
            mood: 85,
            health: 90,
            ..b
        };
        let fat = Body {
            weight: 85,
            fit: 15,
            energy: 65,
            mood: 70,
            health: 55,
            ..b
        };
        [
            ("fit, normal weight", fit),
            ("fit, one beer", Body { booze: 15, ..fit }),
            ("thin and fit", Body { weight: 30, ..fit }),
            ("sumo (heavy, fit)", sumo),
            ("sumo, two beers", Body { booze: 30, ..sumo }),
            (
                "well rested",
                Body {
                    fit: 75,
                    energy: 95,
                    mood: 85,
                    health: 90,
                    rested: true,
                    ..b
                },
            ),
            ("average", b),
            ("fat and unfit", fat),
            ("fat, unfit, three beers", Body { booze: 45, ..fat }),
            (
                "thin and unfit",
                Body {
                    weight: 25,
                    fit: 20,
                    health: 55,
                    mood: 65,
                    ..b
                },
            ),
            (
                "wasted",
                Body {
                    booze: 70,
                    energy: 60,
                    ..b
                },
            ),
            (
                "dependent, sober (craving)",
                Body {
                    craving: true,
                    mood: 55,
                    ..b
                },
            ),
            (
                "wired",
                Body {
                    fun: 92,
                    energy: 55,
                    ..b
                },
            ),
            (
                "bored",
                Body {
                    fun: 8,
                    mood: 55,
                    ..b
                },
            ),
            (
                "hungry",
                Body {
                    full: 18,
                    mood: 60,
                    ..b
                },
            ),
            (
                "stuffed",
                Body {
                    full: 95,
                    energy: 60,
                    ..b
                },
            ),
            ("thirsty", Body { water: 25, ..b }),
            (
                "exhausted",
                Body {
                    energy: 15,
                    mood: 55,
                    ..b
                },
            ),
            (
                "hung over",
                Body {
                    hangover: true,
                    energy: 45,
                    mood: 50,
                    water: 30,
                    ..b
                },
            ),
            (
                "injured",
                Body {
                    injured: true,
                    strain: 50,
                    mood: 55,
                    ..b
                },
            ),
        ]
    }

    pub(super) fn build(name: &str) -> Body {
        builds().iter().find(|(n, _)| *n == name).unwrap().1
    }

    #[test]
    fn builds_match_section_18_4() {
        // (power, guard, speed, hp, stamina, focus), Classic preset, from
        // `cargo run -- battle` at 5e59574.
        let want = [
            (39, 16, 71, 75, 4, 0),
            (43, 15, 71, 75, 4, -2),
            (33, 14, 75, 70, 4, 0),
            (47, 23, 49, 80, 3, 0),
            (52, 21, 49, 80, 3, -3),
            (39, 16, 70, 72, 6, 5),
            (36, 16, 59, 68, 3, 0),
            (45, 25, 20, 60, 1, 0),
            (52, 22, 20, 60, 1, -4),
            (33, 14, 48, 60, 2, 0),
            (36, 14, 59, 68, 3, -48),
            (33, 16, 59, 68, 3, -10),
            (36, 16, 59, 68, 3, -19),
            (36, 16, 59, 68, 3, -12),
            (36, 16, 59, 68, 1, 0),
            (36, 16, 43, 68, 3, 0),
            (36, 16, 59, 68, 2, -4),
            (36, 16, 59, 68, 1, 0),
            (36, 16, 51, 68, 1, -14),
            (20, 16, 35, 68, 3, 0),
        ];
        for ((name, body), w) in builds().iter().zip(want) {
            let s = derive(body);
            let got = (
                s.power as i32,
                s.guard as i32,
                s.speed as i32,
                s.hp as i32,
                s.stamina as i32,
                s.focus as i32,
            );
            assert_eq!(got, w, "{name}");
        }
    }

    #[test]
    fn flags_name_the_conditions() {
        let on = |name: &str| derive(&build(name)).flags;
        let sumo = on("sumo, two beers");
        assert!(sumo.has(Factor::Heavy) && sumo.has(Factor::Tipsy));
        assert!(!sumo.has(Factor::Sloppy));
        assert!(on("wasted").has(Factor::Sloppy));
        let over = on("hung over");
        assert!(over.has(Factor::HungOver) && over.has(Factor::Thirsty));
        let hurt = on("injured");
        assert!(hurt.has(Factor::Injured) && hurt.has(Factor::Sore));
        assert!(on("well rested").has(Factor::Rested));
        assert_eq!(on("average").0 & 0xF_FFFF, 0, "average pet has no factor");
        assert!(!on("average").demo());
        let demo = derive(&Body {
            demo: true,
            ..Body::REFERENCE
        })
        .flags;
        assert!(demo.demo());
        assert_eq!(demo.0 & (1 << 23), 1 << 23);
        assert_eq!(demo.0 & (1 << 22), 0, "bit 22 stays reserved");
        assert!(Flags::default().with(Factor::Filthy).has(Factor::Filthy));
        assert!(Flags::default().with_demo(true).demo());
    }

    fn xorshift(s: &mut u32) -> u32 {
        let mut x = *s;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        *s = x;
        x
    }

    fn body_from(fields: [i32; 14], flags: [bool; 7]) -> Body {
        Body {
            fit: fields[0],
            energy: fields[1],
            booze: fields[2],
            mood: fields[3],
            health: fields[4],
            strain: fields[5],
            full: fields[6],
            water: fields[7],
            fun: fields[8],
            dirt: fields[9],
            weight: fields[10],
            vit: fields[11],
            cur: fields[12],
            res: fields[13],
            rested: flags[0],
            injured: flags[1],
            groggy: flags[2],
            hangover: flags[3],
            craving: flags[4],
            deprived: flags[5],
            demo: flags[6],
        }
    }

    fn check_raw(b: &Body) {
        let r = derive_raw(b);
        let inside = |v: i32, (lo, hi): (u8, u8)| (lo as i32..=hi as i32).contains(&v);
        assert!(inside(r[0], WIRE_POWER), "power {} for {b:?}", r[0]);
        assert!(inside(r[1], WIRE_GUARD), "guard {} for {b:?}", r[1]);
        assert!(inside(r[2], WIRE_SPEED), "speed {} for {b:?}", r[2]);
        assert!(inside(r[3], WIRE_HP), "hp {} for {b:?}", r[3]);
        assert!(inside(r[4], WIRE_STAMINA), "stamina {} for {b:?}", r[4]);
        // Only focus is clamped: it has no floor in the formula.
        let s = derive(b);
        assert_eq!(
            s.focus as i32,
            r[5].clamp(WIRE_FOCUS.0 as i32, WIRE_FOCUS.1 as i32)
        );
        assert_eq!(
            [
                s.power as i32,
                s.guard as i32,
                s.speed as i32,
                s.hp as i32,
                s.stamina as i32
            ],
            [r[0], r[1], r[2], r[3], r[4]]
        );
    }

    #[test]
    fn snapshot_stays_in_wire_ranges() {
        let mut seed = 0x9E37_79B9;
        for _ in 0..20_000 {
            let mut f = [0i32; 14];
            for v in &mut f {
                *v = (xorshift(&mut seed) % 101) as i32;
            }
            let bits = xorshift(&mut seed);
            let mut fl = [false; 7];
            for (i, v) in fl.iter_mut().enumerate() {
                *v = bits >> i & 1 == 1;
            }
            check_raw(&body_from(f, fl));
        }
        for level in [0, 100] {
            for mask in 0..128u32 {
                let mut fl = [false; 7];
                for (i, v) in fl.iter_mut().enumerate() {
                    *v = mask >> i & 1 == 1;
                }
                check_raw(&body_from([level; 14], fl));
            }
        }
    }

    /// One body per number that reaches its true maximum under the presets.
    /// Pins the headroom of the WIRE ranges: random sampling never gets there.
    #[test]
    fn worst_cases_reach_the_wire_maxima() {
        let b = Body::REFERENCE;
        let cases = [
            (
                "power",
                Body {
                    weight: 100,
                    booze: 60,
                    mood: 85,
                    ..b
                },
                0,
                61,
                WIRE_POWER,
            ),
            (
                "guard",
                Body {
                    res: 100,
                    weight: 100,
                    ..b
                },
                1,
                39,
                WIRE_GUARD,
            ),
            (
                "speed",
                Body {
                    weight: 0,
                    fit: 100,
                    vit: 100,
                    rested: true,
                    ..b
                },
                2,
                100,
                WIRE_SPEED,
            ),
            (
                "hp",
                Body {
                    vit: 100,
                    health: 100,
                    weight: 100,
                    ..b
                },
                3,
                117,
                WIRE_HP,
            ),
            (
                "stamina",
                Body {
                    energy: 100,
                    fit: 100,
                    vit: 100,
                    rested: true,
                    ..b
                },
                4,
                9,
                WIRE_STAMINA,
            ),
        ];
        for (name, body, i, max, (lo, hi)) in cases {
            let v = derive_raw(&body)[i];
            assert_eq!(v, max, "{name}");
            assert!(
                (lo as i32..=hi as i32).contains(&v),
                "{name} {v} outside wire"
            );
        }
    }

    #[test]
    fn felt_never_goes_negative() {
        // A custom base below the weight: booze * (base - weight) is negative.
        let heavy_drunk = Body {
            booze: 100,
            weight: 100,
            ..Body::REFERENCE
        };
        assert_eq!(felt_with(&heavy_drunk, 20), 0);
        assert_eq!(felt_with(&heavy_drunk, 1000), 100);
        assert_eq!(felt_with(&heavy_drunk, 150), felt(&heavy_drunk));
    }

    #[test]
    fn mood_band_pair_is_validated() {
        use bornhack_aegg::game::engine::thresholds::validate;
        let mut t = Thresholds::CLASSIC;
        t.BATTLE_LOW_MOOD_PCT = 80;
        t.BATTLE_HIGH_MOOD_PCT = 60;
        let v = validate(&t, &Thresholds::CLASSIC);
        assert_eq!((v.BATTLE_LOW_MOOD_PCT, v.BATTLE_HIGH_MOOD_PCT), (40, 80));
    }

    #[test]
    fn felt_is_the_one_formula() {
        // booze * (150 - weight) / 100, at most 100
        assert_eq!(
            felt(&Body {
                booze: 30,
                weight: 80,
                ..Body::REFERENCE
            }),
            21
        );
        assert_eq!(
            felt(&Body {
                booze: 100,
                weight: 0,
                ..Body::REFERENCE
            }),
            100
        );
        assert_eq!(
            felt(&Body {
                booze: 0,
                ..Body::REFERENCE
            }),
            0
        );
    }

    #[test]
    fn snapshot_of_a_pet_reads_its_state() {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.phase = Phase::Active;
        s.weight = (80 * P) as u16;
        s.fitness = (80 * P) as u16;
        s.booze = (20 * P) as u16;
        let b = Body::of(&s);
        assert_eq!(b.weight, 80);
        assert_eq!(b.fit, 80);
        assert_eq!(b.booze, 20);
        assert!(!b.demo && !b.injured && !b.rested);
        assert_eq!(s.snapshot(), derive(&b));
        s.injured_min = 5;
        assert!(Body::of(&s).injured && s.snapshot().flags.has(Factor::Injured));
    }

    #[test]
    fn bands_move_the_fight() {
        // Classic FULL_LO is 38: 36 is hungry, 40 is not.
        let hungry = |full| {
            derive(&Body {
                full,
                ..Body::REFERENCE
            })
            .flags
            .has(Factor::Hungry)
        };
        assert!(hungry(36));
        assert!(!hungry(40));
        // The accessors decide, not literals: none of the band literals of
        // the prototype survive in the source (comments stripped).
        let src: String = include_str!("../src/game/engine/combat.rs")
            .lines()
            .map(|l| l.split("//").next().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        for lit in [" 78", " 83", " 38", " 33", " 73", " 58", " 63"] {
            assert!(!src.contains(lit), "band literal{lit} in combat.rs");
        }
        // Stricter: the number as a whole token, whatever precedes it.
        let b = src.as_bytes();
        for lit in ["78", "83", "38", "33", "73", "58", "63"] {
            for (i, _) in src.match_indices(lit) {
                let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
                let before = i > 0 && word(b[i - 1]);
                let after = b.get(i + 2).is_some_and(|&c| word(c));
                assert!(before || after, "band literal {lit} in combat.rs");
            }
        }
    }

    #[test]
    fn felt_base_survives_the_cfg_load() {
        // A percent key would clamp to 1..=99 and break every cell.
        let mut t = Thresholds::CLASSIC;
        apply_overrides(&mut t, b"BATTLE_FELT_BASE_PCT=150\n");
        assert_eq!(t.BATTLE_FELT_BASE_PCT, 150);
        apply_overrides(&mut t, b"BATTLE_FELT_BASE_PCT=200\n");
        assert_eq!(t.BATTLE_FELT_BASE_PCT, 200);
    }

    #[test]
    fn factor_words_are_listed_once() {
        let mut seen = std::collections::HashSet::new();
        for (i, f) in Factor::ALL.iter().enumerate() {
            assert!(seen.insert(f.text()), "duplicate {}", f.text());
            assert_eq!(f.helps(), i < 4, "{}", f.text());
            assert_eq!(*f as usize, i);
            assert!(f.text().len() <= 11, "{} too wide", f.text());
        }
        assert_eq!(seen.len(), 20);
    }
}

mod fight {
    use super::snapshot::{build, builds};
    use bornhack_aegg::game::engine::combat::{
        Body, Decisive, Factor, Form, ROUNDS, Snapshot, chance_seed, decisive, derive, factors,
        fight, form, seed, win_chance,
    };
    use bornhack_aegg::game::engine::thresholds as t;

    const GOLDEN_SEED: u32 = 1_560_395_003;

    fn snap(name: &str) -> Snapshot {
        derive(&build(name))
    }

    fn round5(w: u32) -> u8 {
        ((w * 100 + 32) / 64 + 2) as u8 / 5 * 5
    }

    #[test]
    fn matrix_matches_section_18_4() {
        let bs = builds();
        let snaps: Vec<Snapshot> = bs.iter().map(|(_, b)| derive(b)).collect();
        let mut avgs = Vec::new();
        let mut cells = vec![vec![0u32; 20]; 20];
        for i in 0..20 {
            let (mut tot, mut cnt) = (0, 0);
            for j in 0..20 {
                let mut w = 0u32;
                let base = ((i * 131 + j * 17) as u32).wrapping_add(7);
                for k in 0..2000u32 {
                    let s = k.wrapping_mul(2654435761).wrapping_add(base);
                    if fight(s, &snaps[i], &snaps[j]).a_won {
                        w += 1;
                    }
                }
                let p = w * 100 / 2000;
                cells[i][j] = p;
                if i != j {
                    tot += p;
                    cnt += 1;
                }
            }
            avgs.push(tot / cnt);
        }
        assert_eq!(
            avgs,
            [
                82, 84, 74, 85, 86, 81, 60, 41, 42, 22, 9, 44, 41, 48, 33, 48, 44, 34, 20, 2
            ]
        );
        assert_eq!(cells[0][3], 44, "fit vs sumo");
        assert_eq!(cells[3][0], 53, "sumo vs fit");
    }

    #[test]
    fn fight_is_deterministic_and_bounded() {
        let bs = builds();
        for (i, (_, a)) in bs.iter().enumerate() {
            for (j, (_, b)) in bs.iter().enumerate() {
                let (sa, sb) = (derive(a), derive(b));
                for k in 0..8u32 {
                    let s = k.wrapping_mul(2654435761).wrapping_add((i * 20 + j) as u32);
                    let o = fight(s, &sa, &sb);
                    assert_eq!(o, fight(s, &sa, &sb));
                    assert!(o.rounds <= ROUNDS && o.rounds >= 1);
                    assert!(o.a_hp_pct <= 100 && o.b_hp_pct <= 100);
                }
            }
        }
    }

    #[test]
    fn fight_reads_no_config() {
        let src = include_str!("../src/game/engine/combat.rs");
        let from = src.find("pub const ROUNDS").expect("consts");
        let to = src.find("pub fn form").expect("form");
        assert!(from < to);
        let order = [
            "pub const ROUNDS",
            "fn strike",
            "pub fn fight",
            "pub fn seed",
            "pub fn chance_seed",
            "pub fn win_chance",
            "pub fn form",
        ];
        let mut at = 0;
        for o in order {
            let p = src[at..]
                .find(o)
                .unwrap_or_else(|| panic!("{o} out of order"));
            at += p + o.len();
        }
        let stripped: String = src[from..to]
            .lines()
            .map(|l| l.split("//").next().unwrap())
            .collect::<Vec<_>>()
            .join("\n");
        assert!(!stripped.contains("thresholds"), "fight reads thresholds");
        let b = stripped.as_bytes();
        let word = |c: u8| c.is_ascii_alphanumeric() || c == b'_';
        for (i, _) in stripped.match_indices("t::") {
            let before = i > 0 && word(b[i - 1]);
            assert!(
                before,
                "fight reads a threshold: {}",
                &stripped[i.saturating_sub(10)..i + 10]
            );
        }
    }

    #[test]
    fn seed_is_symmetric_in_roles() {
        let (na, nb, ia, ib) = (0x1234, 0x5678, [1, 2], [3, 4]);
        let s = seed(na, nb, ia, ib);
        assert_eq!(s, seed(na, nb, ia, ib));
        // Swapping the roles changes the seed: the challenger goes first on both badges.
        assert_ne!(s, seed(na, nb, ib, ia));
        assert_ne!(s, seed(nb, na, ia, ib));
        assert_ne!(s, seed(na ^ 1, nb, ia, ib));
        assert_ne!(s, seed(na, nb ^ 1, ia, ib));
        assert_ne!(s, seed(na, nb, ia, [ib[0] ^ 1, ib[1]]));
        // Golden values: the wire contract must not drift between builds.
        assert_eq!(s, GOLDEN_SEED);
        assert_eq!(chance_seed(0), 7);
        assert_eq!(chance_seed(1), 2_654_435_768);
        assert_eq!(
            chance_seed(64),
            64u32.wrapping_mul(2_654_435_761).wrapping_add(7)
        );
        // A fallback fight uses nonce_b = 0.
        assert_ne!(seed(na, 0, ia, ib), s);
    }

    #[test]
    fn chance_is_honest() {
        let bs = builds();
        for (_, a) in bs.iter() {
            for (_, b) in bs.iter() {
                let (sa, sb) = (derive(a), derive(b));
                let w = (0..64u32)
                    .filter(|&k| fight(chance_seed(k), &sa, &sb).a_won)
                    .count() as u32;
                let c = win_chance(&sa, &sb);
                assert_eq!(c, round5(w));
                assert_eq!(c % 5, 0);
            }
        }
        for (n, b) in bs.iter() {
            let s = derive(b);
            assert!((40..=60).contains(&win_chance(&s, &s)), "{n} mirror");
        }
        for n in ["average", "fit, normal weight"] {
            let s = snap(n);
            assert!((45..=55).contains(&win_chance(&s, &s)), "{n} mirror");
        }
        assert!(win_chance(&snap("fit, normal weight"), &snap("injured")) >= 95);
        assert!(win_chance(&snap("injured"), &snap("fit, normal weight")) <= 5);
    }

    #[test]
    fn form_words_follow_the_chance() {
        assert_eq!(form(&snap("fit, normal weight")), Form::Top);
        assert_eq!(form(&snap("hung over")), Form::Poor);
        assert_eq!(form(&snap("injured")), Form::Poor);
        let reference = derive(&Body::REFERENCE);
        for (n, b) in builds().iter() {
            let s = derive(b);
            let c = win_chance(&s, &reference) as u32;
            let want = if c >= t::BATTLE_FORM_TOP_PCT() as u32 {
                Form::Top
            } else if c >= t::BATTLE_FORM_IN_PCT() as u32 {
                Form::In
            } else if c >= t::BATTLE_FORM_OFF_PCT() as u32 {
                Form::Off
            } else {
                Form::Poor
            };
            assert_eq!(form(&s), want, "{n}");
        }
    }

    #[test]
    fn factors_pick_one_of_each_sign() {
        assert_eq!(
            factors(&snap("sumo, two beers")),
            [Some(Factor::Heavy), Some(Factor::Tipsy)]
        );
        assert_eq!(
            factors(&snap("hung over")),
            [Some(Factor::HungOver), Some(Factor::Thirsty)]
        );
        let rested = derive(&Body {
            strain: 50,
            ..build("well rested")
        });
        assert_eq!(factors(&rested), [Some(Factor::Rested), Some(Factor::Sore)]);
        assert_eq!(factors(&snap("average")), [None, None]);
    }

    #[test]
    fn decisive_names_the_losers_problem() {
        let (wasted, fit) = (snap("wasted"), snap("fit, normal weight"));
        let k = (0..100u32)
            .find(|&k| !fight(k, &wasted, &fit).a_won)
            .expect("fit wins somewhere");
        let o = fight(k, &wasted, &fit);
        assert_eq!(
            decisive(&o, &wasted, &fit),
            Some((true, Decisive::Factor(Factor::Sloppy)))
        );
        let fat = snap("fat and unfit");
        assert_eq!(fat.stamina, 1);
        let k = (0..100u32)
            .find(|&k| {
                let o = fight(k, &fat, &fit);
                !o.a_won && o.rounds > 1
            })
            .expect("fit wins a long fight");
        let o = fight(k, &fat, &fit);
        assert_eq!(
            decisive(&o, &fat, &fit),
            Some((true, Decisive::OutOfSteam(2)))
        );
    }

    #[test]
    fn factor_words_fit_half_a_line() {
        for f in Factor::ALL {
            assert!(1 + f.text().len() <= 12, "{}", f.text());
        }
    }

    #[test]
    fn form_words_fit_the_row() {
        for f in [Form::Top, Form::In, Form::Off, Form::Poor] {
            assert!("Battle: ".len() + f.text().len() <= 17, "{}", f.text());
        }
    }

    #[test]
    fn form_keys_are_validated_as_a_triple() {
        use bornhack_aegg::game::engine::thresholds::{Thresholds, validate};
        let mut t = Thresholds::CLASSIC;
        t.BATTLE_FORM_OFF_PCT = 60;
        t.BATTLE_FORM_IN_PCT = 50;
        let v = validate(&t, &Thresholds::CLASSIC);
        assert_eq!(
            (
                v.BATTLE_FORM_TOP_PCT,
                v.BATTLE_FORM_IN_PCT,
                v.BATTLE_FORM_OFF_PCT
            ),
            (65, 50, 30)
        );
    }
}

mod rules {
    use bornhack_aegg::game::engine::causes::{Cause, StrainWhy};
    use bornhack_aegg::game::engine::doses::{Reason, Refusal};
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::thresholds as t;
    use bornhack_aegg::game::engine::{Fought, GameState, PetKind, Phase};

    fn pct(v: i32) -> u16 {
        (v * P) as u16
    }

    /// A fit adult: nothing hungry, tired, thirsty or rested, no strain.
    fn fit() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(t::HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s.energy = pct(80);
        s.fullness = pct(60);
        s.hydration = pct(60);
        s.fun = pct(50);
        s.dirt = pct(20);
        s.mood = pct(50);
        s.health = pct(80);
        s.weight = pct(50);
        s.fitness = pct(50);
        s.strain = 0;
        s.booze = 0;
        s
    }

    fn won(hp: u8) -> Fought {
        Fought {
            won: true,
            hp_left_pct: hp,
            costs: true,
            paid: false,
        }
    }

    #[test]
    fn refusals_follow_the_list() {
        assert_eq!(fit().battle_refusal(), None);

        let mut s = fit();
        s.hibernating = true;
        assert_eq!(s.battle_refusal(), Some(Reason::Hibernating));

        let mut s = fit();
        s.is_sleeping = true;
        assert_eq!(s.battle_refusal(), Some(Reason::Refused(Refusal::Asleep)));

        let mut s = fit();
        s.injured_min = 30;
        assert_eq!(s.battle_refusal(), Some(Reason::Refused(Refusal::Injured)));

        let mut s = fit();
        s.health = pct(29);
        assert_eq!(s.battle_refusal(), Some(Reason::Refused(Refusal::TooWeak)));
        s.health = pct(30);
        assert_eq!(s.battle_refusal(), None);

        // Felt drink: booze * (150 - weight) / 100.
        let mut s = fit();
        s.booze = pct(60);
        s.weight = pct(50);
        assert_eq!(s.battle_refusal(), None);
        s.booze = pct(50);
        s.weight = 0;
        assert_eq!(s.battle_refusal(), Some(Reason::Refused(Refusal::TooDrunk)));

        // No age limit: a pet just past hatching fights.
        let mut s = fit();
        s.age_ticks = 60;
        assert_eq!(s.battle_refusal(), None);

        let mut s = fit();
        s.cooldown_battle = 6;
        assert_eq!(s.battle_refusal(), Some(Reason::Wait(60)));
    }

    #[test]
    fn refusals_have_a_fixed_order() {
        // Asleep and injured: asleep comes first for the challenger ...
        let mut s = fit();
        s.is_sleeping = true;
        s.injured_min = 30;
        s.health = pct(10);
        s.booze = pct(90);
        s.cooldown_battle = 6;
        assert_eq!(s.battle_refusal(), Some(Reason::Refused(Refusal::Asleep)));
        // ... but the body check alone names the injury first.
        assert_eq!(s.body_refusal(), Some(Reason::Refused(Refusal::Injured)));
        s.injured_min = 0;
        assert_eq!(s.body_refusal(), Some(Reason::Refused(Refusal::TooWeak)));
        s.health = pct(80);
        assert_eq!(s.body_refusal(), Some(Reason::Refused(Refusal::TooDrunk)));
        s.booze = 0;
        assert_eq!(s.body_refusal(), None);
        // The cooldown is last.
        s.is_sleeping = false;
        assert_eq!(s.battle_refusal(), Some(Reason::Wait(60)));
    }

    #[test]
    fn injured_pet_cannot_battle() {
        let mut s = fit();
        assert!(s.stats(s.last_update_tick).can_battle);
        s.injured_min = 10;
        assert!(!s.stats(s.last_update_tick).can_battle);
    }

    #[test]
    fn a_fight_costs_what_the_spec_says() {
        let mut s = fit();
        let before = s.clone();
        assert_eq!(s.fight_over(won(80)), 0);
        assert_eq!(s.energy as i32, before.energy as i32 - 10 * P);
        assert_eq!(s.hydration as i32, before.hydration as i32 - 8 * P);
        assert_eq!(s.fun as i32, before.fun as i32 + 20 * P);
        assert_eq!(s.dirt as i32, before.dirt as i32 + 8 * P);
        assert_eq!(s.strain as i32, 20 * P);
        assert_eq!(s.mood as i32, before.mood as i32 + 8 * P);
        // (100 - 80) * 5 / 100 = 1 %
        assert_eq!(s.health as i32, before.health as i32 - P);
        assert_eq!(s.cooldown_battle, 180);
        assert_eq!(s.wins, 1);
        assert_eq!(s.losses, 0);

        let mut s = fit();
        s.fight_over(Fought {
            won: false,
            hp_left_pct: 0,
            costs: true,
            paid: false,
        });
        assert_eq!(s.mood as i32, 42 * P);
        assert_eq!(s.health as i32, 75 * P);
        assert_eq!((s.wins, s.losses), (0, 1));
    }

    #[test]
    fn a_tired_fighter_strains_more_and_can_tear() {
        let mut s = fit();
        s.fullness = pct(5);
        s.fight_over(won(100));
        assert_eq!(s.strain as i32, 30 * P);
        assert_eq!(s.injured_min, 0);

        let mut s = fit();
        s.fullness = pct(5);
        s.strain = pct(45);
        s.fight_over(won(100));
        assert_eq!(s.injured_min, t::DOSE_INJURY_MIN());
        let causes: Vec<Cause> = s.take_causes().iter().copied().collect();
        assert_eq!(causes, vec![Cause::TornMuscle(StrainWhy::Fight)]);

        let mut s = fit();
        s.rested_min = 60;
        s.fight_over(won(100));
        assert_eq!(s.strain as i32, 14 * P);
    }

    #[test]
    fn a_sore_fighter_is_told_so() {
        let mut s = fit();
        s.strain = pct(30);
        s.fight_over(won(100));
        let causes: Vec<Cause> = s.take_causes().iter().copied().collect();
        assert_eq!(causes, vec![Cause::Strained(StrainWhy::Fight)]);
        // Below the sore line nothing is said.
        let mut s = fit();
        s.fight_over(won(100));
        assert!(s.take_causes().is_empty());
    }

    #[test]
    fn sport_injury_is_unchanged() {
        let mut s = fit();
        s.strain = pct(55);
        let health = s.health;
        assert!(s.exercise());
        s.update(s.last_update_tick + t::EXERCISE_DURATION() as u32);
        assert_eq!(s.injured_min, t::DOSE_INJURY_MIN());
        assert_eq!(
            s.health as i32,
            health as i32 - t::DOSE_INJURY_HEALTH_PCT() as i32 * P
        );
        let causes: Vec<Cause> = s.take_causes().iter().copied().collect();
        assert_eq!(causes, vec![Cause::TornMuscle(StrainWhy::Overdone)]);
    }

    #[test]
    fn first_win_of_the_day_pays_100_then_20() {
        let mut s = fit();
        s.money_enabled = true;
        s.money = 0;
        s.sync_clock(s.last_update_tick, Some(20 * 60));
        let paid = Fought { paid: true, ..won(100) };
        assert_eq!(s.fight_over(paid), 100);
        assert_eq!(s.prize_min, 240);
        assert_eq!(s.fight_over(paid), 20);
        assert_eq!(s.money, 120);
        assert_eq!(s.prize_min, 240);
        // The clock runs the prize timer down; at midnight the first prize is back.
        s.update(s.last_update_tick + 240 * 6);
        assert_eq!(s.prize_min, 0);
        assert_eq!(s.fight_over(paid), 100);

        // No wall clock: a day.
        let mut s = fit();
        s.money_enabled = true;
        assert_eq!(s.fight_over(paid), 100);
        assert_eq!(s.prize_min, 1440);

        // A loss pays nothing and leaves the timer.
        let mut s = fit();
        s.money_enabled = true;
        let lost = Fought { won: false, ..paid };
        assert_eq!(s.fight_over(lost), 0);
        assert_eq!(s.prize_min, 0);
        assert_eq!(s.money, 100);
    }

    #[test]
    fn fallback_target_is_never_paid() {
        let mut s = fit();
        s.money_enabled = true;
        s.money = 50;
        let (energy, mood) = (s.energy, s.mood);
        s.record_only(true);
        assert_eq!(s.money, 50);
        assert_eq!(s.wins, 1);
        assert_eq!((s.energy, s.mood, s.cooldown_battle), (energy, mood, 0));
        s.record_only(false);
        assert_eq!((s.wins, s.losses), (1, 1));
        assert_eq!(s.prize_min, 0);
    }

    #[test]
    fn unpaid_fight_pays_nothing() {
        let mut s = fit();
        s.money_enabled = true;
        s.money = 50;
        assert_eq!(s.fight_over(won(90)), 0);
        assert_eq!(s.money, 50);
        assert_eq!(s.prize_min, 0);
        assert_eq!(s.cooldown_battle, 180);
        assert!(s.strain > 0);
    }

    #[test]
    fn a_fight_without_costs_still_rests_the_pet() {
        let mut s = fit();
        let (energy, mood) = (s.energy, s.mood);
        let f = Fought {
            costs: false,
            ..won(100)
        };
        s.fight_over(f);
        assert_eq!((s.energy, s.mood, s.strain), (energy, mood, 0));
        assert_eq!(s.cooldown_battle, 180);
        assert_eq!(s.wins, 1);
    }

    #[test]
    fn demo_fights_have_their_own_record() {
        let mut s = fit();
        s.money_enabled = true;
        s.demo.on = true;
        s.health = pct(26);
        let paid = Fought { paid: true, ..won(0) };
        assert_eq!(s.fight_over(paid), 0);
        assert_eq!((s.demo_wins, s.demo_losses), (1, 0));
        assert_eq!((s.wins, s.losses), (0, 0));
        assert_eq!(s.battle_record(), (1, 0));
        assert_eq!(s.health as i32, 25 * P, "the demo floor holds");
        s.record_only(false);
        assert_eq!(s.battle_record(), (1, 1));
        s.demo.on = false;
        assert_eq!(s.battle_record(), (0, 0));
    }

    #[test]
    fn battle_reasons_fit() {
        for r in [
            Refusal::TooWeak,
            Refusal::TooDrunk,
            Refusal::DemoOnly,
            Refusal::NoDemo,
        ] {
            let why = Reason::Refused(r);
            assert!(why.text().len() <= 16, "{}", why.text());
            let (kind, secs) = why.pack();
            assert_eq!(Reason::unpack(kind, secs), why);
        }
        assert_eq!(Refusal::TooWeak.text(), "too weak");
        assert_eq!(Refusal::TooDrunk.text(), "too drunk");
        assert_eq!(Refusal::DemoOnly.text(), "demo pets only");
        assert_eq!(Refusal::NoDemo.text(), "no demo pets");
    }

    #[test]
    fn fight_word_fits() {
        let c = Cause::TornMuscle(StrainWhy::Fight);
        assert_eq!(c.why(), "in a fight");
        assert!(c.why().len() <= 15 && c.what().len() <= 12);
        assert!(StrainWhy::ALL.contains(&StrainWhy::Fight));
        assert_eq!(Cause::unpack(c.pack()), Some(c));
        let c = Cause::Strained(StrainWhy::Fight);
        assert_eq!(Cause::unpack(c.pack()), Some(c));
    }

    #[test]
    fn battle_keys_load_and_validate() {
        use bornhack_aegg::game::bornpets_cfg::apply_overrides;
        use bornhack_aegg::game::engine::thresholds::{Thresholds, validate};
        let mut v = Thresholds::CLASSIC;
        apply_overrides(&mut v, b"BATTLE_STRAIN_NEED_PCT=150\nBATTLE_STRAIN_RESTED_PCT=70\n");
        assert_eq!((v.BATTLE_STRAIN_NEED_PCT, v.BATTLE_STRAIN_RESTED_PCT), (150, 70));
        assert_eq!(Thresholds::CASUAL.BATTLE_BRUISE_MAX_PCT, 2);
        assert_eq!(Thresholds::CLASSIC.BATTLE_BRUISE_MAX_PCT, 5);
        let mut bad = Thresholds::CLASSIC;
        bad.BATTLE_PRIZE_LATER_HAX = 101;
        bad.BATTLE_COOLDOWN = 0;
        let ok = validate(&bad, &Thresholds::CLASSIC);
        assert_eq!(
            (ok.BATTLE_PRIZE_FIRST_HAX, ok.BATTLE_PRIZE_LATER_HAX),
            (100, 20)
        );
        assert_eq!(ok.BATTLE_COOLDOWN, 1);
    }
}

mod save {
    use bornhack_aegg::game::engine::{GameState, PetKind, SAVE_SIZE};

    fn state() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.demo_wins = 0x1234;
        s.demo_losses = 0x0567;
        s.prize_min = 777;
        s.decline_min = 59;
        s
    }

    #[test]
    fn battle_fields_round_trip() {
        let s = state();
        let b = s.to_bytes();
        assert_eq!(&b[38..40], &0x1234u16.to_le_bytes());
        assert_eq!(&b[42..44], &0x0567u16.to_le_bytes());
        assert_eq!(&b[44..46], &777u16.to_le_bytes());
        assert_eq!(&b[46..48], &59u16.to_le_bytes());
        let back = GameState::from_bytes(&b).expect("decodes");
        assert_eq!(
            (back.demo_wins, back.demo_losses, back.prize_min, back.decline_min),
            (0x1234, 0x0567, 777, 59)
        );
        assert_eq!(back.to_bytes(), b);
    }

    #[test]
    fn plan3_save_loads_with_empty_battle_slots() {
        let mut b = state().to_bytes();
        // 40..42 is med_cd_min: it stays.
        b[38..40].fill(0);
        b[42..48].fill(0);
        let s = GameState::from_bytes(&b).expect("decodes");
        assert_eq!((s.demo_wins, s.demo_losses, s.prize_min, s.decline_min), (0, 0, 0, 0));
    }

    #[test]
    fn tail_stays_zero_for_plan_6() {
        let mut s = state();
        s.demo_wins = u16::MAX;
        s.demo_losses = u16::MAX;
        s.prize_min = u16::MAX;
        s.decline_min = u16::MAX;
        s.wins = u16::MAX;
        s.losses = u16::MAX;
        let b = s.to_bytes();
        assert_eq!(b.len(), SAVE_SIZE);
        assert_eq!(SAVE_SIZE, 177);
        assert_eq!(b[0], 3);
        assert!(b[160..177].iter().all(|&x| x == 0));
    }
}

mod decline {
    use bornhack_aegg::game::engine::needs::P;
    use bornhack_aegg::game::engine::regime::mood_target;
    use bornhack_aegg::game::engine::status::status_line;
    use bornhack_aegg::game::engine::thresholds as t;
    use bornhack_aegg::game::engine::{GameState, PetKind, Phase};

    fn pct(v: i32) -> u16 {
        (v * P) as u16
    }

    /// A content adult with the mood on its target.
    fn content() -> GameState {
        let mut s = GameState::new_egg(7, PetKind(0));
        s.update(t::HATCHING_TICKS() as u32);
        assert_eq!(s.phase, Phase::Active);
        s.is_sleeping = false;
        s.tick_rem = 0;
        s.money_enabled = false;
        s.energy = pct(80);
        s.fullness = pct(60);
        s.hydration = pct(60);
        s.fun = pct(50);
        s.dirt = pct(20);
        s.health = pct(80);
        s.weight = pct(50);
        s.fitness = pct(50);
        s.strain = 0;
        s.booze = 0;
        s.mood = mood_target(&s) as u16;
        s
    }

    fn rng(seed: u64) -> impl FnMut(u64) -> u64 {
        let mut r = seed;
        move |bound| {
            r = r
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (r >> 33) % (bound + 1)
        }
    }

    #[test]
    fn decline_lowers_the_mood_target_for_an_hour() {
        let mut s = content();
        let before = mood_target(&s);
        s.declined();
        assert_eq!(s.decline_min, t::BATTLE_DECLINE_MIN());
        assert_eq!(
            before - mood_target(&s),
            t::BATTLE_DECLINE_MOOD_PCT() as i32 * P
        );
        // The hour must cross no need floor, or the target moves for other reasons.
        let t0 = s.last_update_tick;
        let mut probe = content();
        probe.update(t0 + t::BATTLE_DECLINE_MIN() as u32 * 6);
        assert_eq!(mood_target(&probe), before, "fixture drifts over the hour");
        s.update(t0 + t::BATTLE_DECLINE_MIN() as u32 * 6);
        assert_eq!(s.decline_min, 0);
        assert_eq!(mood_target(&s), before);
    }

    #[test]
    fn decline_end_is_exact() {
        // (a) one update across the minute decline_min reaches 0 equals the
        // same span cut at that minute.
        let mut s = content();
        s.declined();
        s.decline_min = 5;
        let t0 = s.last_update_tick;
        let end = t0 + 5 * 6;
        let mut whole = s.clone();
        whole.update(end + 60);
        let mut cut = s.clone();
        cut.update(end);
        assert_eq!(cut.decline_min, 0);
        cut.update(end + 60);
        assert_eq!(whole.to_bytes(), cut.to_bytes());
        // One minute earlier decline_min is 1: a bound at `remaining` or none
        // at all splits the span differently and the bytes differ.
        let mut early = s.clone();
        early.update(end - 6);
        assert_eq!(early.decline_min, 1);
        early.update(end + 60);
        assert_eq!(whole.to_bytes(), early.to_bytes());
        // (b) the target term is exactly 3 * P at decline_min = 1.
        let mut a = content();
        a.decline_min = 1;
        let mut b = a.clone();
        b.decline_min = 0;
        assert_eq!(
            mood_target(&b) - mood_target(&a),
            t::BATTLE_DECLINE_MOOD_PCT() as i32 * P
        );
    }

    fn obs(s: &GameState) -> impl PartialEq + use<> {
        (
            status_line(s),
            s.display_anim(),
            s.is_sleeping,
            s.phase,
            s.active_action,
            s.onset_bits(),
        )
    }

    #[test]
    fn decline_wake_is_exact_on_random_states() {
        let mut next = rng(11);
        for i in 0..1000 {
            let mut s = content();
            s.fullness = pct(next(100) as i32);
            s.hydration = pct(next(100) as i32);
            s.fun = pct(next(100) as i32);
            s.mood = pct(next(100) as i32);
            s.decline_min = next(60) as u16;
            let t0 = s.last_update_tick;
            let o = obs(&s);
            let mut c = s.clone();
            let first = (1..=120u32).find(|m| {
                c.update(t0 + m * 6);
                obs(&c) != o
            });
            let expect = first.map_or(t0 + 120 * 6, |m| t0 + m * 6);
            assert_eq!(s.next_wake_tick(), expect, "random state {i}");
        }
    }

    #[test]
    fn decline_is_split_invariant() {
        let mut next = rng(5);
        for i in 0..200 {
            let mut s = content();
            s.mood = pct(next(100) as i32);
            s.fun = pct(next(100) as i32);
            s.decline_min = next(60) as u16;
            let t0 = s.last_update_tick;
            let span = 3 * 360u32;
            let mut whole = s.clone();
            whole.update(t0 + span);
            let mut cuts: Vec<u32> = (0..30).map(|_| t0 + next(span as u64) as u32).collect();
            cuts.sort_unstable();
            let mut cut = s.clone();
            for c in cuts {
                cut.update(c);
            }
            cut.update(t0 + span);
            assert_eq!(whole.to_bytes(), cut.to_bytes(), "seed {i}");
        }
    }
}
