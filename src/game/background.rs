//! The background behind the pet (spec 14.2 layers 1 and 2): the sky, the
//! floor items, the grass and the poorly cross. Everything is drawn from
//! primitives and small const bitmaps, no image files. The same function
//! draws into the firmware framebuffer and the simulator display.

use embedded_graphics::prelude::*;
use embedded_graphics::primitives::{PrimitiveStyle, Rectangle};

use super::engine::scene::{BackgroundState, FLOOR_ITEMS, Sky};
use crate::{BLACK, RED, TriColor, WHITE};

/// First and last sky row (the pet area starts under the separator at y 34).
pub const SKY_TOP: i32 = 35;
pub const SKY_BOTTOM: i32 = 99;
/// First and last floor row.
pub const FLOOR_TOP: i32 = 100;
pub const FLOOR_BOTTOM: i32 = 109;

pub const BAYER4: [[u8; 4]; 4] = [[0, 8, 2, 10], [12, 4, 14, 6], [3, 11, 1, 9], [15, 7, 13, 5]];
/// Black where `BAYER4[y % 4][x % 4] < level`; one level per 16-row band from
/// `SKY_TOP` (the last band runs to `SKY_BOTTOM`). Denser towards the top,
/// about 53 % black overall.
pub const NIGHT_LEVELS: [u8; 4] = [10, 9, 8, 7];
/// Rows per dither band.
const BAND_ROWS: i32 = 16;

/// Where the sun or the moon sits.
pub const SUN_MOON_BOX: Rectangle = Rectangle::new(Point::new(129, 49), Size::new(19, 19));
/// Centre of the sun and of the moon.
const SUN_MOON_CENTRE: Point = Point::new(138, 58);
/// Top-left of each night star (5 x 5).
pub const STARS: [Point; 6] = [
    Point::new(18, 44),
    Point::new(50, 40),
    Point::new(86, 46),
    Point::new(110, 62),
    Point::new(28, 74),
    Point::new(72, 68),
];

/// Slot boxes in slot order: crumbs, peel, can, sock, pizza box (bottom row
/// y 109).
pub const FLOOR_SLOTS: [Rectangle; FLOOR_ITEMS] = [
    Rectangle::new(Point::new(4, 107), Size::new(7, 3)),
    Rectangle::new(Point::new(120, 104), Size::new(11, 6)),
    Rectangle::new(Point::new(16, 104), Size::new(10, 6)),
    Rectangle::new(Point::new(136, 102), Size::new(9, 8)),
    Rectangle::new(Point::new(29, 105), Size::new(16, 5)),
];

/// Black pixels of each floor item, one `u16` per row, the leftmost column in
/// the highest of the slot's `width` bits.
const CRUMBS: [u16; 3] = [0b0100010, 0b1010100, 0b0010001];
const PEEL: [u16; 6] = [
    0b00000110000,
    0b00011001001,
    0b01100000101,
    0b10010001010,
    0b10001110010,
    0b01110000000,
];
const CAN: [u16; 6] = [
    0b0011111110,
    0b0100000001,
    0b1010011100,
    0b1010011100,
    0b0100000001,
    0b0011111110,
];
const SOCK: [u16; 8] = [
    0b011111000,
    0b010001000,
    0b010001000,
    0b010001000,
    0b010001110,
    0b010000010,
    0b010000001,
    0b001111111,
];
const PIZZA_BOX: [u16; 5] = [
    0b1111111111111111,
    0b1000000000000001,
    0b1000110011000001,
    0b1000000000000001,
    0b1111111111111111,
];
const FLOOR_BITMAPS: [&[u16]; FLOOR_ITEMS] = [&CRUMBS, &PEEL, &CAN, &SOCK, &PIZZA_BOX];

