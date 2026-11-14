//! The pet's condition as the fight sees it (plan 5, spec 15.7 and 18.4).
//!
//! [`Body`] is the pet in whole percent; [`derive`] turns it into a
//! [`Snapshot`]: six fighting numbers and twenty condition flags. Port of
//! the prototype's `battle::derive`, integer math with truncating division.
//! The balance numbers and the need bands are read from `thresholds` (so a
//! Casual or custom band moves the fight with it); the slopes and divisors
//! of the formula are structure and stay literals.

use super::needs::{self, SORE_PCT};
use super::{GameState, regime, thresholds as t, trait_pct};

/// The pet as the fight sees it, every field in whole percent (traits 0..=100).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Body {
    pub fit: i32,
    pub energy: i32,
    pub booze: i32,
    pub mood: i32,
    pub health: i32,
    pub strain: i32,
    pub full: i32,
    pub water: i32,
    pub fun: i32,
    pub dirt: i32,
    pub weight: i32,
    pub vit: i32,
    pub cur: i32,
    pub res: i32,
    pub rested: bool,
    pub injured: bool,
    pub groggy: bool,
    pub hangover: bool,
    pub craving: bool,
    pub deprived: bool,
    pub demo: bool,
}

impl Body {
    /// The prototype's average pet (`Cond::BASE`).
    pub const REFERENCE: Body = Body {
        fit: 60,
        energy: 70,
        booze: 0,
        mood: 75,
        health: 85,
        strain: 0,
        full: 55,
        water: 60,
        fun: 50,
        dirt: 30,
        weight: 50,
        vit: 50,
        cur: 50,
        res: 50,
        rested: false,
        injured: false,
        groggy: false,
        hangover: false,
        craving: false,
        deprived: false,
        demo: false,
    };

    /// The pet's state now: each level as whole percent, the traits as
    /// percent, a flag for each running timer.
    pub fn of(s: &GameState) -> Body {
        let pct = |v: u16| needs::pct(v as i32).min(100);
        Body {
            fit: pct(s.fitness),
            energy: pct(s.energy),
            booze: pct(s.booze),
            mood: pct(s.mood),
            health: pct(s.health),
            strain: pct(s.strain),
            full: pct(s.fullness),
            water: pct(s.hydration),
            fun: pct(s.fun),
            dirt: pct(s.dirt),
            weight: pct(s.weight),
            vit: trait_pct(s.vitality) as i32,
            cur: trait_pct(s.curiosity) as i32,
            res: trait_pct(s.resilience) as i32,
            rested: s.rested_min > 0,
            injured: s.injured_min > 0,
            groggy: s.groggy_min > 0,
            hangover: s.hangover_min > 0,
            craving: regime::craving(s),
            deprived: s.deprived,
            demo: s.demo.on,
        }
    }
}

/// The twenty conditions the pre-battle screen names. The bit of a factor in
/// [`Flags`] is its number; declaration order is the display priority within
/// each sign.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Factor {
    // plus (helps)
    Rested,
    Heavy,
    Tipsy,
    Cheerful,
    // minus (hurts)
    Injured,
    Sloppy,
    HungOver,
    TooLight,
    Hungry,
    Thirsty,
    Craving,
    Wired,
    Stuffed,
    Bored,
    Down,
    Groggy,
    WornOut,
    Waterlogged,
    Sore,
    Filthy,
}

impl Factor {
    pub const ALL: [Factor; 20] = [
        Factor::Rested,
        Factor::Heavy,
        Factor::Tipsy,
        Factor::Cheerful,
        Factor::Injured,
        Factor::Sloppy,
        Factor::HungOver,
        Factor::TooLight,
        Factor::Hungry,
        Factor::Thirsty,
        Factor::Craving,
        Factor::Wired,
        Factor::Stuffed,
        Factor::Bored,
        Factor::Down,
        Factor::Groggy,
        Factor::WornOut,
        Factor::Waterlogged,
        Factor::Sore,
        Factor::Filthy,
    ];

    /// The first four factors help the pet; the rest hurt it.
    pub fn helps(self) -> bool {
        (self as u8) < 4
    }

