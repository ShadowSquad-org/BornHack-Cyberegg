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
