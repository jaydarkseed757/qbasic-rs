// Test DRAW statement rendering — saves framebuffer as a text-art grid
// so we can verify car and donkey sprite shapes without a live window.
//
// Run: cargo test --test draw_test -- --nocapture

use qbasic_runtime::Runtime;

/// Render a framebuffer region as ASCII art.
/// '#' = non-background pixel, '.' = background (color 0)
fn fb_to_ascii(rt: &Runtime, x0: i32, y0: i32, w: i32, h: i32) -> String {
    let mut out = String::new();
    for row in y0..y0+h {
        for col in x0..x0+w {
            let px = rt.point(col as f64, row as f64);
            if px != 0.0 { out.push('#'); } else { out.push('.'); }
        }
        out.push('\n');
    }
    out
}

/// Render with two characters: one for each distinct non-zero color index seen.
fn fb_to_ascii2(rt: &Runtime, x0: i32, y0: i32, w: i32, h: i32) -> String {
    let mut out = String::new();
    for row in y0..y0+h {
        for col in x0..x0+w {
            let px = rt.point(col as f64, row as f64) as u8;
            out.push(match px {
                0 => '.',
                3 => '#',
                15 => '@',
                _ => '?',
            });
        }
        out.push('\n');
    }
    out
}

#[test]
fn test_draw_car_sprite_full() {
    // Replicate donkey.bas lines 1780–1930 (full sprite creation)
    let mut rt = Runtime::headless();
    rt.screen(1.0); // SCREEN 1 — 320×200
    rt.cls(0);

    // DRAW the car outline
    rt.draw("S8C3");
    rt.draw("BM12,1r3m+1,3d2R1ND2u1r2d4l2u1l1");
    rt.draw("d7R1nd2u2r3d6l3u2l1d3m-1,1l3");
    rt.draw("m-1,-1u3l1d2l3u6r3d2nd2r1u7l1d1l2");
    rt.draw("u4r2d1nd2R1U2");
    rt.draw("M+1,-3");
    rt.draw("BD10D2R3U2M-1,-1L1M-1,1");
    rt.draw("BD3D1R1U1L1BR2R1D1L1U1");
    rt.draw("BD2BL2D1R1U1L1BR2R1D1L1U1");
    rt.draw("BD2BL2D1R1U1L1BR2R1D1L1U1");

    println!("After DRAW (before LINE+PAINT), 50×55:");
    println!("{}", fb_to_ascii2(&rt, 0, 0, 50, 55));

    // Bounding box + fill (donkey.bas lines 1890–1900)
    rt.line_box(0.0, 0.0, 40.0, 60.0, 3.0); // LINE(0,0)-(40,60),,B  color=3
    rt.paint(1.0, 1.0, -1.0, -1.0);          // PAINT(1,1) — default color

    println!("After LINE+PAINT, 50×55:");
    let art = fb_to_ascii2(&rt, 0, 0, 50, 55);
    println!("{}", art);

    let drawn_3  = art.chars().filter(|&c| c == '#').count();
    let drawn_15 = art.chars().filter(|&c| c == '@').count();
    println!("Color-3 pixels: {}, Color-15 pixels: {}", drawn_3, drawn_15);
    assert_eq!(drawn_15, 0, "Color-15 (EGA white) should be 0 — fill must use draw_color(3)");
    assert!(drawn_3 > 50, "Expected filled car sprite pixels");
}