    /// The on-screen word (at most 11 columns, so `+word` fits half a line).
    pub fn text(self) -> &'static str {
        match self {
            Factor::Rested => "well rested",
            Factor::Heavy => "heavy",
            Factor::Tipsy => "tipsy",
            Factor::Cheerful => "cheerful",
            Factor::Injured => "injured",
            Factor::Sloppy => "sloppy",
            Factor::HungOver => "hung over",
            Factor::TooLight => "too light",
            Factor::Hungry => "hungry",
            Factor::Thirsty => "thirsty",
            Factor::Craving => "craving",
            Factor::Wired => "wired",
            Factor::Stuffed => "stuffed",
            Factor::Bored => "bored",
            Factor::Down => "down",
            Factor::Groggy => "groggy",
            Factor::WornOut => "worn out",
            Factor::Waterlogged => "waterlogged",
            Factor::Sore => "sore",
            Factor::Filthy => "filthy",
        }
    }
}

/// Bit set of [`Factor`]s: bit = the factor's number (0..=19), bit 22 is
/// reserved for the star (plan 8, always 0 here), bit 23 is the demo bit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Flags(pub u32);

const DEMO_BIT: u32 = 1 << 23;

impl Flags {
    pub fn has(self, f: Factor) -> bool {
        self.0 >> (f as u8) & 1 == 1
    }

    pub fn demo(self) -> bool {
        self.0 & DEMO_BIT != 0
    }

    pub fn with(self, f: Factor) -> Flags {
        Flags(self.0 | 1 << (f as u8))
    }

    pub fn with_demo(self, on: bool) -> Flags {
        Flags(if on {
            self.0 | DEMO_BIT
        } else {
            self.0 & !DEMO_BIT
        })
    }
}

/// Six fighting numbers and the condition flags. This is what travels in a
/// challenge; both badges run the fight from two of these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub power: u8,
    pub guard: u8,
    pub speed: u8,
    pub hp: u8,
    pub stamina: u8,
    pub focus: i8,
    pub flags: Flags,
}

/// Inclusive wire ranges. `derive` clamps into them; a received snapshot
/// outside them is refused (Task 5).
pub const WIRE_POWER: (u8, u8) = (5, 70);
pub const WIRE_GUARD: (u8, u8) = (0, 40);
pub const WIRE_SPEED: (u8, u8) = (5, 100);
pub const WIRE_HP: (u8, u8) = (15, 120);
pub const WIRE_STAMINA: (u8, u8) = (0, 10);
pub const WIRE_FOCUS: (i8, i8) = (-100, 20);

/// How much of the drink the pet feels: a light pet feels more of it.
/// `booze * (base - weight) / 100`, at most 100. The one copy of the rule;
/// the snapshot, its `Tipsy` and `Sloppy` flags and the too-drunk refusal
/// all read it.
pub fn felt(b: &Body) -> i32 {
    (b.booze * (t::BATTLE_FELT_BASE_PCT() as i32 - b.weight) / 100).min(100)
}

/// Liquid courage: a tent over the felt drink, its peak at 30, gone at 60.
fn courage(load: i32) -> i32 {
    let peak = t::BATTLE_COURAGE_PEAK_PTS() as i32;
    if load <= 30 {
        load * peak / 30
    } else {
        (peak - (load - 30) * peak / 30).max(0)
    }
}

