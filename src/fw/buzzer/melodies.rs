#![allow(dead_code)]

use super::{Note, Tone};

// BPM 100 note durations:
//   quarter       (Q)  = 600ms
//   dotted-eighth (D8) = 450ms
//   eighth        (E)  = 300ms
//   sixteenth     (S)  = 150ms
//   half          (H)  = 1200ms
//
// Original key: G minor.  Transposed DOWN one octave so every note fits
// within the C3-B4 range supported by the Buzzer driver.
//
// Rhythm confirmed from sheet music: the opening motif is
//   G(Q) G(Q) G(Q) Eb(D8) Bb(S)  — three equal quarters, then dotted-eighth + sixteenth
// NOT three dotted-quarters as commonly misremembered.
//
// P = inter-note pause inserted between consecutive same-pitch notes.
// The note before is shortened by P to keep bar length correct.

const Q:  u32 = 600;
const D8: u32 = 450;
const E:  u32 = 300;
const S:  u32 = 150;
const H:  u32 = 1200;
const P:  u32 = 50;

pub const STARTUP: &[Tone] = &[
    Tone::new(Note::A3, 120),
    Tone::new(Note::C4, 120),
    Tone::new(Note::E4, 120),
    Tone::new(Note::A4, 300),
];

// "Never Gonna Give You Up" – Rick Astley (1987)
// Chorus melody, transposed to G major to fit C3-B4 buzzer range.
// BPM ~113: RQ=530ms RE=265ms RD8=400ms RS=133ms RH=1060ms
// RP=30ms inter-note pause (tighter than Imperial March to preserve dotted rhythm)
const RQ:  u32 = 530;
const RE:  u32 = 265;
const RD8: u32 = 400;
const RS:  u32 = 133;
const RH:  u32 = 1060;
const RP:  u32 = 30;

pub const RICK_ROLL: &[Tone] = &[
    // "Never gonna give you up"
    // D4(D8) D4(S) E4(D8) C4(S) G3(E) A3(E) G3(Q)
    Tone::new(Note::D4,  RD8 - RP), Tone::new(Note::Rest, RP),
    Tone::new(Note::D4,  RS),
    Tone::new(Note::E4,  RD8),
    Tone::new(Note::C4,  RS),
    Tone::new(Note::G3,  RE),
    Tone::new(Note::A3,  RE),
    Tone::new(Note::G3,  RQ),

    // "Never gonna let you down"
    // D4(D8) D4(S) E4(D8) C4(S) G3(E) A3(H)
    Tone::new(Note::D4,  RD8 - RP), Tone::new(Note::Rest, RP),
    Tone::new(Note::D4,  RS),
    Tone::new(Note::E4,  RD8),
    Tone::new(Note::C4,  RS),
    Tone::new(Note::G3,  RE),
    Tone::new(Note::A3,  RH),

    // "Never gonna run around and desert you"
    // D4(D8) D4(S) C4(D8) A3(S) G3(E) A3(E) B3(E) G3(Q)
    Tone::new(Note::D4,  RD8 - RP), Tone::new(Note::Rest, RP),
    Tone::new(Note::D4,  RS),
    Tone::new(Note::C4,  RD8),
    Tone::new(Note::A3,  RS),
    Tone::new(Note::G3,  RE),
    Tone::new(Note::A3,  RE),
    Tone::new(Note::B3,  RE),
    Tone::new(Note::G3,  RQ),

    // "Never gonna say goodbye"
    // D4(D8) D4(S) C4(D8) A3(S) G3(E) A3(H)
    Tone::new(Note::D4,  RD8 - RP), Tone::new(Note::Rest, RP),
    Tone::new(Note::D4,  RS),
    Tone::new(Note::C4,  RD8),
    Tone::new(Note::A3,  RS),
    Tone::new(Note::G3,  RE),
    Tone::new(Note::A3,  RH),
];