/// Left edges of the 13 grass tufts, each 5 x 4 at y 106..=109.
pub const TUFT_XS: [i32; 13] = [2, 14, 26, 38, 50, 62, 74, 86, 98, 110, 122, 134, 146];
const TUFT_Y: i32 = 106;
const TUFT_W: u32 = 5;
const TUFT_H: u32 = 4;
const TUFT: [u16; 4] = [0b10101, 0b10101, 0b01110, 0b00100];

/// Where the poorly cross sits (9 x 9); the plus inside is 7 long, 3 wide.
pub const CROSS_BOX: Rectangle = Rectangle::new(Point::new(140, 70), Size::new(9, 9));

/// Draw the background of `bg` into the pet area: sky, floor items, grass,
/// then the red cross.
pub fn draw_background<D: DrawTarget<Color = TriColor>>(
    display: &mut D,
    bg: &BackgroundState,
) -> Result<(), D::Error> {
    match bg.sky {
        Sky::Plain => {}
        Sky::Day => draw_sun(display)?,
        Sky::Night => draw_night(display)?,
    }
    draw_floor_items(display, bg.floor)?;
    if bg.grass {
        draw_grass(display, bg.floor)?;
    }
    if bg.poorly {
        draw_cross(display)?;
    }
    Ok(())
}

/// Is (`x`, `y`) inside the disc of radius `r` around `c`?
fn in_disc(x: i32, y: i32, c: Point, r: i32) -> bool {
    let (dx, dy) = (x - c.x, y - c.y);
    dx * dx + dy * dy <= r * r
}

/// A filled black disc of radius 4 and eight 2-px rays.
fn draw_sun<D: DrawTarget<Color = TriColor>>(display: &mut D) -> Result<(), D::Error> {
    let c = SUN_MOON_CENTRE;
    let disc = SUN_MOON_BOX
        .points()
        .filter(|p| in_disc(p.x, p.y, c, 4))
        .map(|p| Pixel(p, BLACK));
    // Straight rays 3..=4 px clear of the disc edge, diagonal rays on the
    // 45 degree lines.
    const RAYS: [(i32, i32); 8] = [
        (0, -1),
        (0, 1),
        (-1, 0),
        (1, 0),
        (-1, -1),
        (1, -1),
        (-1, 1),
        (1, 1),
    ];
    let rays = RAYS.iter().flat_map(move |&(dx, dy)| {
        let (near, far) = if dx != 0 && dy != 0 { (5, 6) } else { (6, 7) };
        [near, far]
            .into_iter()
            .map(move |d| Pixel(Point::new(c.x + dx * d, c.y + dy * d), BLACK))
    });
    display.draw_iter(disc.chain(rays))
}

/// The Bayer dither, the stars and the moon.
fn draw_night<D: DrawTarget<Color = TriColor>>(display: &mut D) -> Result<(), D::Error> {
    let sky = (SKY_TOP..=SKY_BOTTOM).flat_map(|y| {
        let band = (((y - SKY_TOP) / BAND_ROWS) as usize).min(NIGHT_LEVELS.len() - 1);
        let level = NIGHT_LEVELS[band];
        (0..152).map(move |x| {
            let black = BAYER4[(y % 4) as usize][(x % 4) as usize] < level;
            Pixel(Point::new(x, y), if black { BLACK } else { WHITE })
        })
    });
    display.draw_iter(sky)?;
    for s in STARS {
        let star = Rectangle::new(s, Size::new(5, 5)).points().map(|p| {
            let (dx, dy) = (p.x - s.x, p.y - s.y);
            let plus = (dx == 2 && (1..=3).contains(&dy)) || (dy == 2 && (1..=3).contains(&dx));
            Pixel(p, if plus { WHITE } else { BLACK })
        });
        display.draw_iter(star)?;
    }
    let c = SUN_MOON_CENTRE;
    let bite = Point::new(141, 56);
    let moon = SUN_MOON_BOX.points().map(|p| {
        let white = in_disc(p.x, p.y, c, 7) && !in_disc(p.x, p.y, bite, 6);
        Pixel(p, if white { WHITE } else { BLACK })
    });
    display.draw_iter(moon)
}