/// The prototype's `derive`: `[power, guard, speed, hp, stamina, focus]`
/// with the prototype's floors, before the wire clamp. Only focus has no
/// floor, so only focus can leave its wire range.
pub fn derive_raw(b: &Body) -> [i32; 6] {
    let pts = |v: u8| v as i32;
    let load = felt(b);
    let mut focus_adj = 0;
    let courage = courage(load);

    // POWER: mass, courage, mood, soreness.
    let mut power = pts(t::BATTLE_POWER_BASE_PTS())
        + b.weight * t::BATTLE_POWER_WEIGHT_PCT() as i32 / 100
        + courage;
    let low_mood = t::BATTLE_LOW_MOOD_PCT() as i32;
    if b.mood < low_mood {
        power -= (low_mood - b.mood) / 3;
    } else if b.mood >= t::BATTLE_HIGH_MOOD_PCT() as i32 {
        power += pts(t::BATTLE_HIGH_MOOD_PTS());
    }
    let sore = SORE_PCT as i32;
    if b.strain > sore {
        power -= (b.strain - sore) / 4;
    }

    // GUARD: resilience and mass; a felt drink makes it reckless.
    let guard = b.res / 5 + b.weight * t::BATTLE_GUARD_WEIGHT_PCT() as i32 / 100
        - pts(t::BATTLE_GUARD_BASE_PTS())
        - load / 8;

    // SPEED: fitness, vitality, belly, sleep state. Heavy is slow, light is quick.
    let mut speed = pts(t::BATTLE_SPEED_BASE_PTS())
        + b.fit * t::BATTLE_SPEED_FIT_PCT() as i32 / 100
        + b.vit / 5
        - (b.weight - 50).max(0) * 3 / 5
        + (50 - b.weight).max(0) / 5;
    let full_hi = t::FULL_HI_PCT() as i32;
    if b.full > full_hi {
        speed -= (b.full - full_hi) * 3 / 4 + 4;
    }
    let water_hi = t::WATER_HI_PCT() as i32;
    if b.water > water_hi {
        speed -= (b.water - water_hi) / 2 + 3;
    }
    if b.rested {
        speed += pts(t::BATTLE_RESTED_SPEED_PTS());
    }
    if b.groggy {
        speed -= pts(t::BATTLE_GROGGY_SPEED_PTS());
    }

    // STAMINA: rounds at full strength. Energy times fitness, vitality, belly.
    let mut stamina = 1 + b.energy * (40 + b.fit) / 2600 + (b.vit - 50) / 25;
    let heavy = b.weight > t::WEIGHT_HI_PCT() as i32;
    if heavy {
        stamina -= 1; // excess weight tires
    }
    if b.deprived {
        stamina -= 1;
    }
    if heavy && b.fit < t::BATTLE_HEAVY_UNFIT_FIT_PCT() as i32 {
        stamina -= 1; // and tires an unfit pet twice
    }
    let weight_lo = t::WEIGHT_LO_PCT() as i32;
    if b.weight < weight_lo {
        power -= (weight_lo - b.weight) / 2; // too light to hurt
    }
    if b.hangover {
        speed -= pts(t::BATTLE_HANGOVER_SPEED_PTS());
        focus_adj = -pts(t::BATTLE_HANGOVER_FOCUS_PTS());
        stamina -= 1;
    }
    if b.craving {
        focus_adj -= pts(t::BATTLE_CRAVING_FOCUS_PTS());
        power -= pts(t::BATTLE_CRAVING_POWER_PTS());
    }
    let full_lo = t::FULL_LO_PCT() as i32;
    if b.full < full_lo {
        stamina -= 1 + (full_lo - b.full) / 15;
    }
    let water_lo = t::WATER_LO_PCT() as i32;
    if b.water < water_lo {
        stamina -= 1 + (water_lo - b.water) / 15;
    }
    if b.rested {
        stamina += 1;
    }

    // FOCUS (hit-chance points): stimulation band, drink, curiosity, mood, grime.
    let mut focus = (b.cur - 50) / 5 + focus_adj;
    let fun_lo = t::FUN_LO_PCT() as i32;
    if b.fun < fun_lo {
        focus -= (fun_lo - b.fun) / 2; // bored: not into it
    }
    let fun_hi = t::FUN_HI_PCT() as i32;
    if b.fun > fun_hi {
        focus -= b.fun - fun_hi; // wired: wild swings
    }
    focus -= load / 6; // any drink costs a little aim
    let sloppy = t::BATTLE_SLOPPY_PCT() as i32;
    if load > sloppy {
        focus -= (load - sloppy) * 5 / 4;
    }
    if b.mood < low_mood {
        focus -= (low_mood - b.mood) / 4;
    }
    let dirt_hi = t::DIRT_HI_PCT() as i32;
    if b.dirt > dirt_hi {
        focus -= (b.dirt - dirt_hi) / 4;
    }
    if b.water < water_lo {
        focus -= (water_lo - b.water) / 3;
    }
    if b.groggy {
        focus -= pts(t::BATTLE_GROGGY_FOCUS_PTS());
    }
    if b.rested {
        focus += pts(t::BATTLE_RESTED_FOCUS_PTS());
    }

    // HP: health, vitality, mass.
    let hp = (pts(t::BATTLE_HP_BASE_PTS()) + b.vit / 2) * b.health / 100
        + b.weight * t::BATTLE_HP_WEIGHT_PCT() as i32 / 100
        - 3;

    // Puncher's chance: nobody is entirely harmless (owner decisions 18 and 23).
    let mut power = power.max(pts(t::BATTLE_POWER_FLOOR_PTS()));
    let hp = hp.max(pts(t::BATTLE_HP_FLOOR_PTS()));
    let stamina = stamina.max(pts(t::BATTLE_STAMINA_FLOOR_PTS()));
    let guard = guard.max(pts(t::BATTLE_GUARD_FLOOR_PTS()));
    let mut speed = speed;
    if b.injured {
        let pct = t::BATTLE_INJURED_PCT() as i32;
        power = power * pct / 100;
        speed = speed * pct / 100;
    }
    [
        power.max(5),
        guard.max(0),
        speed.max(5),
        hp.max(15),
        stamina.max(0),
        focus,
    ]
}