#[test]
fn test_draw_donkey_sprite_full() {
    // Replicate donkey.bas lines 1940–2050
    let mut rt = Runtime::headless();
    rt.screen(1.0);
    rt.cls(0);

    rt.draw("S08");
    rt.draw("BM14,18");
    rt.draw("M+2,-4R8M+1,-1U1M+1,+1M+2,-1");
    rt.draw("M-1,1M+1,3M-1,1M-1,-2M-1,2");
    rt.draw("D3L1U3M-1,1D2L1U2L3D2L1U2M-1,-1");
    rt.draw("D3L1U5M-2,3U1");

    // PAINT fills the donkey body (line 2010)
    rt.paint(21.0, 14.0, 3.0, 3.0);

    // PRESET punches out eyes — should set to bg_color (0) not fg_color
    rt.pset(37.0, 10.0, 0.0);   // simulating PRESET
    rt.pset(40.0, 10.0, 0.0);
    rt.pset(37.0, 11.0, 0.0);
    rt.pset(40.0, 11.0, 0.0);

    let art = fb_to_ascii2(&rt, 13, 0, 33, 26);
    println!("DONKEY SPRITE (capture region 13..45, 0..25):");
    println!("{}", art);

    let drawn = art.chars().filter(|&c| c == '#').count();
    let bg    = art.chars().filter(|&c| c == '.').count();
    println!("Filled pixels: {}, Background holes: {}", drawn, bg);
    assert!(drawn > 30, "Donkey body should be mostly filled");
}

#[test]
fn test_draw_car_sprite_outline_only() {
    let mut rt = Runtime::headless();
    rt.screen(1.0);
    rt.cls(0);

    rt.draw("S8C3");
    rt.draw("BM12,1r3m+1,3d2R1ND2u1r2d4l2u1l1");
    rt.draw("d7R1nd2u2r3d6l3u2l1d3m-1,1l3");
    rt.draw("m-1,-1u3l1d2l3u6r3d2nd2r1u7l1d1l2");
    rt.draw("u4r2d1nd2R1U2");
    rt.draw("M+1,-3");
    rt.draw("BD10D2R3U2M-1,-1L1M-1,1");
    rt.draw("BD3D1R1U1L1BR2R1D1L1U1");
    rt.draw("BD2BL2D1R1U1L1BR2R1D1L1U1");
    rt.draw("BD2BL2D1R1U1L1BR2R1D1L1U1");

    let drawn = fb_to_ascii(&rt, 0, 0, 50, 55).chars().filter(|&c| c == '#').count();
    assert!(drawn > 20, "Car outline should have >20 pixels, got {}", drawn);
}

// ── MML sound tests ───────────────────────────────────────────────────────────

#[test]
fn test_mml_parse_basic() {
    use qbasic_runtime::sound_test_helpers::*;

    // Default state: O4, L4, T120
    let mut state = default_mml_state();

    // "O4L4A" — A4, quarter note at 120 BPM = 500ms
    let events = parse_mml_test("O4L4A", &mut state);
    assert_eq!(events.len(), 2); // note + rest (MN style: 7/8 sound, 1/8 rest)
    let (freq, dur) = events[0];
    assert!((freq - 440.0).abs() < 1.0, "A4 should be ~440 Hz, got {}", freq);
    assert!((dur as f64 - 437.5).abs() < 5.0, "Quarter at 120 BPM ≈ 437ms sound, got {}", dur);

    // "O4L4A" played again — state preserved, no MB flag
    assert!(!state.background);

    // "MBT120O0L32E" — background mode, O0, L32, E note
    let events = parse_mml_test("MBT120O0L32E", &mut state);
    assert!(state.background, "MB should set background");
    assert_eq!(state.octave, 0);
    assert_eq!(state.length, 32);
    assert!(!events.is_empty());
    let (freq, _) = events[0];
    // E0 should be very low (~20.6 Hz)
    assert!(freq < 30.0, "E0 should be <30 Hz, got {}", freq);
}

#[test]
fn test_mml_parse_gorilla_explosion() {
    use qbasic_runtime::sound_test_helpers::*;

    // First explosion line from gorillas.bas
    let mut state = default_mml_state();
    let events = parse_mml_test("t120o1l16b9n0baan0bn0bn0baaan0b9n0baan0b", &mut state);
    assert!(events.len() > 10, "Explosion should have many notes");

    // State should carry forward (tempo=120, octave=1, length=16)
    assert_eq!(state.tempo as i32, 120);
    assert_eq!(state.octave, 1);
    assert_eq!(state.length, 16);
}