/// The black pixels of a bitmap `width` columns wide at `origin`.
fn bitmap<D: DrawTarget<Color = TriColor>>(
    display: &mut D,
    origin: Point,
    width: i32,
    rows: &[u16],
) -> Result<(), D::Error> {
    let px = rows.iter().enumerate().flat_map(move |(dy, bits)| {
        (0..width)
            .filter(move |dx| bits & (1 << (width - 1 - dx)) != 0)
            .map(move |dx| Pixel(origin + Point::new(dx, dy as i32), BLACK))
    });
    display.draw_iter(px)
}

/// The first `n` floor items, each in its own slot.
fn draw_floor_items<D: DrawTarget<Color = TriColor>>(
    display: &mut D,
    n: u8,
) -> Result<(), D::Error> {
    for (slot, rows) in FLOOR_SLOTS.iter().zip(FLOOR_BITMAPS).take(n as usize) {
        bitmap(display, slot.top_left, slot.size.width as i32, rows)?;
    }
    Ok(())
}

/// A tuft at every `TUFT_XS` whose box meets no shown floor item.
fn draw_grass<D: DrawTarget<Color = TriColor>>(display: &mut D, shown: u8) -> Result<(), D::Error> {
    for x in TUFT_XS {
        let tuft = Rectangle::new(Point::new(x, TUFT_Y), Size::new(TUFT_W, TUFT_H));
        let blocked = FLOOR_SLOTS
            .iter()
            .take(shown as usize)
            .any(|s| !s.intersection(&tuft).is_zero_sized());
        if !blocked {
            bitmap(display, tuft.top_left, TUFT_W as i32, &TUFT)?;
        }
    }
    Ok(())
}

/// `CROSS_BOX` white with a red plus, arms 3 px wide and 7 px long.
fn draw_cross<D: DrawTarget<Color = TriColor>>(display: &mut D) -> Result<(), D::Error> {
    CROSS_BOX
        .into_styled(PrimitiveStyle::with_fill(WHITE))
        .draw(display)?;
    let c = CROSS_BOX.center();
    for (dx, dy, w, h) in [(-3, -1, 7, 3), (-1, -3, 3, 7)] {
        Rectangle::new(c + Point::new(dx, dy), Size::new(w, h))
            .into_styled(PrimitiveStyle::with_fill(RED))
            .draw(display)?;
    }
    Ok(())
}

#[cfg(test)]
mod draw {
    use super::*;
    use crate::game::test_canvas::Canvas;

    const PET_AREA: Rectangle = Rectangle::new(Point::new(0, 35), Size::new(152, 75));

    fn states() -> impl Iterator<Item = BackgroundState> {
        [Sky::Plain, Sky::Day, Sky::Night]
            .into_iter()
            .flat_map(|sky| {
                (0..=FLOOR_ITEMS as u8).flat_map(move |floor| {
                    [false, true].into_iter().flat_map(move |grass| {
                        [false, true]
                            .into_iter()
                            .map(move |poorly| BackgroundState {
                                sky,
                                floor,
                                grass,
                                poorly,
                            })
                    })
                })
            })
    }

    fn drawn(bg: BackgroundState) -> Canvas {
        let mut c = Canvas::new();
        draw_background(&mut c, &bg).unwrap();
        c
    }

    fn state(sky: Sky, floor: u8, grass: bool, poorly: bool) -> BackgroundState {
        BackgroundState {
            sky,
            floor,
            grass,
            poorly,
        }
    }

    fn non_white(c: &Canvas, area: Rectangle) -> usize {
        area.points().filter(|p| c.get(p.x, p.y) != WHITE).count()
    }

    fn black_share(c: &Canvas, y0: i32, y1: i32, skip: &[Rectangle]) -> f32 {
        let (mut black, mut all) = (0, 0);
        for y in y0..=y1 {
            for x in 0..152 {
                let p = Point::new(x, y);
                if skip.iter().any(|r| r.contains(p)) {
                    continue;
                }
                all += 1;
                black += (c.get(x, y) == BLACK) as u32;
            }
        }
        black as f32 / all as f32
    }