/// The condition flags of a body.
fn flags_of(b: &Body, load: i32) -> Flags {
    let (full_lo, full_hi) = (t::FULL_LO_PCT() as i32, t::FULL_HI_PCT() as i32);
    let (water_lo, water_hi) = (t::WATER_LO_PCT() as i32, t::WATER_HI_PCT() as i32);
    let (fun_lo, fun_hi) = (t::FUN_LO_PCT() as i32, t::FUN_HI_PCT() as i32);
    let sloppy = t::BATTLE_SLOPPY_PCT() as i32;
    let on = [
        (Factor::Rested, b.rested),
        (Factor::Heavy, b.weight > t::WEIGHT_HI_PCT() as i32),
        (Factor::Tipsy, courage(load) > 0 && load <= sloppy),
        (Factor::Cheerful, b.mood >= t::BATTLE_HIGH_MOOD_PCT() as i32),
        (Factor::Injured, b.injured),
        (Factor::Sloppy, load > sloppy),
        (Factor::HungOver, b.hangover),
        (Factor::TooLight, b.weight < t::WEIGHT_LO_PCT() as i32),
        (Factor::Hungry, b.full < full_lo),
        (Factor::Thirsty, b.water < water_lo),
        (Factor::Craving, b.craving),
        (Factor::Wired, b.fun > fun_hi),
        (Factor::Stuffed, b.full > full_hi),
        (Factor::Bored, b.fun < fun_lo),
        (Factor::Down, b.mood < t::BATTLE_LOW_MOOD_PCT() as i32),
        (Factor::Groggy, b.groggy),
        (Factor::WornOut, b.deprived),
        (Factor::Waterlogged, b.water > water_hi),
        (Factor::Sore, b.strain > SORE_PCT as i32),
        (Factor::Filthy, b.dirt > t::DIRT_HI_PCT() as i32),
    ];
    let mut flags = Flags::default().with_demo(b.demo);
    for (f, set) in on {
        if set {
            flags = flags.with(f);
        }
    }
    flags
}

/// The snapshot of a body: [`derive_raw`] clamped into the wire ranges,
/// plus the condition flags.
pub fn derive(b: &Body) -> Snapshot {
    let r = derive_raw(b);
    let wire = |v: i32, (lo, hi): (u8, u8)| v.clamp(lo as i32, hi as i32) as u8;
    Snapshot {
        power: wire(r[0], WIRE_POWER),
        guard: wire(r[1], WIRE_GUARD),
        speed: wire(r[2], WIRE_SPEED),
        hp: wire(r[3], WIRE_HP),
        stamina: wire(r[4], WIRE_STAMINA),
        focus: r[5].clamp(WIRE_FOCUS.0 as i32, WIRE_FOCUS.1 as i32) as i8,
        flags: flags_of(b, felt(b)),
    }
}