pub const IMPERIAL_MARCH: &[Tone] = &[
    // ── Phrase 1 ────────────────────────────────────────────────────
    // G(Q) G(Q) G(Q) Eb(D8) Bb(S) | G(Q) Eb(D8) Bb(S) G(H)
    Tone::new(Note::G3,  Q  - P), Tone::new(Note::Rest, P), // G  quarter
    Tone::new(Note::G3,  Q  - P), Tone::new(Note::Rest, P), // G  quarter
    Tone::new(Note::G3,  Q),                                 // G  quarter
    Tone::new(Note::Ds3, D8),                                // Eb dotted-eighth
    Tone::new(Note::As3, S),                                 // Bb sixteenth
    Tone::new(Note::G3,  Q),                                 // G  quarter
    Tone::new(Note::Ds3, D8),                                // Eb dotted-eighth
    Tone::new(Note::As3, S),                                 // Bb sixteenth
    Tone::new(Note::G3,  H),                                 // G  half

    // ── Phrase 2 (a fifth higher) ────────────────────────────────────
    // D(Q) D(Q) D(Q) Eb(D8) Bb(S) | Gb(Q) Eb(D8) Bb(S) G(H)
    Tone::new(Note::D4,  Q  - P), Tone::new(Note::Rest, P), // D  quarter
    Tone::new(Note::D4,  Q  - P), Tone::new(Note::Rest, P), // D  quarter
    Tone::new(Note::D4,  Q),                                 // D  quarter
    Tone::new(Note::Ds4, D8),                                // Eb dotted-eighth
    Tone::new(Note::As3, S),                                 // Bb sixteenth
    Tone::new(Note::Fs3, Q),                                 // Gb quarter
    Tone::new(Note::Ds3, D8),                                // Eb dotted-eighth
    Tone::new(Note::As3, S),                                 // Bb sixteenth
    Tone::new(Note::G3,  H),                                 // G  half

    // ── Section B – part 1 ───────────────────────────────────────────
    // G4(Q) G3(D8) G3(S) G4(Q) F#4(D8) F4(S)
    Tone::new(Note::G4,  Q),                                 // G4 quarter
    Tone::new(Note::G3,  D8 - P), Tone::new(Note::Rest, P), // G3 dotted-eighth
    Tone::new(Note::G3,  S),                                 // G3 sixteenth
    Tone::new(Note::G4,  Q),                                 // G4 quarter
    Tone::new(Note::Fs4, D8),                                // F# dotted-eighth
    Tone::new(Note::F4,  S),                                 // F  sixteenth

    // ── Section B – part 2 ───────────────────────────────────────────
    // E(S) Eb(S) E(E) rest(E) Ab(E) Db(Q) C(D8) B(S)
    Tone::new(Note::E4,   S),                                // E  sixteenth
    Tone::new(Note::Ds4,  S),                                // Eb sixteenth
    Tone::new(Note::E4,   E),                                // E  eighth
    Tone::new(Note::Rest, E),                                // -  eighth rest
    Tone::new(Note::Gs3,  E),                                // Ab eighth
    Tone::new(Note::Cs4,  Q),                                // Db quarter
    Tone::new(Note::C4,   D8),                               // C  dotted-eighth
    Tone::new(Note::B3,   S),                                // B  sixteenth

    // ── Section B – part 3 ───────────────────────────────────────────
    // Bb(S) A(S) Bb(E) rest(E) Eb(E) Gb(Q) Eb(D8) Bb(S)
    Tone::new(Note::As3,  S),                                // Bb sixteenth
    Tone::new(Note::A3,   S),                                // A  sixteenth
    Tone::new(Note::As3,  E),                                // Bb eighth
    Tone::new(Note::Rest, E),                                // -  eighth rest
    Tone::new(Note::Ds3,  E),                                // Eb eighth
    Tone::new(Note::Fs3,  Q),                                // Gb quarter
    Tone::new(Note::Ds3,  D8),                               // Eb dotted-eighth
    Tone::new(Note::As3,  S),                                // Bb sixteenth

    // ── Phrase 1 reprise ─────────────────────────────────────────────
    // G(Q) Eb(D8) Bb(S) G(H)
    Tone::new(Note::G3,  Q),                                 // G  quarter
    Tone::new(Note::Ds3, D8),                                // Eb dotted-eighth
    Tone::new(Note::As3, S),                                 // Bb sixteenth
    Tone::new(Note::G3,  H),                                 // G  half
];