    fn star_boxes() -> [Rectangle; 6] {
        STARS.map(|p| Rectangle::new(p, Size::new(5, 5)))
    }

    #[test]
    fn background_stays_in_the_pet_area() {
        for bg in states() {
            let c = drawn(bg);
            for y in 0..152 {
                for x in 0..152 {
                    if !PET_AREA.contains(Point::new(x, y)) {
                        assert_eq!(c.get(x, y), WHITE, "{bg:?} at ({x},{y})");
                    }
                }
            }
        }
    }

    #[test]
    fn plain_sky_is_white() {
        let c = drawn(state(Sky::Plain, 0, false, false));
        assert_eq!(
            non_white(&c, Rectangle::new(Point::zero(), Size::new(152, 152))),
            0
        );
    }

    #[test]
    fn night_is_about_half_black_and_denser_on_top() {
        let c = drawn(state(Sky::Night, 0, false, false));
        let skip = [SUN_MOON_BOX];
        let top = black_share(&c, 35, 50, &skip);
        let bottom = black_share(&c, 83, 99, &skip);
        assert!(top > bottom, "top {top} bottom {bottom}");
        let mut skip = star_boxes().to_vec();
        skip.push(SUN_MOON_BOX);
        let all = black_share(&c, 35, 99, &skip);
        assert!((0.45..=0.60).contains(&all), "overall {all}");
    }

    #[test]
    fn day_has_a_sun_and_night_a_moon() {
        let day = drawn(state(Sky::Day, 0, false, false));
        assert!(day.any(SUN_MOON_BOX, BLACK), "sun");
        assert_eq!(day.get(138, 58), BLACK, "disc centre");
        let sky = Rectangle::new(Point::new(0, 35), Size::new(152, 65));
        for p in sky.points().filter(|p| !SUN_MOON_BOX.contains(*p)) {
            assert_eq!(day.get(p.x, p.y), WHITE, "day sky at {p:?}");
        }
        let night = drawn(state(Sky::Night, 0, false, false));
        assert!(night.any(SUN_MOON_BOX, WHITE), "crescent");
        assert_eq!(
            night.get(SUN_MOON_BOX.top_left.x, SUN_MOON_BOX.top_left.y),
            BLACK
        );
        // The bite is dark again: the black disc inside the white one.
        assert_eq!(night.get(141, 56), BLACK);
        assert_eq!(night.get(133, 58), WHITE, "crescent body");
    }

    #[test]
    fn night_stars_are_white_plus_signs_on_black() {
        let c = drawn(state(Sky::Night, 0, false, false));
        for s in STARS {
            assert_eq!(c.get(s.x, s.y), BLACK, "corner {s:?}");
            assert_eq!(c.get(s.x + 2, s.y + 2), WHITE, "centre {s:?}");
            assert_eq!(c.get(s.x + 2, s.y + 1), WHITE);
            assert_eq!(c.get(s.x + 1, s.y + 2), WHITE);
            assert_eq!(c.get(s.x + 4, s.y + 4), BLACK);
        }
    }

    #[test]
    fn floor_items_are_cumulative_and_never_move() {
        let all = drawn(state(Sky::Plain, 5, false, false));
        for n in 0..=FLOOR_ITEMS as u8 {
            let c = drawn(state(Sky::Plain, n, false, false));
            for (k, slot) in FLOOR_SLOTS.iter().enumerate() {
                let shown = non_white(&c, *slot) > 0;
                assert_eq!(shown, (k as u8) < n, "slot {k} with {n} items");
                if shown {
                    for p in slot.points() {
                        assert_eq!(
                            c.get(p.x, p.y),
                            all.get(p.x, p.y),
                            "slot {k} moved at {p:?}"
                        );
                    }
                }
            }
            let outside = Rectangle::new(Point::new(0, 35), Size::new(152, 75));
            for p in outside.points() {
                if !FLOOR_SLOTS.iter().any(|s| s.contains(p)) {
                    assert_eq!(c.get(p.x, p.y), WHITE, "stray pixel {p:?} with {n} items");
                }
            }
        }
    }

    #[test]
    fn slots_and_cross_never_overlap() {
        for (i, a) in FLOOR_SLOTS.iter().enumerate() {
            let bb = a.bottom_right().unwrap();
            assert!(
                a.top_left.y >= FLOOR_TOP && bb.y <= FLOOR_BOTTOM,
                "slot {i}"
            );
            assert!(a.top_left.x >= 0 && bb.x <= 151, "slot {i}");
            for b in &FLOOR_SLOTS[i + 1..] {
                assert!(a.intersection(b).is_zero_sized(), "{a:?} meets {b:?}");
            }
        }
        assert!(CROSS_BOX.intersection(&SUN_MOON_BOX).is_zero_sized());
        for r in [CROSS_BOX, SUN_MOON_BOX] {
            let bb = r.bottom_right().unwrap();
            assert!(r.top_left.y >= SKY_TOP && bb.y <= SKY_BOTTOM);
        }
        // The cross and the sun sit clear of the HAX row and the battery.
        assert!(CROSS_BOX.top_left.y > 45 && SUN_MOON_BOX.top_left.y > 45);
    }

    fn tuft_rect(x: i32) -> Rectangle {
        Rectangle::new(Point::new(x, TUFT_Y), Size::new(TUFT_W, TUFT_H))
    }

    #[test]
    fn grass_steps_around_the_items() {
        let bare = drawn(state(Sky::Day, 5, false, false));
        let grassy = drawn(state(Sky::Day, 5, true, false));
        for slot in FLOOR_SLOTS {
            for p in slot.points() {
                assert_eq!(
                    grassy.get(p.x, p.y),
                    bare.get(p.x, p.y),
                    "tuft in a slot at {p:?}"
                );
            }
        }
        let free = drawn(state(Sky::Day, 0, true, false));
        assert_eq!(TUFT_XS.len(), 13);
        for x in TUFT_XS {
            assert_eq!(free.get(x, TUFT_Y), BLACK, "tuft at {x}");
            assert_eq!(free.get(x + 4, TUFT_Y + 3), WHITE);
            assert!(tuft_rect(x).bottom_right().unwrap().y <= FLOOR_BOTTOM);
        }
        // A tuft is drawn exactly when it meets no shown item.
        for n in 0..=FLOOR_ITEMS {
            let c = drawn(state(Sky::Day, n as u8, true, false));
            let bare = drawn(state(Sky::Day, n as u8, false, false));
            for x in TUFT_XS {
                let meets = FLOOR_SLOTS[..n]
                    .iter()
                    .any(|s| !s.intersection(&tuft_rect(x)).is_zero_sized());
                let differs = tuft_rect(x)
                    .points()
                    .any(|p| c.get(p.x, p.y) != bare.get(p.x, p.y));
                assert_eq!(differs, !meets, "tuft {x} with {n} items");
            }
        }
    }

    #[test]
    fn only_the_cross_is_red() {
        for bg in states() {
            let c = drawn(bg);
            let red_outside = PET_AREA
                .points()
                .any(|p| c.get(p.x, p.y) == RED && !CROSS_BOX.contains(p));
            assert!(!red_outside, "{bg:?}");
            assert_eq!(c.any(CROSS_BOX, RED), bg.poorly, "{bg:?}");
        }
        let c = drawn(state(Sky::Night, 0, false, true));
        assert_eq!(c.get(144, 74), RED, "centre");
        assert_eq!(c.get(144, 71), RED);
        assert_eq!(c.get(147, 74), RED);
        assert_eq!(
            c.get(141, 71),
            WHITE,
            "corner of the box is white, not dither"
        );
    }
}
