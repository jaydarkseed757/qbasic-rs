//! QBasic runtime library — linked by every transpiled program.
#![allow(non_snake_case, dead_code, unused_variables)]

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};
use std::io::{BufRead, Write, Seek, SeekFrom};
use minifb::{Key, KeyRepeat};

mod sound;
use sound::MmlState;

/// Helpers for integration tests — parse MML without requiring an audio device.
pub mod sound_test_helpers {
    use crate::sound::{MmlState, parse_mml_for_test};
    pub fn default_mml_state() -> MmlState { MmlState::default() }
    pub fn parse_mml_test(mml: &str, state: &mut MmlState) -> Vec<(f32, u64)> {
        parse_mml_for_test(mml, state)
    }
}

// ── IBM PC 8×8 bitmap font (CP437, all 256 characters) ───────────────────────
// Each entry is 8 bytes: one byte per row, MSB = leftmost pixel.
// Source: classic IBM PC BIOS / CP437 character ROM (public domain).
// draw_char_fb uses (ch as u32 & 0xFF) as the index — so Latin-1–encoded
// source files (byte N stored as U+00N0) map directly to the CP437 glyph.
static FONT_8X8: [[u8; 8]; 256] = [
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00], // 0x00 NUL
    [0x7E,0x81,0xA5,0x81,0xBD,0x99,0x81,0x7E], // 0x01
    [0x7E,0xFF,0xDB,0xFF,0xC3,0xE7,0xFF,0x7E], // 0x02
    [0x6C,0xFE,0xFE,0xFE,0x7C,0x38,0x10,0x00], // 0x03
    [0x10,0x38,0x7C,0xFE,0x7C,0x38,0x10,0x00], // 0x04
    [0x38,0x7C,0x38,0xFE,0xFE,0xD6,0x10,0x38], // 0x05
    [0x10,0x10,0x38,0x7C,0xFE,0x7C,0x10,0x38], // 0x06
    [0x00,0x00,0x18,0x3C,0x3C,0x18,0x00,0x00], // 0x07
    [0xFF,0xFF,0xE7,0xC3,0xC3,0xE7,0xFF,0xFF], // 0x08
    [0x00,0x3C,0x66,0x42,0x42,0x66,0x3C,0x00], // 0x09
    [0xFF,0xC3,0x99,0xBD,0xBD,0x99,0xC3,0xFF], // 0x0A
    [0x0F,0x07,0x0F,0x7D,0xCC,0xCC,0xCC,0x78], // 0x0B
    [0x3C,0x66,0x66,0x66,0x3C,0x18,0x7E,0x18], // 0x0C
    [0x3F,0x33,0x3F,0x30,0x30,0x70,0xF0,0xE0], // 0x0D
    [0x1E,0x36,0x1E,0x36,0x66,0x66,0x3C,0x00], // 0x0E ♫
    [0x99,0x5A,0x3C,0xE7,0xE7,0x3C,0x5A,0x99], // 0x0F
    [0x80,0xE0,0xF8,0xFE,0xF8,0xE0,0x80,0x00], // 0x10
    [0x02,0x0E,0x3E,0xFE,0x3E,0x0E,0x02,0x00], // 0x11
    [0x18,0x3C,0x7E,0x18,0x18,0x7E,0x3C,0x18], // 0x12
    [0x66,0x66,0x66,0x66,0x66,0x00,0x66,0x00], // 0x13
    [0x7F,0xDB,0xDB,0x7B,0x1B,0x1B,0x1B,0x00], // 0x14
    [0x3E,0x63,0x38,0x6C,0x6C,0x38,0xCC,0x78], // 0x15
    [0x00,0x00,0x00,0x00,0x7E,0x7E,0x7E,0x00], // 0x16
    [0x18,0x3C,0x7E,0x18,0x7E,0x3C,0x18,0xFF], // 0x17
    [0x18,0x3C,0x7E,0x18,0x18,0x18,0x18,0x00], // 0x18
    [0x18,0x18,0x18,0x18,0x7E,0x3C,0x18,0x00], // 0x19
    [0x00,0x18,0x0C,0xFE,0x0C,0x18,0x00,0x00], // 0x1A
    [0x00,0x30,0x60,0xFE,0x60,0x30,0x00,0x00], // 0x1B
    [0x00,0x00,0xC0,0xC0,0xC0,0xFE,0x00,0x00], // 0x1C
    [0x00,0x24,0x66,0xFF,0x66,0x24,0x00,0x00], // 0x1D
    [0x00,0x18,0x3C,0x7E,0xFF,0xFF,0x00,0x00], // 0x1E
    [0x00,0xFF,0xFF,0x7E,0x3C,0x18,0x00,0x00], // 0x1F
    // ── Printable ASCII 0x20..0x7F ─────────────────────────────────────────────
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00], // 0x20 ' '
    [0x18,0x18,0x18,0x18,0x18,0x00,0x18,0x00], // 0x21 '!'
    [0x66,0x66,0x24,0x00,0x00,0x00,0x00,0x00], // 0x22 '"'
    [0x36,0x36,0x7F,0x36,0x7F,0x36,0x36,0x00], // 0x23 '#'
    [0x0C,0x3E,0x03,0x1E,0x30,0x1F,0x0C,0x00], // 0x24 '$'
    [0x60,0x63,0x30,0x18,0x0C,0x63,0x03,0x00], // 0x25 '%'
    [0x1C,0x36,0x36,0x1C,0x7B,0x33,0x7B,0x00], // 0x26 '&'
    [0x06,0x06,0x0C,0x00,0x00,0x00,0x00,0x00], // 0x27 '\''
    [0x06,0x0C,0x18,0x18,0x18,0x0C,0x06,0x00], // 0x28 '('
    [0x18,0x0C,0x06,0x06,0x06,0x0C,0x18,0x00], // 0x29 ')'
    [0x00,0x66,0x3C,0xFF,0x3C,0x66,0x00,0x00], // 0x2A '*'
    [0x00,0x0C,0x0C,0x3F,0x0C,0x0C,0x00,0x00], // 0x2B '+'
    [0x00,0x00,0x00,0x00,0x00,0x0C,0x0C,0x06], // 0x2C ','
    [0x00,0x00,0x00,0x3F,0x00,0x00,0x00,0x00], // 0x2D '-'
    [0x00,0x00,0x00,0x00,0x00,0x18,0x18,0x00], // 0x2E '.'
    [0x00,0x03,0x06,0x0C,0x18,0x30,0x60,0x00], // 0x2F '/'
    [0x3C,0x66,0x6E,0x76,0x66,0x66,0x3C,0x00], // 0x30 '0'
    [0x18,0x38,0x18,0x18,0x18,0x18,0x7E,0x00], // 0x31 '1'
    [0x3C,0x66,0x06,0x1C,0x30,0x60,0x7E,0x00], // 0x32 '2'
    [0x3C,0x66,0x06,0x1C,0x06,0x66,0x3C,0x00], // 0x33 '3'
    [0x0E,0x1E,0x36,0x66,0x7F,0x06,0x0F,0x00], // 0x34 '4'
    [0x7E,0x60,0x7C,0x06,0x06,0x66,0x3C,0x00], // 0x35 '5'
    [0x1C,0x30,0x60,0x7C,0x66,0x66,0x3C,0x00], // 0x36 '6'
    [0x7E,0x66,0x06,0x0C,0x18,0x18,0x18,0x00], // 0x37 '7'
    [0x3C,0x66,0x66,0x3C,0x66,0x66,0x3C,0x00], // 0x38 '8'
    [0x3C,0x66,0x66,0x3E,0x06,0x0C,0x38,0x00], // 0x39 '9'
    [0x00,0x18,0x18,0x00,0x00,0x18,0x18,0x00], // 0x3A ':'
    [0x00,0x18,0x18,0x00,0x00,0x18,0x18,0x30], // 0x3B ';'
    [0x06,0x0C,0x18,0x30,0x18,0x0C,0x06,0x00], // 0x3C '<'
    [0x00,0x00,0x3F,0x00,0x00,0x3F,0x00,0x00], // 0x3D '='
    [0x60,0x30,0x18,0x0C,0x18,0x30,0x60,0x00], // 0x3E '>'
    [0x3C,0x66,0x06,0x0C,0x0C,0x00,0x0C,0x00], // 0x3F '?'
    [0x3C,0x66,0x6E,0x6A,0x6E,0x60,0x3C,0x00], // 0x40 '@'
    [0x18,0x3C,0x66,0x66,0x7E,0x66,0x66,0x00], // 0x41 'A'
    [0x7C,0x66,0x66,0x7C,0x66,0x66,0x7C,0x00], // 0x42 'B'
    [0x3C,0x66,0x60,0x60,0x60,0x66,0x3C,0x00], // 0x43 'C'
    [0x78,0x6C,0x66,0x66,0x66,0x6C,0x78,0x00], // 0x44 'D'
    [0x7E,0x60,0x60,0x7C,0x60,0x60,0x7E,0x00], // 0x45 'E'
    [0x7E,0x60,0x60,0x7C,0x60,0x60,0x60,0x00], // 0x46 'F'
    [0x3C,0x66,0x60,0x6E,0x66,0x66,0x3C,0x00], // 0x47 'G'
    [0x66,0x66,0x66,0x7E,0x66,0x66,0x66,0x00], // 0x48 'H'
    [0x3C,0x18,0x18,0x18,0x18,0x18,0x3C,0x00], // 0x49 'I'
    [0x1E,0x0C,0x0C,0x0C,0x0C,0x6C,0x38,0x00], // 0x4A 'J'
    [0x66,0x6C,0x78,0x70,0x78,0x6C,0x66,0x00], // 0x4B 'K'
    [0x60,0x60,0x60,0x60,0x60,0x60,0x7E,0x00], // 0x4C 'L'
    [0x63,0x77,0x7F,0x6B,0x63,0x63,0x63,0x00], // 0x4D 'M'
    [0x66,0x76,0x7E,0x7E,0x6E,0x66,0x66,0x00], // 0x4E 'N'
    [0x3C,0x66,0x66,0x66,0x66,0x66,0x3C,0x00], // 0x4F 'O'
    [0x7C,0x66,0x66,0x7C,0x60,0x60,0x60,0x00], // 0x50 'P'
    [0x3C,0x66,0x66,0x66,0x6E,0x3C,0x0E,0x00], // 0x51 'Q'
    [0x7C,0x66,0x66,0x7C,0x6C,0x66,0x66,0x00], // 0x52 'R'
    [0x3C,0x66,0x60,0x3C,0x06,0x66,0x3C,0x00], // 0x53 'S'
    [0x7E,0x18,0x18,0x18,0x18,0x18,0x18,0x00], // 0x54 'T'
    [0x66,0x66,0x66,0x66,0x66,0x66,0x3C,0x00], // 0x55 'U'
    [0x66,0x66,0x66,0x66,0x66,0x3C,0x18,0x00], // 0x56 'V'
    [0x63,0x63,0x63,0x6B,0x7F,0x77,0x63,0x00], // 0x57 'W'
    [0x66,0x66,0x3C,0x18,0x3C,0x66,0x66,0x00], // 0x58 'X'
    [0x66,0x66,0x66,0x3C,0x18,0x18,0x18,0x00], // 0x59 'Y'
    [0x7E,0x06,0x0C,0x18,0x30,0x60,0x7E,0x00], // 0x5A 'Z'
    [0x3C,0x30,0x30,0x30,0x30,0x30,0x3C,0x00], // 0x5B '['
    [0x00,0x60,0x30,0x18,0x0C,0x06,0x03,0x00], // 0x5C '\\'
    [0x3C,0x0C,0x0C,0x0C,0x0C,0x0C,0x3C,0x00], // 0x5D ']'
    [0x18,0x3C,0x66,0x00,0x00,0x00,0x00,0x00], // 0x5E '^'
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0xFF], // 0x5F '_'
    [0x18,0x18,0x0C,0x00,0x00,0x00,0x00,0x00], // 0x60 '`'
    [0x00,0x00,0x3C,0x06,0x3E,0x66,0x3E,0x00], // 0x61 'a'
    [0x60,0x60,0x7C,0x66,0x66,0x66,0x7C,0x00], // 0x62 'b'
    [0x00,0x00,0x3C,0x66,0x60,0x66,0x3C,0x00], // 0x63 'c'
    [0x06,0x06,0x3E,0x66,0x66,0x66,0x3E,0x00], // 0x64 'd'
    [0x00,0x00,0x3C,0x66,0x7E,0x60,0x3C,0x00], // 0x65 'e'
    [0x1C,0x30,0x7E,0x30,0x30,0x30,0x30,0x00], // 0x66 'f'
    [0x00,0x00,0x3E,0x66,0x66,0x3E,0x06,0x3C], // 0x67 'g'
    [0x60,0x60,0x7C,0x66,0x66,0x66,0x66,0x00], // 0x68 'h'
    [0x18,0x00,0x38,0x18,0x18,0x18,0x3C,0x00], // 0x69 'i'
    [0x06,0x00,0x06,0x06,0x06,0x06,0x6C,0x38], // 0x6A 'j'
    [0x60,0x60,0x66,0x6C,0x78,0x6C,0x66,0x00], // 0x6B 'k'
    [0x38,0x18,0x18,0x18,0x18,0x18,0x3C,0x00], // 0x6C 'l'
    [0x00,0x00,0x66,0x7F,0x7F,0x6B,0x63,0x00], // 0x6D 'm'
    [0x00,0x00,0x7C,0x66,0x66,0x66,0x66,0x00], // 0x6E 'n'
    [0x00,0x00,0x3C,0x66,0x66,0x66,0x3C,0x00], // 0x6F 'o'
    [0x00,0x00,0x7C,0x66,0x66,0x7C,0x60,0x60], // 0x70 'p'
    [0x00,0x00,0x3E,0x66,0x66,0x3E,0x06,0x06], // 0x71 'q'
    [0x00,0x00,0x6C,0x76,0x60,0x60,0x60,0x00], // 0x72 'r'
    [0x00,0x00,0x3C,0x60,0x3C,0x06,0x7C,0x00], // 0x73 's'
    [0x30,0x30,0x7C,0x30,0x30,0x34,0x18,0x00], // 0x74 't'
    [0x00,0x00,0x66,0x66,0x66,0x66,0x3E,0x00], // 0x75 'u'
    [0x00,0x00,0x66,0x66,0x66,0x3C,0x18,0x00], // 0x76 'v'
    [0x00,0x00,0x63,0x6B,0x7F,0x3E,0x36,0x00], // 0x77 'w'
    [0x00,0x00,0x66,0x3C,0x18,0x3C,0x66,0x00], // 0x78 'x'
    [0x00,0x00,0x66,0x66,0x66,0x3E,0x06,0x3C], // 0x79 'y'
    [0x00,0x00,0x7E,0x0C,0x18,0x30,0x7E,0x00], // 0x7A 'z'
    [0x0C,0x18,0x18,0x70,0x18,0x18,0x0C,0x00], // 0x7B '{'
    [0x18,0x18,0x18,0x00,0x18,0x18,0x18,0x00], // 0x7C '|'
    [0x30,0x18,0x18,0x0E,0x18,0x18,0x30,0x00], // 0x7D '}'
    [0x00,0x6E,0x3B,0x00,0x00,0x00,0x00,0x00], // 0x7E '~'
    [0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF], // 0x7F DEL
    // CP437 0x80–0xFF (IBM PC extended characters)
    [0x3C,0x66,0x60,0x60,0x66,0x3C,0x06,0x3C], // 0x80 Ç
    [0x00,0x66,0x00,0x66,0x66,0x66,0x3E,0x00], // 0x81 ü
    [0x1C,0x00,0x7E,0x60,0x7C,0x60,0x7E,0x00], // 0x82 é
    [0x7E,0xD8,0xD8,0xFE,0xD8,0xD8,0xCE,0x00], // 0x83 â
    [0x6C,0x00,0x38,0x0C,0x3C,0x6C,0x3E,0x00], // 0x84 ä
    [0x70,0x38,0x38,0x7C,0x6C,0x6C,0x38,0x00], // 0x85 à
    [0x1C,0x36,0x38,0x7C,0x6C,0x6C,0x38,0x00], // 0x86 å
    [0x00,0x3C,0x66,0x60,0x60,0x66,0x3C,0x30], // 0x87 ç
    [0x3E,0x60,0x7C,0x66,0x7C,0x06,0x7C,0x00], // 0x88 ê
    [0xCC,0x00,0x7C,0x66,0x7E,0x60,0x3C,0x00], // 0x89 ë
    [0x38,0x00,0x7E,0x60,0x7C,0x60,0x7E,0x00], // 0x8A è
    [0xCC,0x00,0x78,0x30,0x30,0x30,0x78,0x00], // 0x8B ï
    [0x78,0xCC,0x78,0x30,0x30,0x30,0x78,0x00], // 0x8C î
    [0x38,0x00,0x78,0x30,0x30,0x30,0x78,0x00], // 0x8D ì
    [0xCC,0x30,0x78,0xCC,0xFC,0xCC,0xCC,0x00], // 0x8E Ä
    [0x1C,0x36,0x78,0xCC,0xFC,0xCC,0xCC,0x00], // 0x8F Å
    [0x3C,0x60,0x7C,0x66,0x7C,0x06,0x3C,0x00], // 0x90 É
    [0x00,0x66,0x3C,0x66,0x7E,0x66,0x66,0x00], // 0x91 æ
    [0x3C,0x66,0x66,0x7E,0x66,0x66,0x66,0x00], // 0x92 Æ
    [0x38,0x00,0x3C,0x66,0x66,0x66,0x3C,0x00], // 0x93 ô
    [0x66,0x00,0x3C,0x66,0x66,0x66,0x3C,0x00], // 0x94 ö
    [0x70,0x00,0x3C,0x66,0x66,0x66,0x3C,0x00], // 0x95 ò
    [0x38,0x00,0x66,0x66,0x66,0x66,0x3E,0x00], // 0x96 û
    [0x70,0x00,0x66,0x66,0x66,0x66,0x3E,0x00], // 0x97 ù
    [0x66,0x00,0x66,0x66,0x66,0x3E,0x06,0x7C], // 0x98 ÿ
    [0xCC,0x00,0x78,0xCC,0xCC,0xCC,0x78,0x00], // 0x99 Ö
    [0xCC,0x00,0xCC,0xCC,0xCC,0xCC,0x7C,0x00], // 0x9A Ü
    [0x00,0x02,0x7C,0xCE,0xD6,0xE6,0x7C,0x80], // 0x9B ¢
    [0x38,0x6C,0x64,0xF0,0x60,0x66,0xFC,0x00], // 0x9C £
    [0xCC,0x78,0x78,0xCC,0xCC,0xCC,0x78,0x00], // 0x9D ¥
    [0x7C,0xC6,0xDC,0xD8,0xDC,0xC6,0x7C,0x00], // 0x9E ₧
    [0x0E,0x1B,0x18,0x3C,0x18,0x18,0xD8,0x70], // 0x9F ƒ
    [0x1C,0x00,0x38,0x0C,0x3C,0x6C,0x3E,0x00], // 0xA0 á
    [0x38,0x00,0x78,0x30,0x30,0x30,0x78,0x00], // 0xA1 í
    [0x38,0x00,0x3C,0x66,0x66,0x66,0x3C,0x00], // 0xA2 ó
    [0x38,0x00,0x66,0x66,0x66,0x66,0x3E,0x00], // 0xA3 ú
    [0x7A,0x00,0x6C,0x66,0x66,0x66,0x66,0x00], // 0xA4 ñ
    [0x7A,0x00,0xE6,0xF6,0xDE,0xCE,0xC6,0x00], // 0xA5 Ñ
    [0x3C,0x6C,0x6C,0x3E,0x00,0x7E,0x00,0x00], // 0xA6 ª
    [0x38,0x6C,0x6C,0x38,0x00,0x7C,0x00,0x00], // 0xA7 º
    [0x30,0x18,0x0C,0x18,0x30,0x00,0x7E,0x00], // 0xA8 ¿
    [0x70,0xD8,0xD8,0x70,0x00,0x00,0x00,0x00], // 0xA9 ⌐
    [0x00,0x00,0x00,0xFE,0x06,0x06,0x00,0x00], // 0xAA ¬
    [0x00,0x66,0x36,0x1C,0x36,0x66,0x00,0x00], // 0xAB ½
    [0x00,0x66,0x36,0x1E,0x3E,0x06,0x00,0x00], // 0xAC ¼
    [0x18,0x00,0x18,0x18,0x18,0x18,0x00,0x00], // 0xAD ¡
    [0x00,0x00,0x36,0x6C,0xD8,0x6C,0x36,0x00], // 0xAE «
    [0x00,0x00,0xD8,0x6C,0x36,0x6C,0xD8,0x00], // 0xAF »
    [0x11,0x44,0x11,0x44,0x11,0x44,0x11,0x44], // 0xB0 ░
    [0x55,0xAA,0x55,0xAA,0x55,0xAA,0x55,0xAA], // 0xB1 ▒
    [0xDD,0x77,0xDD,0x77,0xDD,0x77,0xDD,0x77], // 0xB2 ▓
    [0x18,0x18,0x18,0x18,0x18,0x18,0x18,0x18], // 0xB3 │
    [0x18,0x18,0x18,0x18,0xF8,0x18,0x18,0x18], // 0xB4 ┤
    [0x18,0x18,0xF8,0x18,0xF8,0x18,0x18,0x18], // 0xB5 ╡
    [0x6C,0x6C,0x6C,0x6C,0xEC,0x6C,0x6C,0x6C], // 0xB6 ╢
    [0x00,0x00,0x00,0x00,0xFC,0x6C,0x6C,0x6C], // 0xB7 ╖
    [0x00,0x00,0xF8,0x18,0xF8,0x18,0x18,0x18], // 0xB8 ╕
    [0x6C,0x6C,0xEC,0x0C,0xEC,0x6C,0x6C,0x6C], // 0xB9 ╣
    [0x6C,0x6C,0x6C,0x6C,0x6C,0x6C,0x6C,0x6C], // 0xBA ║
    [0x00,0x00,0xFC,0x0C,0xEC,0x6C,0x6C,0x6C], // 0xBB ╗
    [0x6C,0x6C,0xEC,0x0C,0xFC,0x00,0x00,0x00], // 0xBC ╝
    [0x6C,0x6C,0x6C,0x6C,0xFC,0x00,0x00,0x00], // 0xBD ╜
    [0x18,0x18,0xF8,0x18,0xF8,0x00,0x00,0x00], // 0xBE ╛
    [0x00,0x00,0x00,0x00,0xF8,0x18,0x18,0x18], // 0xBF ┐
    [0x18,0x18,0x18,0x18,0x1F,0x00,0x00,0x00], // 0xC0 └
    [0x18,0x18,0x18,0x18,0xFF,0x00,0x00,0x00], // 0xC1 ┴
    [0x00,0x00,0x00,0x00,0xFF,0x18,0x18,0x18], // 0xC2 ┬
    [0x18,0x18,0x18,0x18,0x1F,0x18,0x18,0x18], // 0xC3 ├
    [0x00,0x00,0x00,0x00,0xFF,0x00,0x00,0x00], // 0xC4 ─
    [0x18,0x18,0x18,0x18,0xFF,0x18,0x18,0x18], // 0xC5 ┼
    [0x18,0x18,0x1F,0x18,0x1F,0x00,0x00,0x00], // 0xC6 ╞
    [0x6C,0x6C,0x6C,0x6C,0x6F,0x6C,0x6C,0x6C], // 0xC7 ╟
    [0x6C,0x6C,0x6F,0x60,0x7F,0x00,0x00,0x00], // 0xC8 ╚
    [0x00,0x00,0x7F,0x60,0x6F,0x6C,0x6C,0x6C], // 0xC9 ╔
    [0x6C,0x6C,0xEF,0x00,0xFF,0x00,0x00,0x00], // 0xCA ╩
    [0x00,0x00,0xFF,0x00,0xEF,0x6C,0x6C,0x6C], // 0xCB ╦
    [0x6C,0x6C,0x6F,0x60,0x6F,0x6C,0x6C,0x6C], // 0xCC ╠
    [0x00,0x00,0xFF,0x00,0xFF,0x00,0x00,0x00], // 0xCD ═
    [0x6C,0x6C,0xEF,0x00,0xEF,0x6C,0x6C,0x6C], // 0xCE ╬
    [0x18,0x18,0xFF,0x00,0xFF,0x00,0x00,0x00], // 0xCF ╧
    [0x18,0x18,0xFF,0x00,0xFF,0x18,0x18,0x18], // 0xD0 ╨ (was ╧ in some versions)
    [0x00,0x00,0xFF,0x00,0xFF,0x18,0x18,0x18], // 0xD1 ╤
    [0x18,0x18,0x1F,0x18,0xFF,0x00,0x00,0x00], // 0xD2 ╥
    [0x00,0x00,0x7F,0x60,0x6F,0x6C,0x6C,0x6C], // 0xD3 ╙ (same as C9, approx)
    [0x00,0x00,0xFC,0x0C,0xEC,0x6C,0x6C,0x6C], // 0xD4 ╘ (approx)
    [0x6C,0x6C,0x6F,0x60,0xFF,0x00,0x00,0x00], // 0xD5 ╒ (approx)
    [0x00,0x00,0xFF,0x00,0xEF,0x6C,0x6C,0x6C], // 0xD6 ╓ (approx)
    [0x6C,0x6C,0xEF,0x00,0xFF,0x18,0x18,0x18], // 0xD7 ╫
    [0x18,0x18,0xFF,0x00,0xEF,0x6C,0x6C,0x6C], // 0xD8 ╪
    [0x18,0x18,0x18,0x18,0xF8,0x00,0x00,0x00], // 0xD9 ┘
    [0x00,0x00,0x00,0x00,0x1F,0x18,0x18,0x18], // 0xDA ┌
    [0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF,0xFF], // 0xDB █
    [0x00,0x00,0x00,0x00,0xFF,0xFF,0xFF,0xFF], // 0xDC ▄
    [0xF0,0xF0,0xF0,0xF0,0xF0,0xF0,0xF0,0xF0], // 0xDD ▌
    [0x0F,0x0F,0x0F,0x0F,0x0F,0x0F,0x0F,0x0F], // 0xDE ▐
    [0xFF,0xFF,0xFF,0xFF,0x00,0x00,0x00,0x00], // 0xDF ▀
    [0x78,0xCC,0xC0,0xCC,0x78,0x18,0x0C,0x78], // 0xE0 α
    [0x00,0xCC,0x76,0x66,0x76,0x60,0x66,0x00], // 0xE1 ß
    [0x00,0xFE,0xC0,0xFC,0xC0,0xC0,0xFE,0x00], // 0xE2 Γ
    [0x00,0xFF,0x99,0xFF,0x18,0x18,0x18,0x00], // 0xE3 π
    [0x00,0xFE,0xC0,0xF8,0xC0,0xC0,0xFE,0x00], // 0xE4 Σ
    [0x00,0x7E,0xD8,0xD8,0xD8,0xD8,0x70,0x00], // 0xE5 σ
    [0x00,0x66,0x66,0x66,0x66,0x3C,0x18,0x30], // 0xE6 µ
    [0x00,0x18,0x3C,0x7E,0x18,0x18,0x18,0x18], // 0xE7 τ
    [0x00,0x7E,0x18,0x18,0x18,0x18,0x7E,0x00], // 0xE8 Φ
    [0x66,0x66,0x66,0x7E,0x66,0x66,0x66,0x00], // 0xE9 Θ
    [0x00,0x7E,0x42,0x42,0x42,0x42,0x7E,0x00], // 0xEA Ω
    [0x00,0x0C,0x18,0x3C,0x66,0x3C,0x18,0x0C], // 0xEB δ
    [0x00,0x7C,0xC6,0xC6,0xC6,0xC6,0xC6,0x00], // 0xEC ∞
    [0x00,0xFE,0xC6,0xFE,0xC6,0xC6,0xFE,0x00], // 0xED φ
    [0x00,0x18,0x3C,0x66,0x66,0x3C,0x18,0x00], // 0xEE ε
    [0x00,0x66,0x3C,0x18,0x3C,0x66,0x00,0x00], // 0xEF ∩
    [0x00,0x7E,0x7E,0x00,0x7E,0x7E,0x00,0x00], // 0xF0 ≡
    [0x18,0x18,0x7E,0x18,0x18,0x00,0x7E,0x00], // 0xF1 ±
    [0x00,0x78,0x0C,0x38,0x60,0x78,0x00,0x00], // 0xF2 ≥
    [0x00,0x3C,0x60,0x38,0x0C,0x78,0x00,0x00], // 0xF3 ≤
    [0x3E,0x60,0x60,0x7C,0x60,0x60,0x60,0x00], // 0xF4 ⌠
    [0x06,0x06,0x06,0x3E,0x06,0x06,0x06,0x00], // 0xF5 ⌡
    [0x00,0x18,0x00,0x7E,0x00,0x18,0x00,0x00], // 0xF6 ÷
    [0x00,0x76,0xDC,0x00,0x76,0xDC,0x00,0x00], // 0xF7 ≈
    [0x38,0x6C,0x6C,0x38,0x00,0x00,0x00,0x00], // 0xF8 °
    [0x00,0x00,0x18,0x18,0x00,0x00,0x00,0x00], // 0xF9 ∙
    [0x00,0x00,0x00,0x18,0x00,0x00,0x00,0x00], // 0xFA ·
    [0x00,0x0F,0x0C,0x0C,0x0C,0x6C,0x38,0x00], // 0xFB √
    [0x00,0x60,0x78,0x6C,0x66,0x00,0x00,0x00], // 0xFC ⁿ
    [0x00,0x78,0x0C,0x7C,0xCC,0x78,0x00,0x00], // 0xFD ²
    [0x00,0x7E,0x7E,0x7E,0x7E,0x7E,0x7E,0x00], // 0xFE ■
    [0x00,0x00,0x00,0x00,0x00,0x00,0x00,0x00], // 0xFF nbsp
];

// ── EGA palette ───────────────────────────────────────────────────────────────

pub const EGA: [(u8, u8, u8); 16] = [
    (0,0,0),       (0,0,170),     (0,170,0),     (0,170,170),
    (170,0,0),     (170,0,170),   (170,85,0),    (170,170,170),
    (85,85,85),    (85,85,255),   (85,255,85),   (85,255,255),
    (255,85,85),   (255,85,255),  (255,255,85),  (255,255,255),
];

/// PUT sprite-blit action verb (QBasic `PUT (x,y),array[,verb]`). The default
/// verb in QBasic when none is written is `Xor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PutAction { Pset, Preset, And, Or, Xor }

// ── 256-entry palette storage (SCREEN 13 = MCGA 256-color) ────────────────────

/// Construction-time / non-mode-13 palette: the 16 EGA colors in slots 0–15,
/// black in 16–255. Used by all EGA modes (only 0–15 are ever indexed there).
const fn default_palette_256() -> [(u8, u8, u8); 256] {
    let mut p = [(0u8, 0u8, 0u8); 256];
    let mut i = 0;
    while i < 16 {
        p[i] = EGA[i];
        i += 1;
    }
    p
}
const DEFAULT_PALETTE_256: [(u8, u8, u8); 256] = default_palette_256();

/// Expand a 6-bit DAC channel value (0–63) to 8-bit (0–255), VGA-style.
#[inline]
fn dac6_to_8(c: u8) -> u8 { (c << 2) | (c >> 4) }

/// Decode an 18-bit SCREEN 13 PALETTE color value into 8-bit RGB.
/// QB encoding: `color = red + 256*green + 65536*blue`, each channel 0–63.
fn dac18_to_rgb(v: u64) -> (u8, u8, u8) {
    (dac6_to_8((v & 63) as u8),
     dac6_to_8(((v >> 8) & 63) as u8),
     dac6_to_8(((v >> 16) & 63) as u8))
}

/// The canonical VGA BIOS power-on default palette for mode 13h.
/// Reproduces the well-known table (matches DOSBox 0.74 / Allegro 4.4.2);
/// algorithm from canidlogic/vgapal. Built in 6-bit DAC space, expanded to 8-bit.
fn vga256_default() -> [(u8, u8, u8); 256] {
    // One "run" of 4 colors: first uses `start` (hi where bit set, else lo),
    // then ramps channel `ch` (4=R,2=G,1=B) through the mid levels.
    fn add_run(pal: &mut Vec<(u8, u8, u8)>, start: i32, ch: i32,
               lo: i32, melo: i32, me: i32, mehi: i32, hi: i32) -> i32 {
        let (mut r, mut g, mut b) = (lo, lo, lo);
        if start & 4 == 4 { r = hi; }
        if start & 2 == 2 { g = hi; }
        if start & 1 == 1 { b = hi; }
        pal.push((r as u8, g as u8, b as u8));
        let up = (start & ch) != ch;
        for i in 0..3 {
            let v = if up {
                if i == 0 { melo } else if i == 1 { me } else { mehi }
            } else {
                if i == 0 { mehi } else if i == 1 { me } else { melo }
            };
            match ch { 4 => r = v, 2 => g = v, _ => b = v }
            pal.push((r as u8, g as u8, b as u8));
        }
        start ^ ch
    }
    // One "cycle" of 24 colors: 6 runs ramping R,B,G,R,B,G in turn.
    fn add_cycle(pal: &mut Vec<(u8, u8, u8)>,
                 lo: i32, melo: i32, me: i32, mehi: i32, hi: i32) {
        let mut hue = 1;
        for &ch in &[4, 1, 2, 4, 1, 2] {
            hue = add_run(pal, hue, ch, lo, melo, me, mehi, hi);
        }
    }

    let mut p6: Vec<(u8, u8, u8)> = Vec::with_capacity(256);
    // 0–15: EGA/IRGB colors (>>2 converts our 8-bit EGA const back to 6-bit DAC).
    for &(r, g, b) in EGA.iter() { p6.push((r >> 2, g >> 2, b >> 2)); }
    // 16–31: grayscale ramp.
    for &v in &[0u8, 5, 8, 11, 14, 17, 20, 24, 28, 32, 36, 40, 45, 50, 56, 63] {
        p6.push((v, v, v));
    }
    // 32–247: nine 24-color cycles (high/med/low value × high/med/low saturation).
    add_cycle(&mut p6,  0, 16, 31, 47, 63);
    add_cycle(&mut p6, 31, 39, 47, 55, 63);
    add_cycle(&mut p6, 45, 49, 54, 58, 63);
    add_cycle(&mut p6,  0,  7, 14, 21, 28);
    add_cycle(&mut p6, 14, 17, 21, 24, 28);
    add_cycle(&mut p6, 20, 22, 24, 26, 28);
    add_cycle(&mut p6,  0,  4,  8, 12, 16);
    add_cycle(&mut p6,  8, 10, 12, 14, 16);
    add_cycle(&mut p6, 11, 12, 13, 15, 16);
    // 248–255: black.
    while p6.len() < 256 { p6.push((0, 0, 0)); }

    let mut out = [(0u8, 0u8, 0u8); 256];
    for (i, &(r, g, b)) in p6.iter().enumerate() {
        out[i] = (dac6_to_8(r), dac6_to_8(g), dac6_to_8(b));
    }
    out
}

// ── File I/O types ───────────────────────────────────────────────────────────

enum QbFile {
    /// FOR INPUT / OUTPUT / APPEND — text sequential access.
    Sequential {
        reader: Option<std::io::BufReader<std::fs::File>>,
        writer: Option<std::io::BufWriter<std::fs::File>>,
    },
    /// FOR RANDOM — fixed-length binary record access.
    Random {
        file:       std::fs::File,
        record_len: usize,
        cur_record: i64,  // 0-based current record pointer (incremented on sequential GET/PUT)
    },
}

// ── Runtime struct ────────────────────────────────────────────────────────────

pub struct Runtime {
    // Text state
    pub fg_color: u8,
    pub bg_color: u8,
    cursor_row: usize,  // 1-based text row
    cursor_col: usize,  // 1-based text col
    cursor_visible: bool, // LOCATE's cursor-visibility arg (3rd param); no blinking
                          // cursor is rendered in the windowed runtime, so this is
                          // tracked for fidelity/future use but has no visual effect.
    // Graphics
    pub screen_mode: u8,
    pub width:  u32,
    pub height: u32,
    char_w: u32,        // character cell width in pixels (always 8)
    char_h: u32,        // character cell height in pixels (8 or 14 depending on mode)
    fb: Vec<u8>,           // palette-indexed pixels
    palette_rgb: [(u8,u8,u8); 256], // remappable palette (PALETTE statement); 256 entries for SCREEN 13
    window: Option<minifb::Window>,
    /// Window title and dimensions — stored here so we can create the window
    /// lazily on the first SCREEN call instead of at Runtime::new() time.
    /// This lets text-only programs (no SCREEN) run without opening a GUI window.
    win_title: String,
    last_present: std::time::Instant,  // for auto frame pacing
    pset_counter: u32,                 // cheap throttle for auto_present checks
    fullspeed: bool,                   // when true, skip auto_present() throttle (REM QBC FULLSPEED)
    frame_interval_ms: u64,            // target ms between frames (REM QBC FPS n); default 16 ≈ 60fps
    pace_ms: u64,                      // when >0, auto_present SLEEPS to this interval so the
                                       // drawing is paced/watchable (REM QBC PACE n fps); 0 = off
    slowmo: f64,                       // SLEEP duration multiplier (REM QBC SLOWMO n); default 1.0
    win_w: usize,                      // output window width  (REM QBC SCALE n); default 960
    win_h: usize,                      // output window height (REM QBC SCALE n); default 600
    /// Persistent blit scratch buffer handed to minifb's `update_with_buffer`.
    /// minifb's macOS backend stores the RAW pointer (no copy) and re-reads it on
    /// every later `update()` (event pump). A per-call local Vec would be freed on
    /// return, leaving minifb a dangling pointer → use-after-free segfault in
    /// `drawInMTKView`/`replaceRegion` during the next idle `INKEY$` poll. Keeping
    /// it on the Runtime makes the pointer valid for the window's whole lifetime.
    present_buf: Vec<u32>,
    /// Keys harvested from the window on every update — never lost between frames.
    key_queue: std::collections::VecDeque<String>,
    // RNG (LCG matching QB's generator)
    rng: u32,
    /// Last value returned by rnd() — QB's RND(0) repeats it.
    last_rnd: f64,
    // VIEW / WINDOW logical coordinate system
    view_x1: f64, view_y1: f64, view_x2: f64, view_y2: f64,
    view_active: bool,
    win_x1: f64, win_y1: f64, win_x2: f64, win_y2: f64,
    win_active: bool,
    // WINDOW SCREEN (screen-orientation Y, no inversion) vs plain WINDOW (Y inverted)
    win_screen: bool,
    // Graphics cursor (in logical coords; updated by pset/line/line_to)
    gfx_x: f64,
    gfx_y: f64,
    // DRAW state: persists across DRAW calls in the same screen session
    draw_scale: f64,   // S value; pixels_per_unit = draw_scale / 4.0
    draw_color: u8,    // C value (current DRAW color)
    // PLAY MML state: persists across PLAY calls (tempo, octave, length, mode)
    mml_state: MmlState,
    /// True while a background PLAY thread is still running.
    /// Used by play_count() to throttle re-triggering from PLAY(0) checks.
    bg_playing: std::sync::Arc<std::sync::atomic::AtomicBool>,
    /// Open file handles — keyed by QB file number (1-255).
    files: std::collections::HashMap<u8, QbFile>,
    /// VIEW PRINT text viewport — 1-based row numbers (inclusive).
    /// vp_top=1, vp_bot=0 means "not set" (use full screen).
    vp_top: u32,
    vp_bot: u32,
    /// True once any explicit SCREEN N call has been made.
    /// Controls two behaviours:
    ///   • wait_for_key() only blocks when true (so text-only programs exit
    ///     immediately without hanging the integration-test timeout)
    ///   • print_gfx() only suppresses stdout when true (so text-only programs
    ///     still produce stdout output that the test suite can capture)
    had_screen_call: bool,
    /// ON ERROR support: set true when a trappable error occurs (OPEN failure,
    /// SCREEN mode unavailable, etc.).  The emitter checks this after every
    /// fallible statement and, if true, dispatches to the error-handler label.
    /// Cleared by the emitted error-dispatch code (RESUME / RESUME NEXT).
    pub error_pending: bool,
    /// QB ERR system variable — holds the QB error code for the last error.
    /// 53 = "file not found", 24/25 = printer errors, 0 = no error.
    pub err_code: f64,
    /// Headless driver config (Some when any `QBC_*` env var requests it).
    /// Drives scripted input, framebuffer export, and guaranteed auto-exit so a
    /// transpiled binary can be run non-interactively for debugging and tests.
    headless_cfg: Option<HeadlessCfg>,
    /// When true, `randomize()` is a no-op so `QBC_SEED` pins the RNG even past
    /// a `RANDOMIZE TIMER` in the program (deterministic renders for goldens).
    seed_locked: bool,
    /// Number of `present()`/`auto_present()` blits so far (headless exit policy).
    present_count: u64,
    /// Consecutive empty `inkey()` polls (headless `idle` exit threshold).
    idle_polls: u32,
    /// Simulated byte memory for POKE/PEEK.  QB POKE stores a byte; PEEK reads it back.
    poke_mem: std::collections::HashMap<u32, u8>,
    /// VGA DAC state for OUT &H3C8/&H3C9 port writes.
    dac_write_idx: usize,  // current palette entry being written (auto-advances)
    dac_channel:   u8,     // which sub-channel: 0=R, 1=G, 2=B
    dac_pending_r: u8,     // accumulated red (6-bit DAC value)
    dac_pending_g: u8,     // accumulated green
    dac_read_idx:  usize,  // current palette entry for INP reads
    dac_read_ch:   u8,     // read sub-channel: 0=R, 1=G, 2=B
}

/// When to write the framebuffer image in headless mode.
#[derive(Clone, Copy)]
enum DumpAt { Exit, Present(u64), Ms(u64) }

/// Guaranteed-termination policy for a headless run.
#[derive(Clone, Copy)]
enum ExitAfter { Idle, Ms(u64), Presents(u64) }

/// Parsed `QBC_*` headless-driver configuration.
struct HeadlessCfg {
    dump_path:  Option<String>,
    dump_at:    DumpAt,
    dumped:     bool,
    checksum:   bool,
    fbstats:    bool,
    text_to_fb: bool,
    exit_after: ExitAfter,
    start:      std::time::Instant,
    safety_ms:  u64,
}

/// Parse a `kind:value` env spec like `present:50` or `ms:2000`.
fn parse_kv(s: &str) -> Option<(&str, u64)> {
    let (k, v) = s.split_once(':')?;
    Some((k.trim(), v.trim().parse().ok()?))
}

/// Build a `HeadlessCfg` from the environment, or `None` if no driver var is set.
/// Triggered by `QBC_HEADLESS`, `QBC_KEYS`, `QBC_DUMP`, `QBC_CHECKSUM`,
/// `QBC_FBSTATS`, or `QBC_EXIT_AFTER`. (`QBC_SEED` alone does NOT force headless.)
fn parse_headless_env() -> Option<HeadlessCfg> {
    let env = |k: &str| std::env::var(k).ok();
    let any = env("QBC_HEADLESS").is_some() || env("QBC_KEYS").is_some()
        || env("QBC_DUMP").is_some() || env("QBC_CHECKSUM").is_some()
        || env("QBC_FBSTATS").is_some() || env("QBC_EXIT_AFTER").is_some()
        || env("QBC_TEXT_FB").is_some();
    if !any { return None; }

    let dump_at = match env("QBC_DUMP_AT").as_deref() {
        Some("exit") | None => DumpAt::Exit,
        Some(s) => match parse_kv(s) {
            Some(("present", n)) => DumpAt::Present(n),
            Some(("ms", n))      => DumpAt::Ms(n),
            _ => DumpAt::Exit,
        },
    };
    let exit_after = match env("QBC_EXIT_AFTER").as_deref() {
        Some("idle") | None => ExitAfter::Idle,
        Some(s) => match parse_kv(s) {
            Some(("ms", n))       => ExitAfter::Ms(n),
            Some(("presents", n)) => ExitAfter::Presents(n),
            _ => ExitAfter::Idle,
        },
    };
    Some(HeadlessCfg {
        dump_path:  env("QBC_DUMP"),
        dump_at,
        dumped:     false,
        checksum:   env("QBC_CHECKSUM").is_some(),
        fbstats:    env("QBC_FBSTATS").is_some(),
        text_to_fb: env("QBC_TEXT_FB").is_some(),
        exit_after,
        start:      std::time::Instant::now(),
        safety_ms:  10_000, // wall-clock safety cap so a run never hangs
    })
}

/// Translate a `QBC_KEYS` token into the QB key string `inkey()` returns.
/// Named keys map to QB's extended forms; a single char passes through.
fn normalize_key(tok: &str) -> String {
    match tok.to_ascii_uppercase().as_str() {
        "UP"     => "\u{0}H".to_string(),   // extended scan codes (CHR$(0)+code)
        "DOWN"   => "\u{0}P".to_string(),
        "LEFT"   => "\u{0}K".to_string(),
        "RIGHT"  => "\u{0}M".to_string(),
        "ENTER" | "RETURN" => "\r".to_string(),
        "ESC" | "ESCAPE"   => "\u{1b}".to_string(),
        "SPACE"  => " ".to_string(),
        "TAB"    => "\t".to_string(),
        "BACKSPACE" => "\u{8}".to_string(),
        // DRAIN / BARRIER — synthetic "queue empty" sentinel for headless scripts.
        // A QBasic `WHILE INKEY$ <> "": WEND` drain-loop pops this and sees ""
        // (stopping the loop) while leaving subsequent scripted keys intact.
        "DRAIN" | "BARRIER" => "\u{0}".to_string(),
        _ => {
            // A single letter: the unshifted key returns lowercase (matching
            // minifb_key_to_qb), so `Q` → "q" (reversi's QUIT = ASC("q") = 113).
            if tok.len() == 1 && tok.chars().next().unwrap().is_ascii_alphabetic() {
                tok.to_ascii_lowercase()
            } else {
                tok.to_string() // digit / punctuation / multi-char literal
            }
        }
    }
}

// Default output window size — both text and graphics modes scale into this.
// Text  (640×400) → 960×600 at 1.5× (exact integer: 960/640=1.5, 600/400=1.5)
// SCREEN 7 (320×200) → 960×600 at 3×  (exact integer: 960/320=3,  600/200=3)
// Override per-program with REM QBC SCALE n (multiplies 960×600 by n).
const DEFAULT_WIN_W: usize = 960;
const DEFAULT_WIN_H: usize = 600;

impl Runtime {
    /// Create a Runtime with no window — for tests and headless rendering.
    /// All graphics operations write to the framebuffer normally; `present()`
    /// and `auto_present()` are no-ops.
    pub fn headless() -> Self {
        Self {
            fg_color:     7,
            bg_color:     0,
            cursor_row:   1,
            cursor_col:   1,
            cursor_visible: true,
            screen_mode:  0,
            width:        640,
            height:       400,
            char_w:       8,
            char_h:       16,
            fb:           vec![0u8; 640 * 400],
            palette_rgb:  DEFAULT_PALETTE_256,
            window:             None,
            win_title:          "QBasic".to_string(),
            last_present:       std::time::Instant::now(),
            pset_counter:       0,
            fullspeed:          false,
            frame_interval_ms:  16,
            pace_ms:            0,
            slowmo:             1.0,
            win_w:              DEFAULT_WIN_W,
            win_h:              DEFAULT_WIN_H,
            present_buf:  Vec::new(),
            key_queue:    std::collections::VecDeque::new(),
            rng:          0x50000, // QB power-on seed: first RND = .7055475
            last_rnd:     0.0,
            view_x1: 0.0, view_y1: 0.0, view_x2: 0.0, view_y2: 0.0, view_active: false,
            win_x1:  0.0, win_y1:  0.0, win_x2:  0.0, win_y2:  0.0, win_active:  false, win_screen: false,
            gfx_x: 0.0, gfx_y: 0.0,
            draw_scale: 4.0,
            draw_color: 7,
            mml_state: MmlState::default(),
            bg_playing: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            had_screen_call: false,
            files: std::collections::HashMap::new(),
            vp_top: 1,
            vp_bot: 0, // 0 = unset (use full height)
            error_pending: false,
            err_code: 0.0,
            headless_cfg: None,
            seed_locked: false,
            present_count: 0,
            idle_polls: 0,
            poke_mem: std::collections::HashMap::new(),
            dac_write_idx: 0, dac_channel: 0, dac_pending_r: 0, dac_pending_g: 0,
            dac_read_idx:  0, dac_read_ch:  0,
        }
    }

    /// Create a Runtime with default window title and size.
    pub fn new() -> Self {
        Self::new_configured("QBasic", DEFAULT_WIN_W, DEFAULT_WIN_H)
    }

    /// Create a Runtime with a custom window title and output dimensions.
    /// Used by `REM QBC TITLE` and/or `REM QBC SCALE` directives; the emitter
    /// calls this constructor instead of `new()` when either is present.
    pub fn new_configured(title: &str, win_w: usize, win_h: usize) -> Self {
        // QBC_TITLE / QBC_SCALE override the compile-time pragma values.
        // These must be resolved before the window is created.
        let title_override = std::env::var("QBC_TITLE").ok();
        let effective_title = title_override.as_deref().unwrap_or(title);
        let (effective_w, effective_h) = if let Ok(s) = std::env::var("QBC_SCALE") {
            if let Ok(n) = s.trim().parse::<usize>() {
                (960 * n.max(1), 600 * n.max(1))
            } else {
                (win_w, win_h)
            }
        } else {
            (win_w, win_h)
        };

        // The headless driver (env-var controlled) suppresses the window so a
        // transpiled binary can run non-interactively for debugging / tests.
        let headless_cfg = parse_headless_env();
        // Open the window immediately so that even text-only programs display
        // output in the GUI rather than the terminal.  Window creation is
        // attempted with .ok() so it fails gracefully in headless environments.
        let mut window = if headless_cfg.is_some() {
            None
        } else {
            let opts = minifb::WindowOptions {
                scale: minifb::Scale::X1,
                resize: false,
                ..minifb::WindowOptions::default()
            };
            minifb::Window::new(effective_title, effective_w, effective_h, opts).ok()
        };
        // Disable minifb's built-in frame-rate limiter (default 250 FPS = 4ms).
        // It sleeps inside *both* update() and update_with_buffer(), which makes
        // per-pixel INKEY$ polling catastrophic (mandel: ~73k pixels × 4ms ≈ 5
        // min). set_target_fps(0) = no waiting; we do our own pacing via
        // frame_interval_ms / auto_present().
        if let Some(w) = window.as_mut() { w.set_target_fps(0); }
        let is_headless = headless_cfg.is_some();
        let mut rt = Self {
            fg_color:          7,
            bg_color:          0,
            cursor_row:        1,
            cursor_col:        1,
            cursor_visible:    true,
            screen_mode:       0,
            width:             640,   // pixel width of text-mode fb
            height:            400,   // pixel height of text-mode fb
            char_w:            8,
            char_h:            16,    // 8×16 font → 80×25 text grid
            fb:                vec![0u8; 640 * 400],
            palette_rgb:       DEFAULT_PALETTE_256,
            window,
            win_title:         effective_title.to_string(),
            last_present:      std::time::Instant::now(),
            pset_counter:      0,
            fullspeed:         false,
            frame_interval_ms: 16,
            pace_ms:           0,
            slowmo:            1.0,
            win_w:             effective_w,
            win_h:             effective_h,
            present_buf:  Vec::new(),
            key_queue:    std::collections::VecDeque::new(),
            rng:          0x50000, // QB power-on seed: first RND = .7055475
            last_rnd:     0.0,
            view_x1: 0.0, view_y1: 0.0, view_x2: 0.0, view_y2: 0.0, view_active: false,
            win_x1:  0.0, win_y1:  0.0, win_x2:  0.0, win_y2:  0.0, win_active:  false, win_screen: false,
            gfx_x: 0.0, gfx_y: 0.0,
            draw_scale: 4.0,
            draw_color: 7,
            mml_state: MmlState::default(),
            bg_playing: std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
            had_screen_call: false,
            files: std::collections::HashMap::new(),
            vp_top: 1,
            vp_bot: 0, // 0 = unset (use full height)
            error_pending: false,
            err_code: 0.0,
            headless_cfg,
            seed_locked: false,
            present_count: 0,
            idle_polls: 0,
            poke_mem: std::collections::HashMap::new(),
            dac_write_idx: 0, dac_channel: 0, dac_pending_r: 0, dac_pending_g: 0,
            dac_read_idx:  0, dac_read_ch:  0,
        };
        // QBC_SEED pins the RNG (overrides RANDOMIZE TIMER) — applies in windowed
        // mode too, so a seeded run can still be watched live.
        if let Ok(s) = std::env::var("QBC_SEED") {
            if let Ok(n) = s.trim().parse::<u32>() {
                rt.rng = n;
                rt.seed_locked = true;
            }
        }
        // QBC_KEYS pre-loads scripted keystrokes into the queue (headless input).
        if is_headless {
            if let Ok(keys) = std::env::var("QBC_KEYS") {
                for k in keys.split(',') {
                    let k = k.trim();
                    if !k.is_empty() { rt.inject_key(&normalize_key(k)); }
                }
            }
        }
        rt
    }

    /// Apply behavioral env-var overrides after the compile-time pragma calls in main().
    ///
    /// Called unconditionally from every generated `main()` immediately after the
    /// pragma-emitted `set_*` calls, so env always wins over the pragma:
    ///
    ///   QBC_PACE=30       — sleep-paced draw at N blits/sec (watchable pixel art)
    ///   QBC_FPS=60        — frame rate cap for auto_present() throttle
    ///   QBC_FULLSPEED=1   — skip auto_present() throttle entirely (0 to disable)
    ///   QBC_SLOWMO=2.5    — slow every frame by N× (multiplicative with FPS)
    ///
    /// QBC_SCALE and QBC_TITLE are handled inside new_configured() before window
    /// creation and are not re-read here.
    pub fn apply_behavioral_env(&mut self) {
        if let Ok(v) = std::env::var("QBC_PACE") {
            if let Ok(fps) = v.trim().parse::<f64>() { self.set_pace(fps); }
        }
        if let Ok(v) = std::env::var("QBC_FPS") {
            if let Ok(fps) = v.trim().parse::<f64>() { self.set_fps(fps); }
        }
        if let Ok(v) = std::env::var("QBC_FULLSPEED") {
            self.set_fullspeed(v.trim() != "0");
        }
        if let Ok(v) = std::env::var("QBC_SLOWMO") {
            if let Ok(f) = v.trim().parse::<f64>() { self.set_slowmo(f); }
        }
    }

    // ── Screen / color / cursor ───────────────────────────────────────────────

    pub fn screen(&mut self, mode: f64) {
        self.screen_mode = mode as u8;
        self.had_screen_call = true;
        // Pixel framebuffer dimensions per mode.
        // Mode 0 (text): 80×25 chars × 8×16 px = 640×400 pixels.
        let (w, h) = match mode as u8 {
            0  => (640, 400),
            1  => (320, 200),
            2  => (640, 200),
            7  => (320, 200),
            8  => (640, 200),
            9  => (640, 350),
            12 => (640, 480),
            13 => (320, 200),
            _  => (640, 400),
        };
        self.width  = w;
        self.height = h;
        self.fb     = vec![0u8; (w * h) as usize];
        self.palette_rgb = if mode as u8 == 13 { vga256_default() } else { DEFAULT_PALETTE_256 };
        self.char_w = 8;
        // Character cell height per mode.  VGA hardware text layer:
        // SCREEN 0/11/12 → 8×16 (80×30 or 80×25 grid depending on scanlines)
        // SCREEN 9 → 8×14 (EGA alphanumeric font, 80×25)
        // All other modes (1,7,8,13) → 8×8 (CGA/VGA low-res text)
        self.char_h = match mode as u8 { 0 | 11 | 12 => 16, 9 => 14, _ => 8 };
        // Window is now opened eagerly in new_configured(); nothing to do here
        // except ensure it's alive (creation may have failed on headless systems).
        self.cursor_row = 1;
        self.cursor_col = 1;
        // Reset logical coordinate system and DRAW state
        self.view_active = false;
        self.win_active  = false;
        self.win_screen  = false;
        self.gfx_x = 0.0;
        self.gfx_y = 0.0;
        self.draw_scale = 4.0;
        self.draw_color = self.fg_color;
    }

    /// CLS [arg] — 0 or no arg = full screen, 1 = text area, 2 = viewport only.
    pub fn cls(&mut self, arg: u8) {
        let bg = self.bg_color;
        let max_rows = (self.height / self.char_h) as u32;
        let top = if arg == 2 && self.vp_bot > 0 { self.vp_top } else { 1 };
        let bot = if arg == 2 && self.vp_bot > 0 { self.vp_bot } else { max_rows };
        if top == 1 && bot == max_rows {
            // Full clear
            self.fb.iter_mut().for_each(|p| *p = bg);
        } else {
            // Partial clear — only the viewport rows
            let px_top = ((top - 1) * self.char_h * self.width) as usize;
            let px_bot = (bot * self.char_h * self.width) as usize;
            let end = px_bot.min(self.fb.len());
            if px_top < end {
                self.fb[px_top..end].iter_mut().for_each(|p| *p = bg);
            }
        }
        // Move cursor to top of the cleared region
        self.cursor_row = top as usize;
        self.cursor_col = 1;
        self.present();
    }

    /// VIEW PRINT [top TO bot] — set or reset the text scrolling viewport.
    /// Bare VIEW PRINT (top=None) resets to full screen.
    pub fn view_print(&mut self, top: Option<f64>, bot: Option<f64>) {
        match (top, bot) {
            (Some(t), Some(b)) => {
                self.vp_top = (t as u32).max(1);
                self.vp_bot = (b as u32).max(self.vp_top);
            }
            _ => {
                // Reset to full screen
                self.vp_top = 1;
                self.vp_bot = 0;
            }
        }
    }

    /// Number of distinct color indices in the current mode: 256 for SCREEN 13
    /// (MCGA), 16 for every EGA/CGA mode. Used to wrap out-of-range color
    /// arguments the way QB clamps them.
    #[inline]
    fn color_mod(&self) -> i64 { if self.screen_mode == 13 { 256 } else { 16 } }

    /// Max color value (bit mask) for the current screen mode's pixel depth,
    /// used to invert a sprite pixel for the PUT `PRESET` verb: CGA = 2bpp (3),
    /// MCGA mode 13 = 8bpp (255), every EGA mode = 4bpp (15).
    #[inline]
    fn sprite_color_mask(&self) -> u8 {
        match self.screen_mode { 1 => 3, 13 => 255, _ => 15 }
    }

    pub fn color(&mut self, fg: f64, bg: Option<f64>) {
        if self.screen_mode == 1 {
            // CGA SCREEN 1: COLOR bg_ega_idx, palette_selector
            // fg selects the background color (EGA index 0–15).
            // bg selects CGA palette 0 (green/red/yellow) or 1 (cyan/magenta/white).
            let bg_ega = (fg as i64).rem_euclid(16) as usize;
            self.palette_rgb[0] = EGA[bg_ega];
            self.bg_color = 0;
            let palette_sel = bg.map(|b| (b as i64).rem_euclid(2)).unwrap_or(1);
            // CGA palette 1: cyan(3), magenta(5), white(15)
            // CGA palette 0: green(2), red(4), yellow(14)
            let (c1, c2, c3): (usize, usize, usize) = if palette_sel == 1 {
                (3, 5, 15)
            } else {
                (2, 4, 14)
            };
            self.palette_rgb[1] = EGA[c1];
            self.palette_rgb[2] = EGA[c2];
            self.palette_rgb[3] = EGA[c3];
            self.fg_color = 3; // default fg = brightest CGA color
        } else {
            let m = self.color_mod();
            self.fg_color = (fg as i64).rem_euclid(m) as u8;
            if let Some(b) = bg {
                self.bg_color = (b as i64).rem_euclid(m) as u8;
            }
        }
        // QB: DRAW with no `C` verb uses the current COLOR foreground. Keep the
        // DRAW color in sync so a later `DRAW "..."` (no C) paints in the new
        // foreground — and so a following PAINT whose border = that color sees a
        // matching outline. (Donkey's "S08" sprite relies on this; without it the
        // outline drew in the stale default color and PAINT flooded the region.)
        self.draw_color = self.fg_color;
    }

    /// PALETTE attr, color64 — remap a palette attribute index to an EGA 64-color value.
    /// EGA 64-color encoding: bits [5:4:3] = RGB high, bits [2:1:0] = RGB low (bright).
    /// Formula: R = 170*((v>>2)&1) + 85*((v>>5)&1), same pattern for G (bits 1,4) and B (bits 0,3).
    pub fn palette(&mut self, attr: f64, color64: f64) {
        if self.screen_mode == 13 {
            // MCGA: 256 attributes; color value is an 18-bit DAC triple.
            let idx = (attr as i64).rem_euclid(256) as usize;
            self.palette_rgb[idx] = dac18_to_rgb(color64 as u64);
            return;
        }
        if self.screen_mode == 11 || self.screen_mode == 12 {
            // VGA 16-color hi-res modes also take the 18-bit DAC triple
            // (color = red + 256*green + 65536*blue, each channel 0–63) — NOT the
            // EGA irgb nibble. torus.bas mixes its palette this way in SCREEN 12.
            let idx = (attr as i64).rem_euclid(16) as usize;
            self.palette_rgb[idx] = dac18_to_rgb(color64 as u64);
            return;
        }
        let idx = (attr as i64).rem_euclid(16) as usize;
        let v = color64 as u64;
        let r = (170 * ((v >> 2) & 1) + 85 * ((v >> 5) & 1)) as u8;
        let g = (170 * ((v >> 1) & 1) + 85 * ((v >> 4) & 1)) as u8;
        let b = (170 * ((v >> 0) & 1) + 85 * ((v >> 3) & 1)) as u8;
        self.palette_rgb[idx] = (r, g, b);
    }

    /// Bare PALETTE (no arguments) — restore the mode's default palette.
    /// QB programs use this after a PALETTE USING blackout (e.g. qblocks
    /// hides its sprite-sheet drawing behind an all-black palette, then
    /// resets).  Present so the restored colors become visible immediately.
    pub fn palette_reset(&mut self) {
        self.palette_rgb = if self.screen_mode == 13 { vga256_default() } else { DEFAULT_PALETTE_256 };
        self.present();
    }

    /// LOCATE [row][,[col][,[cursor]]] — move the text cursor and/or set its
    /// visibility. Any argument may be omitted (`None`); omitted row/col leave
    /// the cursor where it is (QB semantics) rather than moving it to (0,0).
    /// The `cursor` arg (0 = hide, non-zero = show) is recorded but has no
    /// visual effect — the windowed runtime draws no blinking cursor.
    pub fn locate(&mut self, row: Option<f64>, col: Option<f64>, cursor: Option<f64>) {
        if let Some(r) = row { self.cursor_row = r as usize; }
        if let Some(c) = col { self.cursor_col = c as usize; }
        if let Some(v) = cursor { self.cursor_visible = v != 0.0; }
        // Cursor position tracked internally; draw_char_fb() renders at this position.
    }

    /// Whether the text cursor is currently set visible (LOCATE's 3rd arg).
    pub fn cursor_visible(&self) -> bool { self.cursor_visible }

    // ── Print ─────────────────────────────────────────────────────────────────

    pub fn print(&mut self, args: &[String]) {
        for s in args { self.print_gfx(s, false); }
    }

    pub fn println(&mut self, args: &[String]) {
        for s in args { self.print_gfx(s, false); }
        self.print_gfx("", true);
    }

    /// Render a literal string to the framebuffer — used by emitted INPUT prompts.
    pub fn print_str(&mut self, s: &str) {
        self.print_gfx(s, false);
    }

    /// Advance to the next 14-column print zone (QB PRINT comma separator).
    /// Emits spaces to reach the next zone boundary; updates cursor_col.
    pub fn print_zone(&mut self) {
        const ZONE: usize = 14;
        let next_zone = ((self.cursor_col.saturating_sub(1)) / ZONE + 1) * ZONE + 1;
        if next_zone > self.cursor_col {
            let spaces = " ".repeat(next_zone - self.cursor_col);
            self.print_gfx(&spaces, false);
        }
    }

    pub fn tab(&self, col: f64) -> String {
        let target = col as usize;
        if target > self.cursor_col {
            " ".repeat(target - self.cursor_col)
        } else {
            String::new()
        }
    }

    // ── Graphics-mode text rendering ──────────────────────────────────────────

    /// Render a single character glyph at (cursor_col, cursor_row) in the framebuffer.
    /// The glyph bitmap is 8×8; in modes with char_h > 8 (e.g. mode 9 = 14px or
    /// mode 0 = 16px) the 8 source rows are distributed across char_h scan lines
    /// using Bresenham-style integer scaling so the glyph fills the full cell height.
    /// char_h=8 → 1 scan/row (identity); char_h=14 → [1,2,2,2,1,2,2,2] scans;
    /// char_h=16 → 2 scans/row (double-scan, matching CGA/EGA hardware text modes).
    fn draw_char_fb(&mut self, ch: char) {
        let cp = ch as u32;
        let idx = if cp < 256 { cp as usize } else { b'?' as usize };
        let glyph = FONT_8X8[idx];
        let px = ((self.cursor_col as u32).saturating_sub(1)) * self.char_w;
        let py = ((self.cursor_row as u32).saturating_sub(1)) * self.char_h;
        let fg = self.fg_color;
        let bg = self.bg_color;
        let ch = self.char_h;
        // Distribute 8 glyph rows across char_h scan lines via integer Bresenham.
        let mut scan_y = py;
        for grow in 0..8u32 {
            let scan_y_end = py + (grow + 1) * ch / 8;
            let bits = glyph[grow as usize];
            while scan_y < scan_y_end {
                for col in 0..self.char_w {
                    let set = col < 8 && (bits >> (7 - col)) & 1 == 1;
                    let x = px + col;
                    if x < self.width && scan_y < self.height {
                        self.fb[(scan_y * self.width + x) as usize] = if set { fg } else { bg };
                    }
                }
                scan_y += 1;
            }
        }
        // Fill any rounding remainder with background.
        while scan_y < py + ch {
            for col in 0..self.char_w {
                let x = px + col;
                if x < self.width && scan_y < self.height {
                    self.fb[(scan_y * self.width + x) as usize] = bg;
                }
            }
            scan_y += 1;
        }
    }

    /// Advance the text cursor by one column; wrap + scroll as needed.
    fn cursor_advance(&mut self) {
        self.cursor_col += 1;
        let max_col = (self.width / self.char_w) as usize;
        if self.cursor_col > max_col {
            self.cursor_col = 1;
            self.cursor_row += 1;
            self.scroll_if_needed();
        }
    }

    /// Scroll the framebuffer up by one character row if the cursor is past the bottom.
    fn scroll_if_needed(&mut self) {
        let max_rows = (self.height / self.char_h) as u32;
        // Determine the effective scroll region
        let bot = if self.vp_bot > 0 && self.vp_bot <= max_rows {
            self.vp_bot
        } else {
            max_rows
        };
        let top = if self.vp_top >= 1 { self.vp_top } else { 1 };

        if self.cursor_row > bot as usize {
            let w = self.width as usize;
            let ch = self.char_h as usize;
            let top_px  = ((top  - 1) as usize) * ch * w;
            let bot_px  = (bot         as usize) * ch * w;
            let row_px  = ch * w;
            // Scroll the viewport region up by one row
            if bot_px > top_px + row_px && bot_px <= self.fb.len() {
                self.fb.copy_within(top_px + row_px..bot_px, top_px);
            }
            // Clear the last row of the viewport
            let clear_start = (bot_px - row_px).min(self.fb.len());
            let clear_end   = bot_px.min(self.fb.len());
            let bg = self.bg_color;
            self.fb[clear_start..clear_end].iter_mut().for_each(|p| *p = bg);
            self.cursor_row = bot as usize;
        }
    }

    /// Render a string to the framebuffer at the current text cursor.
    /// If `newline` is true, advances to the next row after rendering.
    /// Calls `present()` at the end so text appears immediately.
    fn print_gfx(&mut self, s: &str, newline: bool) {
        // When no explicit SCREEN call has been made the program is in text
        // mode and the integration-test runner captures stdout — echo there so
        // tests keep working.  With a SCREEN call the program is a real
        // graphics program; window-only output is correct and desirable.
        // If the window failed to open (headless environment) always fall back
        // to stdout so tests running in CI still capture output — EXCEPT when the
        // QBC_TEXT_FB driver flag is set, where a graphics program renders its
        // text INTO the framebuffer so a screenshot captures the full screen
        // (score panels, labels, title screens), not just the vector graphics.
        // (Off by default so the graphics golden tests keep their stable,
        // graphics-only checksums and present-count exit policies.)
        //
        // Special case: a headless *graphics* program (had_screen_call=true) with
        // no window and no QBC_TEXT_FB is silently dropped — not stdout (would spam
        // at native CPU speed from tight INKEY$ loops like gorilla's GetNum#
        // cursor-blink PRINT) and not the framebuffer (would corrupt golden checksums
        // by drawing label text into a graphics-only fb snapshot).
        let text_to_fb = self.headless_cfg.as_ref().map_or(false, |c| c.text_to_fb);
        if self.had_screen_call && self.window.is_none() && !text_to_fb {
            return; // headless graphics: suppress text output silently
        }
        let use_stdout = !self.had_screen_call
            || (self.window.is_none() && !text_to_fb);
        if use_stdout {
            if newline {
                println!("{s}");
            } else {
                print!("{s}");
                let _ = std::io::Write::flush(&mut std::io::stdout());
            }
            // If the window IS open, also render there (text-only programs see
            // output in both the terminal and the GUI window).
            // Do NOT update cursor_col here — the window branch below is
            // authoritative for cursor tracking so that the draw position is
            // correct. Without this guard the column would advance twice
            // (once here, once in the window loop), shifting every subsequent
            // character one string-width to the right.
            if self.window.is_none() {
                // Headless: track cursor column for TAB() calculations.
                for ch in s.chars() {
                    match ch {
                        '\n' | '\r' => { self.cursor_col = 1; }
                        _ => { self.cursor_col += 1; }
                    }
                }
                if newline { self.cursor_col = 1; }
                return;
            }
            // Window is open — fall through to the framebuffer render which
            // will draw the text and advance cursor_col correctly.
        }
        for ch in s.chars() {
            match ch {
                '\r' => { self.cursor_col = 1; }
                '\n' => { self.cursor_col = 1; self.cursor_row += 1; }
                _ => {
                    self.draw_char_fb(ch);
                    self.cursor_advance();
                }
            }
        }
        if newline {
            self.cursor_col = 1;
            self.cursor_row += 1;
            self.scroll_if_needed();
        }
        self.present();
    }

    // ── Input ─────────────────────────────────────────────────────────────────

    /// Blocking line input with echo to the framebuffer window.
    pub fn input_line(&mut self) -> String {
        // Headless: assemble the line from the scripted key queue up to ENTER.
        // If the queue empties first, the program is blocked on input — finish.
        if self.headless_cfg.is_some() {
            let mut line = String::new();
            loop {
                match self.key_queue.pop_front() {
                    Some(k) if k == "\r" || k == "\n" => return line,
                    Some(k) => line.push_str(&k),
                    None    => self.headless_finish(),
                }
            }
        }

        let mut buf        = String::new();
        let mut blink      = true;
        let mut last_blink = std::time::Instant::now();

        loop {
            // ── Blink cursor ──────────────────────────────────────────────────
            if last_blink.elapsed() >= std::time::Duration::from_millis(500) {
                blink = !blink;
                last_blink = std::time::Instant::now();
                let (sr, sc, sf) = (self.cursor_row, self.cursor_col, self.fg_color);
                self.fg_color = if blink { sf } else { self.bg_color };
                self.draw_char_fb('\u{7F}'); // IBM font: DEL = solid block
                self.cursor_row = sr;
                self.cursor_col = sc;
                self.fg_color   = sf;
            }

            // ── Pump OS events (MUST happen every frame for keys to register) ─
            self.present();

            // ── Read keys ─────────────────────────────────────────────────────
            let keys: Vec<Key> = match self.window.as_mut() {
                Some(w) => { if !w.is_open() { std::process::exit(0); } w.get_keys_pressed(KeyRepeat::Yes) }
                None    => vec![],
            };
            let shift = self.window.as_ref()
                .map(|w| w.is_key_down(Key::LeftShift) || w.is_key_down(Key::RightShift))
                .unwrap_or(false);

            for key in keys {
                match key {
                    Key::Enter => {
                        // Erase cursor, newline, done
                        let sf = self.fg_color;
                        self.fg_color = self.bg_color;
                        self.draw_char_fb(' ');
                        self.fg_color = sf;
                        self.print_gfx("", true);
                        return buf;
                    }
                    Key::Backspace => {
                        if !buf.is_empty() {
                            buf.pop();
                            if self.cursor_col > 1 { self.cursor_col -= 1; }
                            let sf = self.fg_color;
                            self.fg_color = self.bg_color;
                            self.draw_char_fb(' ');
                            self.fg_color = sf;
                        }
                    }
                    Key::Escape => {}
                    k => {
                        if let Some(ch) = window_key_to_char(k, shift) {
                            buf.push(ch);
                            self.draw_char_fb(ch);
                            self.cursor_advance();
                        }
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }

    // ── File I/O ─────────────────────────────────────────────────────────────

    /// OPEN path FOR INPUT|OUTPUT|APPEND AS #n
    pub fn open_seq(&mut self, path: &str, mode: &str, file_num: u8) {
        let result = match mode {
            "input" => std::fs::File::open(path).ok().map(|f| QbFile::Sequential {
                reader: Some(std::io::BufReader::new(f)),
                writer: None,
            }),
            "output" => std::fs::File::create(path).ok().map(|f| QbFile::Sequential {
                reader: None,
                writer: Some(std::io::BufWriter::new(f)),
            }),
            "append" => std::fs::OpenOptions::new().append(true).create(true).open(path)
                .ok().map(|f| QbFile::Sequential {
                    reader: None,
                    writer: Some(std::io::BufWriter::new(f)),
                }),
            _ => None,
        };
        if let Some(fh) = result {
            self.files.insert(file_num, fh);
        } else {
            // File open failed — QB error 53 = "file not found"
            self.err_code = 53.0;
            self.error_pending = true;
        }
    }

    /// QB `ERR` — the code of the most recent trappable error (0 = none).
    /// Provided as a method so the emitter's generic zero-arg call path
    /// (`ERR` → `__rt.err_code()`) compiles; the bare field is also read
    /// directly at other emission sites.
    pub fn err_code(&self) -> f64 {
        self.err_code
    }

    /// OPEN path FOR RANDOM AS #n LEN = rec_len
    pub fn open_random(&mut self, path: &str, file_num: u8, record_len: usize) {
        if let Ok(file) = std::fs::OpenOptions::new()
            .read(true).write(true).create(true).open(path)
        {
            self.files.insert(file_num, QbFile::Random {
                file, record_len, cur_record: 0,
            });
        } else {
            self.err_code = 53.0;
            self.error_pending = true;
        }
    }

    /// CLOSE #n
    pub fn close_file(&mut self, file_num: u8) {
        self.files.remove(&file_num);
    }

    /// CLOSE (all)
    pub fn close_all(&mut self) {
        self.files.clear();
    }

    /// For FIELD statement: just records how many bytes this file's records are.
    /// The actual field layout is handled in emitted code.
    pub fn set_field(&mut self, _file_num: u8, _total_len: usize) {}

    /// GET #n, recnum — reads a fixed-length record (0-based index) into a Vec<u8>.
    /// Returns a vec of `record_len` bytes (space-padded if short/missing).
    pub fn read_record(&mut self, file_num: u8, rec_idx: Option<i64>) -> Vec<u8> {
        if let Some(QbFile::Random { file, record_len, cur_record }) =
            self.files.get_mut(&file_num)
        {
            let idx = match rec_idx {
                Some(n) => { *cur_record = n; n }
                None    => { let n = *cur_record; *cur_record += 1; n }
            };
            let offset = (idx as u64) * (*record_len as u64);
            let rlen = *record_len;
            let _ = file.seek(SeekFrom::Start(offset));
            let mut buf = vec![b' '; rlen];
            let _ = std::io::Read::read(file, &mut buf);
            buf
        } else {
            Vec::new()
        }
    }

    /// PUT #n, recnum — writes a fixed-length record at the given 0-based index.
    pub fn write_record(&mut self, file_num: u8, rec_idx: Option<i64>, data: &[u8]) {
        if let Some(QbFile::Random { file, record_len, cur_record }) =
            self.files.get_mut(&file_num)
        {
            let idx = match rec_idx {
                Some(n) => { *cur_record = n; n }
                None    => { let n = *cur_record; *cur_record += 1; n }
            };
            let offset = (idx as u64) * (*record_len as u64);
            let rlen = *record_len;
            let _ = file.seek(SeekFrom::Start(offset));
            // Pad or truncate data to exactly record_len bytes
            let mut buf = vec![b' '; rlen];
            let copy_len = data.len().min(rlen);
            buf[..copy_len].copy_from_slice(&data[..copy_len]);
            let _ = file.write_all(&buf);
            let _ = file.flush();
        }
    }

    /// INPUT #n — read one line from a sequential file (strips trailing \n/\r).
    pub fn read_file_line(&mut self, file_num: u8) -> String {
        if let Some(QbFile::Sequential { reader: Some(r), .. }) =
            self.files.get_mut(&file_num)
        {
            let mut line = String::new();
            let _ = r.read_line(&mut line);
            line.trim_end_matches(['\n', '\r']).to_string()
        } else {
            String::new()
        }
    }

    /// PRINT #n / WRITE #n — write a string to a sequential file.
    pub fn write_file(&mut self, file_num: u8, s: &str) {
        if let Some(QbFile::Sequential { writer: Some(w), .. }) =
            self.files.get_mut(&file_num)
        {
            let _ = w.write_all(s.as_bytes());
        }
    }

    /// EOF(n) — true if at end of sequential file or file not open.
    pub fn eof_check(&self, file_num: u8) -> bool {
        // For sequential files we check if the reader is exhausted.
        // BufReader has no reliable is_eof; use fill_buf to check.
        true // conservative: most programs only check EOF in loops; let read return "" to signal it
    }

    // ── DATA / READ ───────────────────────────────────────────────────────────

    pub fn read_data(&self) -> String { String::new() }

    // ── RNG ───────────────────────────────────────────────────────────────────

    pub fn randomize(&mut self, seed: f64) {
        // QBC_SEED locks the RNG so a `RANDOMIZE TIMER` can't perturb a
        // deterministic headless render (golden tests).
        if self.seed_locked { return; }
        // QB mixes the seed into bits 8-23 of the 24-bit state, preserving
        // the low byte (we fold the f32 bit pattern like QB folds the FP
        // accumulator's middle words).
        let b = (seed as f32).to_bits();
        let m = ((b >> 16) ^ (b & 0xFFFF)) & 0xFFFF;
        self.rng = (self.rng & 0xFF) | (m << 8);
    }

    /// QBasic's 24-bit LCG: x = (x*16598013 + 12820163) AND &HFFFFFF,
    /// RND = x / 2^24. Same sequence as DOS QBasic 1.1 for the same seed.
    pub fn rnd(&mut self) -> f64 {
        self.rng = self.rng.wrapping_mul(16598013).wrapping_add(12820163) & 0xFF_FFFF;
        self.last_rnd = self.rng as f64 / 16777216.0;
        self.last_rnd
    }

    /// RND with an argument: negative reseeds, 0 repeats the last value,
    /// positive returns the next value (QB semantics).
    pub fn rnd_arg(&mut self, v: f64) -> f64 {
        if v < 0.0 {
            if !self.seed_locked {
                let b = (v as f32).to_bits();
                self.rng = (b ^ (b >> 8)) & 0xFF_FFFF;
            }
            self.rnd()
        } else if v == 0.0 {
            self.last_rnd
        } else {
            self.rnd()
        }
    }

    // ── Graphics ──────────────────────────────────────────────────────────────

    /// Called from pset() to automatically present at ~60fps during any
    /// animation loop that writes pixels, with no explicit present() needed
    /// Set fullspeed mode — skip the auto_present() frame throttle entirely.
    /// Activated by `REM QBC FULLSPEED` in the source file.
    pub fn set_fullspeed(&mut self, v: bool) { self.fullspeed = v; }

    /// Set target frame rate for auto_present() throttle (default 60 fps).
    /// Activated by `REM QBC FPS n` in the source file.
    pub fn set_fps(&mut self, fps: f64) {
        self.frame_interval_ms = (1000.0 / fps.max(1.0)) as u64;
    }

    /// Enable paced rendering at `fps` blits/second. Unlike the normal throttle
    /// (which only *skips* blits that come too soon and never blocks), pacing
    /// makes auto_present() *sleep* the remainder of each frame interval — which
    /// blocks the compute and so makes a fast native draw watchable, sweeping in
    /// roughly the source's drawing order. Activated by `REM QBC PACE n`.
    /// `n <= 0` disables it. The total run time scales with how much the program
    /// draws, so tune `n` until the sweep looks right (lower = slower).
    pub fn set_pace(&mut self, fps: f64) {
        self.pace_ms = if fps > 0.0 { (1000.0 / fps) as u64 } else { 0 };
    }

    /// Set SLEEP duration multiplier (default 1.0 = normal speed).
    /// Activated by `REM QBC SLOWMO n` in the source file.
    pub fn set_slowmo(&mut self, factor: f64) {
        self.slowmo = factor.max(0.0);
    }

    /// in the emitted code.
    #[inline]
    fn auto_present(&mut self) {
        if self.fullspeed { return; }
        self.pset_counter = self.pset_counter.wrapping_add(1);
        if self.pace_ms > 0 {
            // Paced mode (REM QBC PACE n): hold a steady blit cadence by
            // SLEEPING the remainder of each interval, which blocks — and
            // therefore paces — the computation so the draw is watchable. A
            // finer gate (every 64 calls) than the default keeps the sweep
            // smooth; the per-interval sleep dominates the wall-clock time.
            if self.pset_counter & 0x3F == 0 {
                let target = std::time::Duration::from_millis(self.pace_ms);
                let elapsed = self.last_present.elapsed();
                if elapsed < target { std::thread::sleep(target - elapsed); }
                self.present();
                self.last_present = std::time::Instant::now();
            }
            return;
        }
        if self.pset_counter & 0xFF == 0 { // check every 256 psets
            let now = std::time::Instant::now();
            if now.duration_since(self.last_present) >= std::time::Duration::from_millis(self.frame_interval_ms) {
                self.present();
                self.last_present = now;
            }
        }
    }

    /// Process OS window events without blitting — keeps the window alive
    /// during long computations (flood fill, circle drawing, etc.).
    fn tick(&mut self) {
        if let Some(win) = self.window.as_mut() {
            win.update();
            if !win.is_open() {
                self.window = None;
                std::process::exit(0);
            }
        }
    }

    /// Pump OS window events and harvest keypresses into `key_queue` WITHOUT
    /// rebuilding or blitting the framebuffer. Cheap enough to call on every
    /// `INKEY$` poll: minifb's rate limiter is disabled at window creation, so
    /// `update()` does not sleep, and we skip the 2.3 MB alloc + full-frame
    /// rebuild that `present()` does. Used by `inkey()` between throttled blits.
    fn pump_events(&mut self) {
        let new_keys: Vec<Key> = {
            let win = match self.window.as_mut() { Some(w) => w, None => return };
            win.update();
            if !win.is_open() { std::process::exit(0); }
            win.get_keys_pressed(KeyRepeat::No)
        };
        for key in new_keys {
            let s = minifb_key_to_qb(key);
            if !s.is_empty() { self.key_queue.push_back(s); }
        }
    }

    /// Flush the palette-indexed framebuffer to the minifb window.
    /// Software nearest-neighbor scales the fb (any size) into the fixed 960×600 window.
    pub fn present(&mut self) {
        // Headless driver: drives the dump/exit policies (and may terminate here).
        self.headless_tick();
        if self.window.is_none() { return; }
        let fw = self.width  as usize;
        let fh = self.height as usize;
        let palette = self.palette_rgb;
        let win_w = self.win_w;
        let win_h = self.win_h;
        // Render into the PERSISTENT buffer (not a local Vec): minifb's macOS
        // backend keeps the raw pointer and re-reads it on later update() calls,
        // so the buffer must outlive every event pump (see `present_buf` doc).
        if self.present_buf.len() != win_w * win_h {
            self.present_buf.resize(win_w * win_h, 0);
        }
        for oy in 0..win_h {
            let fy = (oy * fh) / win_h;
            let row_base = fy * fw;
            let out_base = oy * win_w;
            for ox in 0..win_w {
                let fx = (ox * fw) / win_w;
                let (r, g, b) = palette[self.fb[row_base + fx] as usize];
                self.present_buf[out_base + ox] = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
            }
        }
        // Borrow window in a nested block so it's released before we touch key_queue.
        // get_keys_pressed returns owned Vec<Key> so new_keys doesn't borrow the window.
        let new_keys: Vec<Key> = {
            let out = &self.present_buf;
            let win = self.window.as_mut().unwrap();
            let _ = win.update_with_buffer(out, win_w, win_h);
            if !win.is_open() { std::process::exit(0); }
            win.get_keys_pressed(KeyRepeat::No)
        }; // window borrow released here
        // Harvest into key_queue so keys are never lost to intervening present() calls.
        for key in new_keys {
            let s = minifb_key_to_qb(key);
            if !s.is_empty() {
                self.key_queue.push_back(s);
            }
        }
    }

    /// Apply one decoded sprite pixel `color` to framebuffer index `fb_idx`
    /// using the QB PUT combine verb. `mask` is the mode's color depth (for the
    /// PRESET inversion). Shared by the EGA and CGA blit paths.
    #[inline]
    fn put_pixel(&mut self, fb_idx: usize, color: u8, mask: u8, action: PutAction) {
        match action {
            PutAction::Pset   => self.fb[fb_idx] = color,
            PutAction::Preset => self.fb[fb_idx] = (!color) & mask,
            PutAction::And    => self.fb[fb_idx] &= color,
            PutAction::Or     => self.fb[fb_idx] |= color,
            PutAction::Xor    => self.fb[fb_idx] ^= color,
        }
    }

    /// PUT (x, y), array, verb — blit a sprite from a QB sprite array.
    /// `action` selects the QB combine verb (PSET/PRESET/AND/OR/XOR); QB's
    /// default verb (no keyword) is XOR. CGA SCREEN 1 uses the authentic 2-bpp
    /// packed INTEGER-array layout; every other mode uses the EGA planar layout.
    pub fn put_sprite(&mut self, data: &[f64], gx: f64, gy: f64, action: PutAction) {
        self.put_sprite_at(data, gx, gy, action, 0);
    }

    /// PUT with a sprite stored at a non-zero element `offset` into `data`.
    /// QB packs multiple sprites into one array (`PUT (x,y), Arr(n)`); the sprite
    /// header lives at `data[offset]`. `put_sprite` is the `offset == 0` case.
    pub fn put_sprite_at(&mut self, data: &[f64], gx: f64, gy: f64, action: PutAction, offset: usize) {
        if self.screen_mode == 0 || offset >= data.len() { return; }
        if self.screen_mode == 1 {
            self.put_sprite_cga(data, gx, gy, action, offset);
            return;
        }
        if self.screen_mode == 13 {
            self.put_sprite_mode13(data, gx, gy, action, offset);
            return;
        }
        let header = data[offset] as i64 as u32;
        let width  = ((header & 0xFFFF) + 1) as i32;
        let height = (((header >> 16) & 0xFFFF) + 1) as i32;
        let bytes_per_plane = ((width as usize) + 7) / 8;
        // Bytes per row = 4 planes × bytes_per_plane, packed into Longs (4 bytes each)
        // longs_per_row = ceil(4 * bytes_per_plane / 4) = bytes_per_plane
        let longs_per_row = bytes_per_plane;
        let mask = self.sprite_color_mask(); // for PRESET inversion within bpp
        let gx = gx as i32;
        let gy = gy as i32;
        for row in 0..height {
            let sy = gy + row;
            if sy < 0 || sy as u32 >= self.height { continue; }
            let long_start = offset + 1 + row as usize * longs_per_row;
            if long_start + longs_per_row > data.len() { break; }
            // Unpack longs into bytes: byte layout is [p0b0..p0bN, p1b0..p1bN, p2b0..p2bN, p3b0..p3bN]
            let mut row_bytes = vec![0u8; longs_per_row * 4];
            for i in 0..longs_per_row {
                let v = data[long_start + i] as i64 as u32;
                row_bytes[i * 4 + 0] = (v & 0xFF) as u8;
                row_bytes[i * 4 + 1] = ((v >> 8) & 0xFF) as u8;
                row_bytes[i * 4 + 2] = ((v >> 16) & 0xFF) as u8;
                row_bytes[i * 4 + 3] = ((v >> 24) & 0xFF) as u8;
            }
            for col in 0..width {
                let sx = gx + col;
                if sx < 0 || sx as u32 >= self.width { continue; }
                let byte_idx = col as usize / 8;
                let bit_pos  = 7 - (col as usize % 8);
                let p0 = (row_bytes[byte_idx] >> bit_pos) & 1;
                let p1 = (row_bytes[bytes_per_plane     + byte_idx] >> bit_pos) & 1;
                let p2 = (row_bytes[bytes_per_plane * 2 + byte_idx] >> bit_pos) & 1;
                let p3 = (row_bytes[bytes_per_plane * 3 + byte_idx] >> bit_pos) & 1;
                let color = p0 | (p1 << 1) | (p2 << 2) | (p3 << 3);
                let fb_idx = (sy as u32 * self.width + sx as u32) as usize;
                self.put_pixel(fb_idx, color, mask, action);
            }
        }
        // PUT is a sprite-level operation (typically 1–2 per animation frame),
        // not a pixel-level one — always blit immediately so sprite animation
        // (banana flight, gorilla arm raise) is visible.
        self.present();
    }

    /// PUT for CGA SCREEN 1 (2 bits/pixel, packed — not planar).
    /// Layout: data[0] = width_px*2, data[1] = height_px, then a byte stream of
    /// `ceil(width/4)` bytes/row (4 pixels/byte, MSB-first), two bytes per
    /// INTEGER element (little-endian within the 16-bit word).
    fn put_sprite_cga(&mut self, data: &[f64], gx: f64, gy: f64, action: PutAction, offset: usize) {
        if data.len() < offset + 2 { return; }
        let width  = (data[offset] as i64 as u16 as usize) / 2;
        let height = data[offset + 1] as i64 as u16 as usize;
        if width == 0 || height == 0 { return; }
        let bytes_per_row = (width + 3) / 4;            // 4 pixels per byte
        let mask = self.sprite_color_mask();            // = 3 in mode 1
        let gx = gx as i32;
        let gy = gy as i32;
        // Element-indexed byte fetch: byte b lives in element offset + 2 + b/2,
        // low byte first (x86 little-endian within each 16-bit INTEGER).
        let get_byte = |b: usize| -> u8 {
            let elem = offset + 2 + b / 2;
            if elem >= data.len() { return 0; }
            let w = data[elem] as i64 as u16;
            if b & 1 == 0 { (w & 0xFF) as u8 } else { (w >> 8) as u8 }
        };
        for row in 0..height {
            let sy = gy + row as i32;
            if sy < 0 || sy as u32 >= self.height { continue; }
            let row_byte0 = row * bytes_per_row;
            for col in 0..width {
                let sx = gx + col as i32;
                if sx < 0 || sx as u32 >= self.width { continue; }
                let byte = get_byte(row_byte0 + col / 4);
                let shift = 6 - 2 * (col % 4);
                let color = (byte >> shift) & 3;
                let fb_idx = (sy as u32 * self.width + sx as u32) as usize;
                self.put_pixel(fb_idx, color, mask, action);
            }
        }
        self.present();
    }

    /// GET (x1,y1)-(x2,y2), array — capture a screen region into a QB sprite
    /// array. CGA SCREEN 1 uses the authentic 2-bpp packed INTEGER layout (so a
    /// later PUT can blit it and hand-built arrays interoperate); every other
    /// mode uses the EGA planar layout.
    pub fn get_sprite(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, data: &mut Vec<f64>) {
        self.get_sprite_at(x1, y1, x2, y2, data, 0);
    }

    /// GET storing the captured sprite at a non-zero element `offset` into `data`.
    /// QB packs multiple sprites into one array (`GET …, Arr(n)`); the header is
    /// written at `data[offset]`. `get_sprite` is the `offset == 0` case.
    ///
    /// For `offset == 0` the array is resized to the exact sprite size (the
    /// historical behavior — keeps all existing callers byte-identical). For
    /// `offset > 0` the resize is **grow-only**, so earlier sprites packed at
    /// lower offsets are never clobbered.
    pub fn get_sprite_at(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, data: &mut Vec<f64>, offset: usize) {
        if self.screen_mode == 0 { return; }
        if self.screen_mode == 1 {
            self.get_sprite_cga(x1, y1, x2, y2, data, offset);
            return;
        }
        if self.screen_mode == 13 {
            self.get_sprite_mode13(x1, y1, x2, y2, data, offset);
            return;
        }
        let x1 = x1 as i32;
        let y1 = y1 as i32;
        let x2 = x2 as i32;
        let y2 = y2 as i32;
        let width  = ((x2 - x1 + 1).max(1)) as usize;
        let height = ((y2 - y1 + 1).max(1)) as usize;
        let bytes_per_plane = (width + 7) / 8;
        let longs_per_row   = bytes_per_plane;
        let total_longs     = 1 + height * longs_per_row;
        if offset == 0 {
            data.resize(total_longs, 0.0);
        } else {
            let need = offset + total_longs;
            if data.len() < need { data.resize(need, 0.0); }
        }
        let header = ((width as u32 - 1) | ((height as u32 - 1) << 16)) as i32;
        data[offset] = header as f64;
        for row in 0..height {
            let sy = y1 + row as i32;
            let mut row_bytes = vec![0u8; longs_per_row * 4];
            for col in 0..width {
                let sx = x1 + col as i32;
                if sx < 0 || sy < 0 || sx as u32 >= self.width || sy as u32 >= self.height { continue; }
                let color = self.fb[(sy as u32 * self.width + sx as u32) as usize];
                let byte_idx = col / 8;
                let bit_pos  = 7 - (col % 8);
                for p in 0..4u8 {
                    if (color >> p) & 1 != 0 {
                        row_bytes[(p as usize) * bytes_per_plane + byte_idx] |= 1 << bit_pos;
                    }
                }
            }
            let long_start = offset + 1 + row * longs_per_row;
            for i in 0..longs_per_row {
                let v = (row_bytes[i * 4]     as u32)        |
                        ((row_bytes[i * 4 + 1] as u32) << 8) |
                        ((row_bytes[i * 4 + 2] as u32) << 16)|
                        ((row_bytes[i * 4 + 3] as u32) << 24);
                data[long_start + i] = (v as i32) as f64;
            }
        }
    }

    /// GET for CGA SCREEN 1 — capture into the authentic 2-bpp packed INTEGER
    /// layout (see `put_sprite_cga`). Symmetric with the CGA PUT path.
    fn get_sprite_cga(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, data: &mut Vec<f64>, offset: usize) {
        let x1 = x1 as i32;
        let y1 = y1 as i32;
        let x2 = x2 as i32;
        let y2 = y2 as i32;
        let width  = ((x2 - x1 + 1).max(1)) as usize;
        let height = ((y2 - y1 + 1).max(1)) as usize;
        let bytes_per_row = (width + 3) / 4;
        let total_bytes   = height * bytes_per_row;
        let total_elems   = 2 + (total_bytes + 1) / 2; // 2-byte header words + data
        if offset == 0 {
            data.clear();
            data.resize(total_elems, 0.0);
        } else {
            let need = offset + total_elems;
            if data.len() < need { data.resize(need, 0.0); }
        }
        data[offset] = (width * 2) as f64;
        data[offset + 1] = height as f64;
        // Build the packed byte stream, then fold pairs into 16-bit elements.
        let mut bytes = vec![0u8; total_bytes];
        for row in 0..height {
            let sy = y1 + row as i32;
            for col in 0..width {
                let sx = x1 + col as i32;
                if sx < 0 || sy < 0 || sx as u32 >= self.width || sy as u32 >= self.height { continue; }
                let color = self.fb[(sy as u32 * self.width + sx as u32) as usize] & 3;
                let b = row * bytes_per_row + col / 4;
                let shift = 6 - 2 * (col % 4);
                bytes[b] |= color << shift;
            }
        }
        for (i, chunk) in bytes.chunks(2).enumerate() {
            let lo = chunk[0] as u16;
            let hi = *chunk.get(1).unwrap_or(&0) as u16;
            data[offset + 2 + i] = (lo | (hi << 8)) as i16 as f64;
        }
    }

    /// PUT for MCGA SCREEN 13 (256-color, 8 bits/pixel — linear "chunky", not
    /// planar). Layout: data[0] = width_px*8 (x-extent in bits), data[1] =
    /// height_px, then a byte stream of `width` bytes/row (one full color index
    /// per pixel), two bytes per INTEGER element (low byte first).
    fn put_sprite_mode13(&mut self, data: &[f64], gx: f64, gy: f64, action: PutAction, offset: usize) {
        if data.len() < offset + 2 { return; }
        let width  = (data[offset] as i64 as u16 as usize) / 8;
        let height = data[offset + 1] as i64 as u16 as usize;
        if width == 0 || height == 0 { return; }
        let mask = self.sprite_color_mask(); // = 255 in mode 13
        let gx = gx as i32;
        let gy = gy as i32;
        // byte b lives in element offset + 2 + b/2, low byte first (little-endian INTEGER).
        let get_byte = |b: usize| -> u8 {
            let elem = offset + 2 + b / 2;
            if elem >= data.len() { return 0; }
            let w = data[elem] as i64 as u16;
            if b & 1 == 0 { (w & 0xFF) as u8 } else { (w >> 8) as u8 }
        };
        for row in 0..height {
            let sy = gy + row as i32;
            if sy < 0 || sy as u32 >= self.height { continue; }
            let row_byte0 = row * width;
            for col in 0..width {
                let sx = gx + col as i32;
                if sx < 0 || sx as u32 >= self.width { continue; }
                let color = get_byte(row_byte0 + col); // full 0–255 index
                let fb_idx = (sy as u32 * self.width + sx as u32) as usize;
                self.put_pixel(fb_idx, color, mask, action);
            }
        }
        self.present();
    }

    /// GET for MCGA SCREEN 13 — capture into the 8-bpp chunky INTEGER layout
    /// (see `put_sprite_mode13`). Symmetric with the mode-13 PUT path.
    fn get_sprite_mode13(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, data: &mut Vec<f64>, offset: usize) {
        let x1 = x1 as i32;
        let y1 = y1 as i32;
        let x2 = x2 as i32;
        let y2 = y2 as i32;
        let width  = ((x2 - x1 + 1).max(1)) as usize;
        let height = ((y2 - y1 + 1).max(1)) as usize;
        let total_bytes = width * height;                 // 1 byte per pixel
        let total_elems = 2 + (total_bytes + 1) / 2;       // 2-word header + data
        if offset == 0 {
            data.clear();
            data.resize(total_elems, 0.0);
        } else {
            let need = offset + total_elems;
            if data.len() < need { data.resize(need, 0.0); }
        }
        data[offset] = (width * 8) as f64;                 // x-extent in bits
        data[offset + 1] = height as f64;
        let mut bytes = vec![0u8; total_bytes];
        for row in 0..height {
            let sy = y1 + row as i32;
            for col in 0..width {
                let sx = x1 + col as i32;
                if sx < 0 || sy < 0 || sx as u32 >= self.width || sy as u32 >= self.height { continue; }
                // Store the FULL 8-bit color index (no masking to 4 bits).
                bytes[row * width + col] = self.fb[(sy as u32 * self.width + sx as u32) as usize];
            }
        }
        for (i, chunk) in bytes.chunks(2).enumerate() {
            let lo = chunk[0] as u16;
            let hi = *chunk.get(1).unwrap_or(&0) as u16;
            data[offset + 2 + i] = (lo | (hi << 8)) as i16 as f64;
        }
    }

    // ── Coordinate-system helpers ─────────────────────────────────────────────

    /// Map logical (or VIEW-relative) coords to framebuffer pixel coords.
    /// Called by pset/line when VIEW or WINDOW is active.
    ///
    /// The result is **rounded to the nearest pixel**. Every caller converts the
    /// returned coords to `i32`, and a bare `as i32` truncates toward zero — but
    /// the PMAP/WINDOW round-trip produces values like `5.99999999` that must map
    /// to pixel 6, not 5. Truncating there drops whole scanlines, which shows up
    /// as horizontal black gaps in line-per-scanline renderers such as mandel.bas
    /// (and is wrong in general: QB maps fractional coords to the nearest pixel).
    /// Effective graphics viewport in framebuffer pixels. With VIEW active it's
    /// the explicit VIEW rect; otherwise QB's default viewport = the whole screen.
    /// WINDOW without an explicit VIEW maps onto the full screen (DOS QB behavior).
    fn effective_viewport(&self) -> (f64, f64, f64, f64) {
        if self.view_active {
            (self.view_x1, self.view_y1, self.view_x2, self.view_y2)
        } else {
            (0.0, 0.0,
             self.width.saturating_sub(1) as f64,
             self.height.saturating_sub(1) as f64)
        }
    }

    fn logical_to_fb(&self, lx: f64, ly: f64) -> (f64, f64) {
        let (px, py) = if self.win_active {
            // WINDOW defines a logical rect mapped onto the (effective) VIEW rect.
            // Without an explicit VIEW the viewport is the entire screen.
            let (vx1, vy1, vx2, vy2) = self.effective_viewport();
            if self.win_screen {
                // `WINDOW SCREEN`: screen orientation (Y increases downward, no
                // inversion). Map by coordinate magnitude so corner order does NOT
                // flip the image — reversi passes (640,480)-(0,0) but expects an
                // identity-style mapping (min → top-left, max → bottom-right).
                let (wxlo, wxhi) = (self.win_x1.min(self.win_x2), self.win_x1.max(self.win_x2));
                let (wylo, wyhi) = (self.win_y1.min(self.win_y2), self.win_y1.max(self.win_y2));
                let px = vx1 + (lx - wxlo) / (wxhi - wxlo) * (vx2 - vx1);
                let py = vy1 + (ly - wylo) / (wyhi - wylo) * (vy2 - vy1);
                (px, py)
            } else {
                // Plain `WINDOW` (no SCREEN) inverts Y: larger logical y → higher
                // on screen (win_y1 → bottom, win_y2 → top).
                let px = vx1 + (lx - self.win_x1) / (self.win_x2 - self.win_x1) * (vx2 - vx1);
                let py = vy1 + (self.win_y2 - ly) / (self.win_y2 - self.win_y1) * (vy2 - vy1);
                (px, py)
            }
        } else if self.view_active {
            // No WINDOW: coords are VIEW-relative; offset by view origin
            (lx + self.view_x1, ly + self.view_y1)
        } else {
            (lx, ly)
        };
        (px.round(), py.round())
    }

    /// Write directly to the framebuffer pixel (no coordinate transform, no cursor update).
    fn pset_raw(&mut self, fx: i32, fy: i32, color: f64) {
        if fx >= 0 && fy >= 0 && (fx as u32) < self.width && (fy as u32) < self.height {
            let m = self.color_mod();
            self.fb[(fy as u32 * self.width + fx as u32) as usize] =
                (color as i64).rem_euclid(m) as u8;
        }
    }

    /// Current graphics cursor (QB "last point referenced") in logical coords.
    /// Used by the emitter to resolve STEP (relative) coordinates.
    pub fn cur_x(&self) -> f64 { self.gfx_x }
    pub fn cur_y(&self) -> f64 { self.gfx_y }

    /// Diagnostic: (non-background pixel count, number of distinct colors present).
    /// Background = color index 0. Used by headless render checks.
    pub fn fb_stats(&self) -> (usize, usize) {
        let mut seen = [false; 256];
        let mut nonzero = 0usize;
        for &p in &self.fb {
            if p != 0 { nonzero += 1; }
            seen[p as usize] = true;
        }
        (nonzero, seen.iter().filter(|&&s| s).count())
    }

    /// Native-resolution framebuffer as RGB triples (palette-resolved). This is
    /// the same palette lookup `present()` does, factored out for image export.
    pub fn fb_to_rgb(&self) -> Vec<(u8, u8, u8)> {
        self.fb.iter().map(|&i| self.palette_rgb[i as usize]).collect()
    }

    /// Write the framebuffer as a binary P6 PPM at native resolution.
    pub fn export_ppm(&self, path: &str) -> std::io::Result<()> {
        use std::io::Write;
        let (w, h) = (self.width as usize, self.height as usize);
        let mut buf = Vec::with_capacity(w * h * 3 + 32);
        buf.extend_from_slice(format!("P6\n{w} {h}\n255\n").as_bytes());
        for &i in &self.fb {
            let (r, g, b) = self.palette_rgb[i as usize];
            buf.push(r); buf.push(g); buf.push(b);
        }
        std::fs::File::create(path)?.write_all(&buf)
    }

    /// Deterministic fingerprint of the visible image (framebuffer + palette),
    /// for golden-image regression tests.
    pub fn fb_checksum(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.width.hash(&mut h);
        self.height.hash(&mut h);
        self.fb.hash(&mut h);
        // Only the in-use palette entries matter, but hashing all 256 is simplest
        // and still deterministic.
        for c in self.palette_rgb.iter() { c.hash(&mut h); }
        h.finish()
    }

    /// Push a scripted key onto the input queue (headless driver / tests).
    pub fn inject_key(&mut self, key: &str) {
        self.key_queue.push_back(key.to_string());
    }

    /// Headless exit funnel: honor `QBC_DUMP`/`QBC_CHECKSUM`/`QBC_FBSTATS`, then
    /// terminate. Called from the exit-policy checks, blocking-input guards, and
    /// `quit()`. No-op (returns) when not in headless mode.
    fn headless_finish(&mut self) -> ! {
        if let Some(cfg) = self.headless_cfg.take() {
            if let Some(path) = &cfg.dump_path {
                let _ = self.export_ppm(path);
            }
            if cfg.checksum {
                println!("QBC_CHECKSUM={:016x}", self.fb_checksum());
            }
            if cfg.fbstats {
                let (nz, nc) = self.fb_stats();
                eprintln!("QBC_FBSTATS nonzero={nz} colors={nc}");
            }
        }
        std::process::exit(0);
    }

    /// Headless bookkeeping called from present()/auto_present(): handles the
    /// `QBC_DUMP_AT` interval dump and the `QBC_EXIT_AFTER` presents/ms policies
    /// plus the wall-clock safety cap. Returns immediately when not headless.
    fn headless_tick(&mut self) {
        if self.headless_cfg.is_none() { return; }
        self.present_count += 1;
        // Copy out the small cfg fields so no borrow of headless_cfg is held
        // across the `&self`/`&mut self` method calls below.
        let (dump_at, exit_after, start, safety_ms, dumped, dump_path) = {
            let c = self.headless_cfg.as_ref().unwrap();
            (c.dump_at, c.exit_after, c.start, c.safety_ms, c.dumped, c.dump_path.clone())
        };
        let elapsed_ms = start.elapsed().as_millis() as u64;
        // Interval dump (does not exit).
        let do_dump = match dump_at {
            DumpAt::Present(n) => self.present_count >= n,
            DumpAt::Ms(t)      => elapsed_ms >= t,
            DumpAt::Exit       => false,
        };
        if do_dump && !dumped {
            if let Some(c) = self.headless_cfg.as_mut() { c.dumped = true; }
            if let Some(path) = dump_path { let _ = self.export_ppm(&path); }
        }
        // Exit policies + safety cap.
        let should_exit = elapsed_ms >= safety_ms || match exit_after {
            ExitAfter::Presents(n) => self.present_count >= n,
            ExitAfter::Ms(t)       => elapsed_ms >= t,
            ExitAfter::Idle        => false,
        };
        if should_exit { self.headless_finish(); }
    }

    pub fn pset(&mut self, x: f64, y: f64, color: f64) {
        self.gfx_x = x;
        self.gfx_y = y;
        let (fx, fy) = self.logical_to_fb(x, y);
        self.pset_raw(fx as i32, fy as i32, color);
        self.auto_present();
    }

    pub fn point(&self, x: f64, y: f64) -> f64 {
        let (fx, fy) = self.logical_to_fb(x, y);
        let (xi, yi) = (fx as i32, fy as i32);
        if xi >= 0 && yi >= 0 && (xi as u32) < self.width && (yi as u32) < self.height {
            self.fb[(yi as u32 * self.width + xi as u32) as usize] as f64
        } else {
            -1.0
        }
    }

    pub fn line(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: f64) {
        self.gfx_x = x2;
        self.gfx_y = y2;
        let (fx1, fy1) = self.logical_to_fb(x1, y1);
        let (fx2, fy2) = self.logical_to_fb(x2, y2);
        bresenham(fx1 as i32, fy1 as i32, fx2 as i32, fy2 as i32, |x, y| {
            self.pset_raw(x, y, color);
        });
        self.auto_present();
    }

    /// Relative LINE — draw from current graphics cursor to (x2,y2).
    pub fn line_to(&mut self, x2: f64, y2: f64, color: f64) {
        let (x1, y1) = (self.gfx_x, self.gfx_y);
        self.line(x1, y1, x2, y2, color);
    }

    pub fn line_box(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: f64) {
        self.line(x1, y1, x2, y1, color);
        self.line(x2, y1, x2, y2, color);
        self.line(x2, y2, x1, y2, color);
        self.line(x1, y2, x1, y1, color);
    }

    pub fn line_box_to(&mut self, x2: f64, y2: f64, color: f64) {
        let (x1, y1) = (self.gfx_x, self.gfx_y);
        self.line_box(x1, y1, x2, y2, color);
    }

    pub fn line_box_fill(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, color: f64) {
        let (fl1, ft1) = self.logical_to_fb(x1, y1);
        let (fl2, ft2) = self.logical_to_fb(x2, y2);
        let (lx, rx) = ((fl1.min(fl2)) as i32, (fl1.max(fl2)) as i32);
        let (ty, by) = ((ft1.min(ft2)) as i32, (ft1.max(ft2)) as i32);
        for y in ty..=by {
            for x in lx..=rx {
                self.pset_raw(x, y, color);
            }
        }
        self.auto_present();
    }

    pub fn line_box_fill_to(&mut self, x2: f64, y2: f64, color: f64) {
        let (x1, y1) = (self.gfx_x, self.gfx_y);
        self.line_box_fill(x1, y1, x2, y2, color);
    }

    // ── VIEW / WINDOW / PMAP ─────────────────────────────────────────────────

    /// VIEW (x1,y1)-(x2,y2) [,fill [,border]] — define graphics viewport.
    /// fill/border negative means "not specified".
    pub fn set_view(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, fill: f64, border: f64) {
        self.view_x1 = x1; self.view_y1 = y1;
        self.view_x2 = x2; self.view_y2 = y2;
        self.view_active = true;
        if fill >= 0.0 {
            // Fill the viewport rectangle
            let (lx, rx) = (x1 as i32, x2 as i32);
            let (ty, by) = (y1 as i32, y2 as i32);
            for py in ty..=by { for px in lx..=rx { self.pset_raw(px, py, fill); } }
        }
        if border >= 0.0 {
            // Draw viewport border (in fb coords, no logical transform)
            let c = border;
            for px in x1 as i32..=x2 as i32 {
                self.pset_raw(px, y1 as i32, c);
                self.pset_raw(px, y2 as i32, c);
            }
            for py in y1 as i32..=y2 as i32 {
                self.pset_raw(x1 as i32, py, c);
                self.pset_raw(x2 as i32, py, c);
            }
        }
    }

    /// WINDOW [SCREEN] (x1,y1)-(x2,y2) — define logical coordinate window mapped
    /// to the viewport. `screen` = the SCREEN keyword was present, meaning
    /// screen-orientation Y (no inversion); plain WINDOW inverts Y (Cartesian).
    pub fn set_window(&mut self, x1: f64, y1: f64, x2: f64, y2: f64, screen: bool) {
        self.win_x1 = x1; self.win_y1 = y1;
        self.win_x2 = x2; self.win_y2 = y2;
        self.win_active = true;
        self.win_screen = screen;
    }

    /// PMAP — map between physical viewport coords and logical window coords.
    /// mode 0: logical X  → viewport physical X
    /// mode 1: logical Y  → viewport physical Y
    /// mode 2: viewport X → logical X  (VIEW-relative physical)
    /// mode 3: viewport Y → logical Y  (VIEW-relative physical)
    pub fn pmap(&self, coord: f64, mode: f64) -> f64 {
        // WINDOW without an explicit VIEW maps onto the whole screen (DOS QB behavior).
        let (vx1, vy1, vx2, vy2) = self.effective_viewport();
        // WINDOW SCREEN maps by magnitude (min → top-left), matching logical_to_fb;
        // plain WINDOW keeps corner order (and inverts Y in the match arms below).
        let (wx1, wy1, wx2, wy2) = if self.win_screen {
            (self.win_x1.min(self.win_x2), self.win_y1.min(self.win_y2),
             self.win_x1.max(self.win_x2), self.win_y1.max(self.win_y2))
        } else {
            (self.win_x1, self.win_y1, self.win_x2, self.win_y2)
        };
        if (vx2 - vx1).abs() < 1e-10 || (vy2 - vy1).abs() < 1e-10 { return coord; }
        if (wx2 - wx1).abs() < 1e-10 || (wy2 - wy1).abs() < 1e-10 { return coord; }
        match mode as i32 {
            // modes 0/1: logical → absolute screen coord. Y inversion matches
            // logical_to_fb (plain WINDOW inverts; WINDOW SCREEN does not), so
            // PMAP round-trips with POINT/LINE coordinates.
            0 => vx1 + (coord - wx1) / (wx2 - wx1) * (vx2 - vx1),
            1 if self.win_screen => vy1 + (coord - wy1) / (wy2 - wy1) * (vy2 - vy1),
            1 => vy1 + (wy2 - coord) / (wy2 - wy1) * (vy2 - vy1),
            // modes 2/3: viewport-relative coord (0 = viewport top/left) → logical
            2 => wx1 + coord / (vx2 - vx1) * (wx2 - wx1),
            3 if self.win_screen => wy1 + coord / (vy2 - vy1) * (wy2 - wy1),
            3 => wy2 - coord / (vy2 - vy1) * (wy2 - wy1),
            _ => coord,
        }
    }

    /// PALETTE USING arr(start) — remap all palette entries from a slice of indices.
    pub fn palette_using(&mut self, arr: &[f64]) {
        if self.screen_mode == 13 {
            // MCGA: up to 256 entries, each an 18-bit DAC color value.
            for (i, &v) in arr.iter().enumerate().take(256) {
                self.palette_rgb[i] = dac18_to_rgb(v as u64);
            }
            return;
        }
        if self.screen_mode == 11 || self.screen_mode == 12 {
            // VGA 16-color modes: each entry is an 18-bit DAC value, not an index.
            for (i, &v) in arr.iter().enumerate().take(16) {
                self.palette_rgb[i] = dac18_to_rgb(v as u64);
            }
            return;
        }
        for (i, &v) in arr.iter().enumerate().take(16) {
            self.palette_rgb[i] = EGA[(v as i32).rem_euclid(16) as usize];
        }
    }

    pub fn circle(&mut self, cx: f64, cy: f64, r: f64, color: f64) {
        // QB moves the "last point referenced" to the circle's center (matters
        // for a following STEP coordinate). Done before any early-return.
        self.gfx_x = cx;
        self.gfx_y = cy;
        let aspect = if self.screen_mode == 7 || self.screen_mode == 1 { 0.8333 } else { 1.0 };
        let (fcx, fcy) = self.logical_to_fb(cx, cy);
        let rx = r as i32;
        let ry = (r * aspect) as i32;
        if rx <= 0 { return; }
        midpoint_ellipse(fcx as i32, fcy as i32, rx, ry.max(1), |x, y| {
            self.pset_raw(x, y, color);
        });
        self.auto_present();
    }

    pub fn paint(&mut self, x: f64, y: f64, fill: f64, border: f64) {
        // Negative fill/border = "use current draw color" (omitted in QB source)
        let m = self.color_mod();
        let fill_idx   = if fill   < 0.0 { self.draw_color }
                         else { (fill   as i64).rem_euclid(m) as u8 };
        let border_idx = if border < 0.0 { fill_idx }
                         else { (border as i64).rem_euclid(m) as u8 };
        let (fx, fy) = self.logical_to_fb(x, y);
        flood_fill(self, fx as i32, fy as i32, fill_idx, border_idx);
        self.auto_present();
    }

    /// QB `PAINT (x,y), CHR$(n)[+...], border` — pattern tiling flood fill.
    /// Each byte in `pattern` defines one row: bit 7 = leftmost pixel; the
    /// pattern tiles horizontally every 8 columns and vertically every
    /// `pattern.len()` rows. Pixels where the bit is 1 receive the current
    /// foreground/draw color; pixels where the bit is 0 are left unchanged.
    pub fn paint_pattern(&mut self, x: f64, y: f64, pattern: &str, border: f64) {
        if pattern.is_empty() { return; }
        // QB strings are Latin-1: each char's code-point IS the raw byte value.
        // Using .as_bytes() would give UTF-8 (two bytes for chars > U+007F), so
        // we do an explicit Latin-1 extraction here.
        let bytes: Vec<u8> = pattern.chars().map(|c| c as u8).collect();
        let m = self.color_mod();
        let fg = self.draw_color;
        let border_idx = if border < 0.0 { fg }
                         else { (border as i64).rem_euclid(m) as u8 };
        let (fx, fy) = self.logical_to_fb(x, y);
        flood_fill_pattern(self, fx as i32, fy as i32, &bytes, fg, border_idx);
        self.auto_present();
    }

    /// Non-blocking key poll — flushes fb (which harvests keys into key_queue), then pops one.
    /// Returns "" if no key is ready.
    pub fn inkey(&mut self) -> String {
        // Poll keys cheaply on every call, but blit the framebuffer at most once
        // per frame interval. A tight per-pixel INKEY$ loop (e.g. mandel's
        // `IF INKEY$ <> "" THEN END`, ~73k calls) would otherwise rebuild the
        // 960×600 frame every call; throttling keeps progressive rendering
        // visible (~60fps) while the rest are cheap event polls. Both branches
        // harvest keypresses into key_queue, so no key is lost.
        let now = std::time::Instant::now();
        if now.duration_since(self.last_present)
              >= std::time::Duration::from_millis(self.frame_interval_ms) {
            self.present();
            self.last_present = now;
        } else {
            self.pump_events();
            // Yield 1 ms so INKEY$-based busy-wait loops (e.g. gorilla's
            // Rest() SUB) don't spin the CPU at 100% and so their timer
            // checks are accurate to ~1 ms rather than sub-microsecond.
            std::thread::sleep(std::time::Duration::from_millis(1));
        }
        match self.key_queue.pop_front() {
            Some(k) if k == "\u{0}" => "".to_string(), // DRAIN sentinel → "" (stops drain loops)
            Some(k) => k,
            None    => "".to_string(),
        }
    }

    pub fn spc(&self, n: f64) -> String { qb_space(n) }

    /// INPUT$(n) — read exactly n characters from the keyboard (blocking).
    /// In text mode uses crossterm; in graphics mode polls the key queue.
    pub fn input_str(&mut self, n: f64) -> String {
        let count = (n as usize).max(1);
        let mut result = String::new();
        while result.len() < count {
            // Headless: satisfy the read from the scripted queue; if it can't be
            // satisfied, the program is blocked on input that won't come — finish.
            if self.headless_cfg.is_some() {
                match self.key_queue.pop_front() {
                    Some(k) => { result.push_str(&k); continue; }
                    None    => self.headless_finish(),
                }
            }
            // Block until we get a character
            let ch = loop {
                let k = self.inkey();
                if !k.is_empty() { break k; }
                std::thread::sleep(std::time::Duration::from_millis(10));
            };
            result.push_str(&ch);
        }
        result
    }

    // ── Sound — stubs (M4 will wire to rodio) ────────────────────────────────

    /// PLAY "MML string" — QB Music Macro Language.
    /// MML state (octave, tempo, length) persists across calls.
    /// MB prefix plays in background; MF (default) blocks until done.
    pub fn play(&mut self, mml: &str) {
        let events = sound::parse_mml(mml, &mut self.mml_state);
        if self.mml_state.background {
            // Don't stack a new play on top of one already running.
            if self.bg_playing.load(std::sync::atomic::Ordering::Relaxed) {
                return;
            }
            self.bg_playing.store(true, std::sync::atomic::Ordering::Relaxed);
            let flag = self.bg_playing.clone();
            sound::play_events_background_flagged(events, flag);
        } else {
            sound::play_events_blocking(&events);
        }
    }

    /// PLAY(n) function — returns number of notes remaining in the background music queue.
    /// Returns 10 while a background PLAY thread is active (≥5 → throttle), 0 when done.
    pub fn play_count(&self) -> f64 {
        if self.bg_playing.load(std::sync::atomic::Ordering::Relaxed) { 10.0 } else { 0.0 }
    }

    /// BEEP — short 800 Hz tone (~220 ms).
    pub fn beep(&mut self) {
        sound::play_beep();
    }

    /// POKE addr, val — store a byte in the simulated memory map.
    /// QB's POKE stores an unsigned byte (0–255) at the given address.
    pub fn qb_poke(&mut self, addr: f64, val: f64) {
        let a = (addr as i64) as u32;
        let v = ((val as i64) & 0xFF) as u8;
        self.poke_mem.insert(a, v);
    }

    /// PEEK(addr) — read a byte previously written by POKE (returns 0 if never written).
    pub fn qb_peek(&mut self, addr: f64) -> f64 {
        let a = (addr as i64) as u32;
        self.poke_mem.get(&a).copied().unwrap_or(0) as f64
    }

    /// OUT port, val — write a byte to a hardware I/O port.
    ///
    /// Intercepts the VGA DAC ports used to set palette entries directly:
    ///   0x3C8 — DAC write-address register: sets the palette index to start writing.
    ///   0x3C9 — DAC data register: R, G, B written in sequence (each 0–63);
    ///           the index auto-advances to the next entry after each blue byte.
    ///   0x3C7 — DAC read-address register: sets the palette index to start reading.
    ///   0x3C6 — DAC pixel mask (normally 0xFF); ignored.
    /// All other ports are silently ignored.
    pub fn qb_out(&mut self, port: f64, val: f64) {
        let v = (val as i64) as u8;
        match port as u16 {
            0x3C8 => {
                self.dac_write_idx = v as usize & 0xFF;
                self.dac_channel   = 0;
            }
            0x3C9 => {
                let dac = v & 63; // DAC values are 6-bit (0–63)
                match self.dac_channel {
                    0 => { self.dac_pending_r = dac; self.dac_channel = 1; }
                    1 => { self.dac_pending_g = dac; self.dac_channel = 2; }
                    _ => {
                        let r = dac6_to_8(self.dac_pending_r);
                        let g = dac6_to_8(self.dac_pending_g);
                        let b = dac6_to_8(dac);
                        self.palette_rgb[self.dac_write_idx] = (r, g, b);
                        self.dac_write_idx = (self.dac_write_idx + 1) & 0xFF;
                        self.dac_channel = 0;
                    }
                }
            }
            0x3C7 => {
                self.dac_read_idx = v as usize & 0xFF;
                self.dac_read_ch  = 0;
            }
            _ => {} // all other ports silently ignored
        }
    }

    /// INP(port) — read a byte from a hardware I/O port.
    ///
    /// Reads back VGA DAC palette entries via port 0x3C9 (R, G, B in sequence,
    /// 6-bit per channel, same as set by OUT 0x3C9).  Port 0x3C7 sets the
    /// read address (same as DAC read-address register).  All other ports return 0.
    pub fn qb_in(&mut self, port: f64) -> f64 {
        match port as u16 {
            0x3C9 => {
                let (r, g, b) = self.palette_rgb[self.dac_read_idx];
                let ch = self.dac_read_ch;
                self.dac_read_ch += 1;
                if self.dac_read_ch == 3 {
                    self.dac_read_ch = 0;
                    self.dac_read_idx = (self.dac_read_idx + 1) & 0xFF;
                }
                // Scale 8-bit back to 6-bit DAC (reverse of dac6_to_8)
                (match ch { 0 => r >> 2, 1 => g >> 2, _ => b >> 2 }) as f64
            }
            _ => 0.0
        }
    }

    /// SOUND freq, duration — freq in Hz, duration in PC timer ticks (18.2/sec).
    pub fn sound(&mut self, freq: f64, dur: f64) {
        sound::play_sound(freq, dur);
    }
    /// DRAW "turtle-graphics-string" — stub until full implementation
    /// DRAW statement — turtle-graphics mini-language interpreter.
    /// Supports: U D L R E F G H (directional), M x,y (move/draw),
    ///           B (blind/no-draw prefix), N (no-advance prefix),
    ///           C n (color), S n (scale).
    pub fn draw(&mut self, cmd: &str) {
        let chars: Vec<char> = cmd.chars().collect();
        let len = chars.len();
        let mut i = 0;

        // Parse an integer that may start with optional '+' or '-'.
        // Returns (value, bool_had_sign, new_i).
        fn parse_int_sign(chars: &[char], mut i: usize) -> (i32, bool, usize) {
            let neg = i < chars.len() && chars[i] == '-';
            let had_sign = neg || (i < chars.len() && chars[i] == '+');
            if had_sign { i += 1; }
            let mut v: i32 = 0;
            while i < chars.len() && chars[i].is_ascii_digit() {
                v = v * 10 + (chars[i] as i32 - '0' as i32);
                i += 1;
            }
            (if neg { -v } else { v }, had_sign, i)
        }

        while i < len {
            while i < len && chars[i] == ' ' { i += 1; }
            if i >= len { break; }

            let ch = chars[i].to_ascii_uppercase();
            i += 1;

            // Handle S (scale) and C (color) commands
            if ch == 'S' {
                let (v, _, ni) = parse_int_sign(&chars, i);
                i = ni;
                self.draw_scale = if v > 0 { v as f64 } else { 4.0 };
                continue;
            }
            if ch == 'C' {
                let (v, _, ni) = parse_int_sign(&chars, i);
                i = ni;
                self.draw_color = (v as i64).rem_euclid(self.color_mod()) as u8;
                continue;
            }

            // All other commands may have B and/or N modifier prefix.
            let mut blind  = false;
            let mut no_adv = false;
            let mut opcode = ch;

            // Consume modifier chain: B and/or N before the actual direction letter
            loop {
                match opcode {
                    'B' => { blind  = true; }
                    'N' => { no_adv = true; }
                    _   => break,
                }
                while i < len && chars[i] == ' ' { i += 1; }
                if i >= len { break; }
                opcode = chars[i].to_ascii_uppercase();
                i += 1;
            }

            let ppu = self.draw_scale / 4.0; // pixels per unit
            let cx0 = self.gfx_x;
            let cy0 = self.gfx_y;
            let color = self.draw_color as f64;

            if opcode == 'M' {
                // M x,y — QB rule: a leading sign on the *X* coordinate makes the
                // WHOLE move relative ("if x is preceded by + or -, x and y are
                // added to the current position"); no sign means an absolute
                // move. The Y sign only sets its own direction, it does NOT
                // independently switch the mode. (Donkey draws shapes with moves
                // like `M-1,1` — relative-x, bare-y — which must be fully
                // relative, else the outline shatters and PAINT floods.)
                let (vx, rel_x, ni) = parse_int_sign(&chars, i);
                i = ni;
                while i < len && (chars[i] == ',' || chars[i] == ' ') { i += 1; }
                let (vy, _rel_y, ni) = parse_int_sign(&chars, i);
                i = ni;
                let tx = if rel_x { cx0 + vx as f64 * ppu } else { vx as f64 };
                let ty = if rel_x { cy0 + vy as f64 * ppu } else { vy as f64 };
                if !blind { self.line(cx0, cy0, tx, ty, color); }
                // `self.line()` advances the cursor to the endpoint, so the `N`
                // (no-advance) modifier must RESTORE the original position —
                // not merely skip a second advance. (Car sprites use `ND2`
                // spurs; without this the cursor drifts and the outline gaps.)
                if no_adv { self.gfx_x = cx0; self.gfx_y = cy0; }
                else      { self.gfx_x = tx;  self.gfx_y = ty;  }
            } else if "UDLREFGH".contains(opcode) {
                // Directional move: optional count follows
                let (count, _, ni) = if i < len && (chars[i].is_ascii_digit() || chars[i] == '+' || chars[i] == '-') {
                    parse_int_sign(&chars, i)
                } else {
                    (1, false, i)
                };
                i = ni;
                let dist = count.max(0) as f64 * ppu;
                let (dx, dy): (f64, f64) = match opcode {
                    'U' => (0.0, -dist),
                    'D' => (0.0,  dist),
                    'L' => (-dist, 0.0),
                    'R' => ( dist, 0.0),
                    'E' => ( dist, -dist),
                    'F' => ( dist,  dist),
                    'G' => (-dist,  dist),
                    'H' => (-dist, -dist),
                    _   => (0.0, 0.0),
                };
                let tx = cx0 + dx;
                let ty = cy0 + dy;
                if !blind { self.line(cx0, cy0, tx, ty, color); }
                // See M-branch note: `self.line()` advances the cursor, so `N`
                // must restore it to the start, not just skip a re-advance.
                if no_adv { self.gfx_x = cx0; self.gfx_y = cy0; }
                else      { self.gfx_x = tx;  self.gfx_y = ty;  }
            }
            // Unknown opcodes are silently skipped
        }
    }

    // ── Sleep ─────────────────────────────────────────────────────────────────

    /// Pause for `secs` seconds. Flushes graphics first so the current frame
    /// is visible during the sleep.
    pub fn sleep(&mut self, secs: f64) {
        self.present();
        std::thread::sleep(std::time::Duration::from_secs_f64((secs * self.slowmo).max(0.0)));
    }

    /// Called by emitted END / STOP — waits for keypress then exits.
    /// Uses the same logic as Drop so the window stays readable.
    pub fn quit(&mut self) -> ! {
        // Headless: dump/checksum and exit immediately (no wait-for-key).
        if self.headless_cfg.is_some() { self.headless_finish(); }
        // Drop will run after process::exit is NOT called here — we do the
        // wait ourselves so we can call process::exit cleanly afterward.
        self.wait_for_key();
        std::process::exit(0);
    }

    /// Shared "hold window open until keypress" logic used by quit() and Drop.
    /// Only blocks when an explicit SCREEN call was made — text-only programs
    /// (hello-world, integration tests) exit immediately without hanging.
    fn wait_for_key(&mut self) {
        if self.window.is_none() || !self.had_screen_call {
            return;
        }
        // Print hint at bottom of screen.
        let max_rows = (self.height / self.char_h) as usize;
        self.cursor_row = max_rows;
        self.cursor_col = 1;
        let saved_fg = self.fg_color;
        self.fg_color = 8; // dark grey
        self.print_gfx("[Press any key to exit]", false);
        self.fg_color = saved_fg;

        'wait: loop {
            let fw = self.width  as usize;
            let fh = self.height as usize;
            let win_w = self.win_w;
            let win_h = self.win_h;
            let palette = self.palette_rgb;
            // Persistent buffer — see `present_buf` doc; a local Vec here would be
            // freed each iteration, leaving minifb a dangling pointer to redraw
            // from during the 16ms sleep (use-after-free segfault).
            if self.present_buf.len() != win_w * win_h {
                self.present_buf.resize(win_w * win_h, 0);
            }
            for oy in 0..win_h {
                let fy = (oy * fh) / win_h;
                let row_base = fy * fw;
                let out_base = oy * win_w;
                for ox in 0..win_w {
                    let fx = (ox * fw) / win_w;
                    let (r, g, b) = palette[self.fb[row_base + fx] as usize];
                    self.present_buf[out_base + ox] = ((r as u32) << 16) | ((g as u32) << 8) | (b as u32);
                }
            }
            let new_keys: Vec<Key> = {
                let out = &self.present_buf;
                let win = match self.window.as_mut() { Some(w) => w, None => break 'wait };
                let _ = win.update_with_buffer(out, win_w, win_h);
                if !win.is_open() { break 'wait; }
                win.get_keys_pressed(KeyRepeat::No)
            };
            if !new_keys.is_empty() { break 'wait; }
            std::thread::sleep(std::time::Duration::from_millis(16));
        }
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        // Headless: a program that falls off the end of main() (no END → quit())
        // still gets its framebuffer dump/checksum. headless_finish() exits, so
        // wait_for_key() below is reached only in the windowed case.
        if self.headless_cfg.is_some() { self.headless_finish(); }
        // Programs that fall off the end of main() (no END statement) still
        // get the "press any key" pause via wait_for_key().
        self.wait_for_key();
        self.window = None;
    }
}

// ── PRINT USING formatter ─────────────────────────────────────────────────────

/// A mixed numeric/string value for PRINT USING.
pub enum QbVal<'a> {
    Num(f64),
    Str(&'a str),
}
impl<'a> From<f64> for QbVal<'a> {
    fn from(v: f64) -> Self { QbVal::Num(v) }
}
impl<'a> From<&'a str> for QbVal<'a> {
    fn from(v: &'a str) -> Self { QbVal::Str(v) }
}
impl<'a> From<&'a String> for QbVal<'a> {
    fn from(v: &'a String) -> Self { QbVal::Str(v.as_str()) }
}

/// Format a value in QBasic `^^^^` exponential (scientific) notation, returning
/// the *core* string `[-]d.ddddE±dd` (the caller right-justifies it in the
/// field). No padding and no space for a positive sign are added here.
///
/// `int_digits` = number of `#` before the decimal point in the format,
/// `frac_digits` = number after it, `exp_digits` = number of exponent digits
/// (4 carets → `E±dd` → 2 exponent digits; 5 → 3, etc.).
///
/// Model (matches Microsoft QBasic): the mantissa is normalized to a single
/// significant integer digit (`d.dddd`) when `int_digits >= 1`, so the extra
/// integer `#` positions become field-width padding (which is where a positive
/// number's leading space or a negative number's `-` lands). When
/// `int_digits == 0` the mantissa is the `.dddd` form (e.g. `.8889E+06`).
fn fmt_exponential(
    v: f64,
    int_digits: usize,
    frac_digits: usize,
    exp_digits: usize,
    force_plus: bool,
) -> String {
    let neg = v < 0.0;
    let av = v.abs();
    let mant_int = if int_digits >= 1 { 1 } else { 0 }; // integer mantissa digits
    let total = mant_int + frac_digits;                 // significant digits shown

    let (digits, exp) = if av == 0.0 || !av.is_finite() {
        ("0".repeat(total.max(1)), 0i32)
    } else {
        let e10 = av.log10().floor() as i32;
        // Decimal exponent so the mantissa has `mant_int` integer digits
        // (or sits in [0.1, 1) when mant_int == 0).
        let big_e = if mant_int >= 1 { e10 } else { e10 + 1 };
        let mantissa = av / 10f64.powi(big_e);
        let mut rounded = (mantissa * 10f64.powi(frac_digits as i32)).round() as i128;
        let mut exp = big_e;
        let mut s = rounded.to_string();
        // Rounding can push e.g. 9.999 → 10.00, adding an integer digit.
        if s.len() > total {
            rounded /= 10;
            exp += 1;
            s = rounded.to_string();
        }
        // Left-pad with zeros for small mantissas.
        while s.len() < total { s.insert(0, '0'); }
        (s, exp)
    };

    // Split the significant digits into integer / fractional parts.
    let (ip, fp) = digits.split_at(mant_int.min(digits.len()));
    let mantissa_str = if mant_int == 0 {
        format!(".{}", fp)
    } else if frac_digits > 0 {
        format!("{}.{}", ip, fp)
    } else {
        ip.to_string()
    };

    let sign = if neg { "-" } else if force_plus { "+" } else { "" };
    let exp_sign = if exp < 0 { '-' } else { '+' };
    let exp_str = format!("E{}{:0width$}", exp_sign, exp.unsigned_abs(), width = exp_digits.max(2));

    format!("{}{}{}", sign, mantissa_str, exp_str)
}

/// Replace the leading run of spaces in `s` with `*` (for `**` PRINT USING prefix).
fn replace_leading_stars(s: &str) -> String {
    let first_non_space = s.find(|c: char| c != ' ').unwrap_or(s.len());
    format!("{}{}", "*".repeat(first_non_space), &s[first_non_space..])
}

/// PRINT USING with mixed numeric and string values.
pub fn qb_print_using(fmt: &str, values: &[QbVal]) -> String {
    let mut result = String::new();
    let chars: Vec<char> = fmt.chars().collect();
    let mut i = 0;
    let mut val_idx = 0;

    while i < chars.len() {
        // ── Literal escape: `_X` prints X verbatim (so `_#` prints a literal '#') ──
        if chars[i] == '_' && i + 1 < chars.len() {
            result.push(chars[i + 1]);
            i += 2;
            continue;
        }
        // ── String field: \...\ (width = spaces + 2), ! (1 char), & (any length) ──
        if chars[i] == '\\' {
            // Count characters until closing backslash
            let mut j = i + 1;
            while j < chars.len() && chars[j] != '\\' { j += 1; }
            let field_len = (j - i + 1).max(2); // minimum 2 (just \\)
            i = j + 1; // skip past closing backslash
            let s = match values.get(val_idx) {
                Some(QbVal::Str(s)) => *s,
                Some(QbVal::Num(n)) => { let tmp = format!("{}", n); result.push_str(&tmp); val_idx += 1; continue; }
                None => "",
            };
            val_idx += 1;
            // Pad or truncate to field_len
            if s.len() >= field_len {
                result.push_str(&s[..field_len]);
            } else {
                result.push_str(s);
                result.push_str(&" ".repeat(field_len - s.len()));
            }
            continue;
        }
        if chars[i] == '!' {
            i += 1;
            let s = match values.get(val_idx) {
                Some(QbVal::Str(s)) => *s,
                _ => "",
            };
            val_idx += 1;
            result.push(s.chars().next().unwrap_or(' '));
            continue;
        }
        if chars[i] == '&' {
            i += 1;
            let s = match values.get(val_idx) {
                Some(QbVal::Str(s)) => *s,
                _ => "",
            };
            val_idx += 1;
            result.push_str(s);
            continue;
        }

        // ── Numeric field: **/$$ prefixes, #, +, -, ##.##, .## etc. ─────────────
        let is_num_start = chars[i] == '#'
            || chars[i] == '+'
            || (chars[i] == '-' && i + 1 < chars.len() && chars[i+1] == '#')
            || (chars[i] == '.' && i + 1 < chars.len() && chars[i+1] == '#')
            || (chars[i] == '*' && i + 1 < chars.len() && chars[i+1] == '*')
            || (chars[i] == '$' && i + 1 < chars.len() && chars[i+1] == '$');
        if is_num_start {
            // ── Prefix tokens: ** (asterisk fill) and $$ / **$ (floating dollar)
            let has_star_fill = chars[i] == '*';
            if has_star_fill { i += 2; }  // consume **
            let (has_floating_dollar, dollar_prefix) =
                if has_star_fill && i < chars.len() && chars[i] == '$' {
                    i += 1; (true, 1usize)   // **$: one $ char, contributes 1 to width
                } else if !has_star_fill && i < chars.len() && chars[i] == '$' {
                    i += 2; (true, 2usize)   // $$: two $ chars, contribute 2 to width
                } else {
                    (false, 0usize)
                };
            let star_count: usize = if has_star_fill { 2 } else { 0 };

            let has_leading_sign = i < chars.len() && (chars[i] == '+' || chars[i] == '-');
            let force_plus = has_leading_sign && chars[i] == '+';
            if has_leading_sign { i += 1; }
            let mut int_digits = 0usize;
            let mut has_comma = false;
            while i < chars.len() && (chars[i] == '#' || chars[i] == ',') {
                if chars[i] == '#' { int_digits += 1; }
                else { has_comma = true; }
                i += 1;
            }
            let mut frac_digits = 0usize;
            if i < chars.len() && chars[i] == '.' {
                i += 1;
                while i < chars.len() && (chars[i] == '#' || chars[i] == '0') {
                    frac_digits += 1;
                    i += 1;
                }
            }
            // Exponential format: `^^^^` (4+ carets). Fewer than 4 carets are
            // not an exponent specifier — leave them to be printed literally.
            let mut caret_count = 0usize;
            {
                let mut k = i;
                while k < chars.len() && chars[k] == '^' { caret_count += 1; k += 1; }
            }
            let exponential = caret_count >= 4;
            if exponential { i += caret_count; }

            let has_trailing_sign = i < chars.len() && chars[i] == '-';
            if has_trailing_sign { i += 1; }

            let v = match values.get(val_idx) {
                Some(QbVal::Num(n)) => *n,
                Some(QbVal::Str(s)) => s.parse::<f64>().unwrap_or(0.0),
                None => 0.0,
            };
            val_idx += 1;

            // ── Exponential (scientific) branch ─────────────────────────────────
            if exponential {
                let exp_digits = caret_count - 2; // 4 carets → E±dd (2 exp digits)
                let core = fmt_exponential(v, int_digits, frac_digits, exp_digits, force_plus);
                // Field width: integer #s + (dot + frac) + (E± + exp digits).
                let field_w = int_digits
                    + if frac_digits > 0 { 1 + frac_digits } else { 0 }
                    + (2 + exp_digits.max(2));
                if core.len() < field_w {
                    result.push_str(&" ".repeat(field_w - core.len()));
                }
                result.push_str(&core);
                continue;
            }

            let total_int_width = int_digits.max(1);
            let formatted = if frac_digits > 0 {
                format!("{:.prec$}", v.abs(), prec = frac_digits)
            } else {
                format!("{}", v.abs() as i64)
            };
            let (int_part, frac_part) = if let Some(pos) = formatted.find('.') {
                (&formatted[..pos], &formatted[pos..])
            } else {
                (formatted.as_str(), "")
            };

            let int_str = if has_comma {
                let digits: Vec<char> = int_part.chars().collect();
                let mut s = String::new();
                let n = digits.len();
                for (k, &d) in digits.iter().enumerate() {
                    if k > 0 && (n - k) % 3 == 0 { s.push(','); }
                    s.push(d);
                }
                s
            } else {
                int_part.to_string()
            };

            // ── Overflow: value too wide for digit capacity → QB prepends `%`.
            //    Star positions (**) extend digit capacity; dollar prefix does not.
            if int_part.len() > star_count + total_int_width {
                let sign_ov = if v < 0.0 { "-" } else if force_plus { "+" } else { "" };
                result.push('%');
                result.push_str(&format!("{}{}{}", sign_ov, int_str, frac_part));
                if has_trailing_sign {
                    if v < 0.0 { result.push('-'); } else { result.push(' '); }
                }
                continue;
            }

            let frac_w = if frac_digits > 0 { frac_digits + 1 } else { 0 };
            // Total output width includes all prefix contributions.
            let total_w = star_count + dollar_prefix + total_int_width + frac_w;

            if has_floating_dollar {
                // $ floats immediately left of the first digit.
                // Compute padding: total_w - len(num_str) - 1 (for $) [- 1 more for sign if negative]
                let num_str_body = format!("{}{}", int_str, frac_part);
                let out = if v < 0.0 {
                    let pad = total_w.saturating_sub(num_str_body.len() + 2);
                    format!("{}-${}", " ".repeat(pad), num_str_body)
                } else {
                    let pad = total_w.saturating_sub(num_str_body.len() + 1);
                    format!("{}${}", " ".repeat(pad), num_str_body)
                };
                result.push_str(&if has_star_fill { replace_leading_stars(&out) } else { out });
            } else if has_star_fill {
                // Leading spaces replaced with '*'. Star slots extend the field.
                let sign = if v < 0.0 { "-" } else if force_plus { "+" } else { "" };
                let signed = format!("{}{}{}", sign, int_str, frac_part);
                let padded = if signed.len() < total_w {
                    format!("{:>width$}", signed, width = total_w)
                } else {
                    signed
                };
                result.push_str(&replace_leading_stars(&padded));
            } else if has_leading_sign || has_trailing_sign {
                // Explicit-sign formats reserve one extra column for the sign,
                // which is included in the (unchanged) original layout.
                let sign = if v < 0.0 { "-" } else if force_plus { "+" } else { " " };
                let num_str = format!("{}{}{}", sign, int_str, frac_part);
                let target_len = total_int_width + frac_w + 1;
                if num_str.len() < target_len {
                    result.push_str(&" ".repeat(target_len - num_str.len()));
                }
                result.push_str(&num_str);
                if has_trailing_sign {
                    if v < 0.0 { result.push('-'); } else { result.push(' '); }
                }
            } else {
                // Common case (no explicit sign): the field width is exactly the
                // literal width of the format spec (`#`s plus `.frac`). A plain
                // positive number is right-justified — its leading blanks come
                // from unused `#` columns, not an extra sign slot — and a
                // negative `-` borrows one of those columns.
                let field_w = total_int_width + frac_w;
                let lead = if v < 0.0 { "-" } else { "" };
                let signed = format!("{}{}{}", lead, int_str, frac_part);
                if signed.len() < field_w {
                    result.push_str(&" ".repeat(field_w - signed.len()));
                }
                result.push_str(&signed);
            }
        } else {
            result.push(chars[i]);
            i += 1;
        }
    }
    result
}

// ── minifb key → QB INKEY$ string ────────────────────────────────────────────

/// Map a minifb Key to a QB INKEY$ string (unshifted, lowercase letters).
fn minifb_key_to_qb(key: Key) -> String {
    use Key::*;
    match key {
        A=>"a", B=>"b", C=>"c", D=>"d", E=>"e", F=>"f", G=>"g",
        H=>"h", I=>"i", J=>"j", K=>"k", L=>"l", M=>"m", N=>"n",
        O=>"o", P=>"p", Q=>"q", R=>"r", S=>"s", T=>"t", U=>"u",
        V=>"v", W=>"w", X=>"x", Y=>"y", Z=>"z",
        Key0=>"0", Key1=>"1", Key2=>"2", Key3=>"3", Key4=>"4",
        Key5=>"5", Key6=>"6", Key7=>"7", Key8=>"8", Key9=>"9",
        NumPad0=>"0", NumPad1=>"1", NumPad2=>"2", NumPad3=>"3", NumPad4=>"4",
        NumPad5=>"5", NumPad6=>"6", NumPad7=>"7", NumPad8=>"8", NumPad9=>"9",
        Space=>" ", Period=>".", Minus=>"-", Equal=>"=",
        LeftBracket=>"[", RightBracket=>"]", Backslash=>"\\",
        Semicolon=>";", Apostrophe=>"'", Comma=>",", Slash=>"/",
        Enter=>"\r", Backspace=>"\x08", Escape=>"\x1b",
        Up=>"\x00H", Down=>"\x00P", Left=>"\x00K", Right=>"\x00M",
        F1=>"\x00;", F2=>"\x00<", F3=>"\x00=", F4=>"\x00>",
        F5=>"\x00?", F6=>"\x00@", F7=>"\x00A", F8=>"\x00B",
        F9=>"\x00C", F10=>"\x00D",
        _=>"",
    }.to_string()
}

/// Map a minifb Key + shift state to a printable char for INPUT echo.
fn window_key_to_char(key: Key, shift: bool) -> Option<char> {
    use Key::*;
    Some(match key {
        A => if shift { 'A' } else { 'a' },
        B => if shift { 'B' } else { 'b' },
        C => if shift { 'C' } else { 'c' },
        D => if shift { 'D' } else { 'd' },
        E => if shift { 'E' } else { 'e' },
        F => if shift { 'F' } else { 'f' },
        G => if shift { 'G' } else { 'g' },
        H => if shift { 'H' } else { 'h' },
        I => if shift { 'I' } else { 'i' },
        J => if shift { 'J' } else { 'j' },
        K => if shift { 'K' } else { 'k' },
        L => if shift { 'L' } else { 'l' },
        M => if shift { 'M' } else { 'm' },
        N => if shift { 'N' } else { 'n' },
        O => if shift { 'O' } else { 'o' },
        P => if shift { 'P' } else { 'p' },
        Q => if shift { 'Q' } else { 'q' },
        R => if shift { 'R' } else { 'r' },
        S => if shift { 'S' } else { 's' },
        T => if shift { 'T' } else { 't' },
        U => if shift { 'U' } else { 'u' },
        V => if shift { 'V' } else { 'v' },
        W => if shift { 'W' } else { 'w' },
        X => if shift { 'X' } else { 'x' },
        Y => if shift { 'Y' } else { 'y' },
        Z => if shift { 'Z' } else { 'z' },
        Key0 => if shift { ')' } else { '0' },
        Key1 => if shift { '!' } else { '1' },
        Key2 => if shift { '@' } else { '2' },
        Key3 => if shift { '#' } else { '3' },
        Key4 => if shift { '$' } else { '4' },
        Key5 => if shift { '%' } else { '5' },
        Key6 => if shift { '^' } else { '6' },
        Key7 => if shift { '&' } else { '7' },
        Key8 => if shift { '*' } else { '8' },
        Key9 => if shift { '(' } else { '9' },
        Space       => ' ',
        Period      => if shift { '>' } else { '.' },
        Comma       => if shift { '<' } else { ',' },
        Minus       => if shift { '_' } else { '-' },
        Equal       => if shift { '+' } else { '=' },
        Slash       => if shift { '?' } else { '/' },
        Backslash   => if shift { '|' } else { '\\' },
        Semicolon   => if shift { ':' } else { ';' },
        Apostrophe  => if shift { '"' } else { '\'' },
        LeftBracket => if shift { '{' } else { '[' },
        RightBracket=> if shift { '}' } else { ']' },
        _ => return None,
    })
}

// ── Bresenham line ────────────────────────────────────────────────────────────

fn bresenham(x0: i32, y0: i32, x1: i32, y1: i32, mut plot: impl FnMut(i32, i32)) {
    let (mut x, mut y) = (x0, y0);
    let dx = (x1 - x0).abs();
    let dy = -(y1 - y0).abs();
    let sx: i32 = if x0 < x1 { 1 } else { -1 };
    let sy: i32 = if y0 < y1 { 1 } else { -1 };
    let mut err = dx + dy;
    loop {
        plot(x, y);
        if x == x1 && y == y1 { break; }
        let e2 = 2 * err;
        if e2 >= dy { err += dy; x += sx; }
        if e2 <= dx { err += dx; y += sy; }
    }
}

// ── Midpoint ellipse ──────────────────────────────────────────────────────────

fn midpoint_ellipse(cx: i32, cy: i32, rx: i32, ry: i32, mut plot: impl FnMut(i32, i32)) {
    let (rx2, ry2) = ((rx*rx) as i64, (ry*ry) as i64);
    let (mut x, mut y) = (0i64, ry as i64);
    let mut d1 = ry2 - rx2 * ry as i64 + rx2 / 4;
    let (mut dx, mut dy) = (2 * ry2 * x, 2 * rx2 * y);

    while dx < dy {
        let (px, py) = (x as i32, y as i32);
        plot(cx + px, cy + py); plot(cx - px, cy + py);
        plot(cx + px, cy - py); plot(cx - px, cy - py);
        if d1 < 0 { x += 1; dx += 2*ry2; d1 += dx + ry2; }
        else       { x += 1; y -= 1; dx += 2*ry2; dy -= 2*rx2; d1 += dx - dy + ry2; }
    }

    let mut d2 = ry2 * (x*x + x) as i64 + rx2 * ((y-1)*(y-1)) as i64 - rx2*ry2;
    while y >= 0 {
        let (px, py) = (x as i32, y as i32);
        plot(cx + px, cy + py); plot(cx - px, cy + py);
        plot(cx + px, cy - py); plot(cx - px, cy - py);
        if d2 > 0 { y -= 1; dy -= 2*rx2; d2 += rx2 - dy; }
        else       { y -= 1; x += 1; dx += 2*ry2; dy -= 2*rx2; d2 += dx - dy + rx2; }
    }
}

// ── Flood fill ────────────────────────────────────────────────────────────────

fn flood_fill(rt: &mut Runtime, sx: i32, sy: i32, fill: u8, border: u8) {
    if sx < 0 || sy < 0 || sx as u32 >= rt.width || sy as u32 >= rt.height { return; }
    let start_color = rt.fb[(sy as u32 * rt.width + sx as u32) as usize];
    if start_color == fill || start_color == border { return; }

    // Mark start pixel immediately so it's not pushed again
    rt.fb[(sy as u32 * rt.width + sx as u32) as usize] = fill;
    let mut stack = vec![(sx, sy)];
    let mut iters = 0usize;

    let try_push = |stack: &mut Vec<(i32,i32)>, fb: &mut Vec<u8>, w: u32, h: u32,
                    x: i32, y: i32| {
        if x < 0 || y < 0 || x as u32 >= w || y as u32 >= h { return; }
        let idx = (y as u32 * w + x as u32) as usize;
        let c = fb[idx];
        if c != border && c != fill {
            fb[idx] = fill; // mark before pushing — prevents duplicates on the stack
            stack.push((x, y));
        }
    };

    while let Some((x, y)) = stack.pop() {
        try_push(&mut stack, &mut rt.fb, rt.width, rt.height, x+1, y);
        try_push(&mut stack, &mut rt.fb, rt.width, rt.height, x-1, y);
        try_push(&mut stack, &mut rt.fb, rt.width, rt.height, x, y+1);
        try_push(&mut stack, &mut rt.fb, rt.width, rt.height, x, y-1);
        iters += 1;
        if iters % 2000 == 0 { rt.tick(); } // keep window alive during big fills
    }
}

fn flood_fill_pattern(rt: &mut Runtime, sx: i32, sy: i32, pattern: &[u8], fg: u8, border: u8) {
    if sx < 0 || sy < 0 || sx as u32 >= rt.width || sy as u32 >= rt.height { return; }
    let start_color = rt.fb[(sy as u32 * rt.width + sx as u32) as usize];
    if start_color == border { return; }

    let plen = pattern.len() as i32;
    let w = rt.width;
    let h = rt.height;

    let mut stack = vec![(sx, sy)];
    let mut visited: HashSet<(i32, i32)> = HashSet::new();
    visited.insert((sx, sy));
    let mut iters = 0usize;

    while let Some((x, y)) = stack.pop() {
        // Paint this pixel based on pattern bit
        let row_byte = pattern[y.rem_euclid(plen) as usize];
        let bit_pos  = 7u8.saturating_sub(x.rem_euclid(8) as u8);
        if (row_byte >> bit_pos) & 1 == 1 {
            rt.fb[(y as u32 * w + x as u32) as usize] = fg;
        }
        // Spread to neighbors that still hold start_color
        for (nx, ny) in [(x+1,y),(x-1,y),(x,y+1),(x,y-1)] {
            if nx < 0 || ny < 0 || nx as u32 >= w || ny as u32 >= h { continue; }
            if visited.contains(&(nx, ny)) { continue; }
            let nc = rt.fb[(ny as u32 * w + nx as u32) as usize];
            if nc != border && nc == start_color {
                visited.insert((nx, ny));
                stack.push((nx, ny));
            }
        }
        iters += 1;
        if iters % 2000 == 0 { rt.tick(); }
    }
}

// ── Boolean helpers ───────────────────────────────────────────────────────────

#[inline] pub fn qb_bool(v: f64) -> bool     { v != 0.0 }
#[inline] pub fn qb_from_bool(b: bool) -> f64 { if b { -1.0 } else { 0.0 } }
#[inline] pub fn qb_not(v: f64) -> f64        { (!(v as i64)) as f64 }

#[inline] pub fn qb_and(a: f64, b: f64) -> f64 { ((a as i64) & (b as i64)) as f64 }
#[inline] pub fn qb_or(a: f64, b: f64)  -> f64 { ((a as i64) | (b as i64)) as f64 }
#[inline] pub fn qb_eqv(a: f64, b: f64) -> f64 { (!((a as i64) ^ (b as i64))) as f64 }
#[inline] pub fn qb_imp(a: f64, b: f64) -> f64 { ((!(a as i64)) | (b as i64)) as f64 }

// ── Math functions ────────────────────────────────────────────────────────────

#[inline] pub fn qb_int(x: f64) -> f64   { x.floor() }
#[inline] pub fn qb_fix(x: f64) -> f64   { x.trunc() }
#[inline] pub fn qb_abs(x: f64) -> f64   { x.abs() }
#[inline] pub fn qb_sqr(x: f64) -> f64   { x.sqrt() }
#[inline] pub fn qb_sgn(x: f64) -> f64   { if x > 0.0 { 1.0 } else if x < 0.0 { -1.0 } else { 0.0 } }
/// QB `CINT` — round to nearest integer using **banker's rounding**
/// (ties round to even), matching QuickBASIC. Note this differs from Rust's
/// `f64::round()`, which rounds halves away from zero.
#[inline]
pub fn qb_cint(x: f64) -> f64 {
    if (x - x.trunc()).abs() == 0.5 {
        // Exactly halfway → round toward the even neighbour.
        let f = x.floor();
        if (f as i64) % 2 == 0 { f } else { f + 1.0 }
    } else {
        x.round()
    }
}

/// QB `\` integer division. Both operands are rounded to integers (CINT,
/// banker's) first, then divided with truncation toward zero.
#[inline]
pub fn qb_idiv(l: f64, r: f64) -> f64 {
    (qb_cint(l) as i64 / qb_cint(r) as i64) as f64
}

/// QB `MOD`. Both operands are rounded to integers (CINT, banker's) first,
/// then the remainder is taken. Rust's `%` on integers already yields the
/// QB-correct sign (that of the dividend).
#[inline]
pub fn qb_mod(l: f64, r: f64) -> f64 {
    (qb_cint(l) as i64 % qb_cint(r) as i64) as f64
}
#[inline] pub fn qb_sin(x: f64) -> f64   { x.sin() }
#[inline] pub fn qb_cos(x: f64) -> f64   { x.cos() }
#[inline] pub fn qb_tan(x: f64) -> f64   { x.tan() }
#[inline] pub fn qb_atn(x: f64) -> f64   { x.atan() }
#[inline] pub fn qb_exp(x: f64) -> f64   { x.exp() }
#[inline] pub fn qb_log(x: f64) -> f64   { x.ln() }

pub fn qb_timer() -> f64 {
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default();
    now.as_secs_f64() % 86400.0
}

// ── String functions ──────────────────────────────────────────────────────────

pub fn qb_str(v: impl std::fmt::Display) -> String { format!("{v}") }

/// QB PRINT numeric format: leading space for positive numbers, trailing space
/// after all numbers.  Used by emitted PRINT statements for numeric expressions.
pub fn qb_print_num(n: f64) -> String {
    // Format the number like QB: integers without ".0", floats with minimal digits.
    let s = format_qb_num(n);
    if n >= 0.0 { format!(" {s} ") } else { format!("{s} ") }
}

/// Format f64 the QB way: no trailing ".0" for integers, enough digits for floats.
pub fn format_qb_num(n: f64) -> String {
    if n.is_infinite() {
        return if n > 0.0 { "1E+38".to_string() } else { "-1E+38".to_string() };
    }
    if n.is_nan() { return "0".to_string(); }
    // If the value is an integer (within f64 precision), show without decimal
    if n.fract() == 0.0 && n.abs() < 1e15 {
        return format!("{}", n as i64);
    }
    // Otherwise use Rust's default display (which omits trailing zeros)
    format!("{n}")
}

pub fn qb_str_fn(n: f64) -> String {
    let s = format_qb_num(n);
    if n >= 0.0 { format!(" {s}") } else { s }
}

pub fn qb_len(s: &str) -> f64 { s.chars().count() as f64 }

pub fn qb_left(s: &str, n: f64) -> String {
    let n = n.max(0.0) as usize;
    s.chars().take(n).collect()
}

pub fn qb_right(s: &str, n: f64) -> String {
    let n = n.max(0.0) as usize;
    let chars: Vec<char> = s.chars().collect();
    let start = chars.len().saturating_sub(n);
    chars[start..].iter().collect()
}

pub fn qb_mid(s: &str, pos: f64, len: Option<f64>) -> String {
    let chars: Vec<char> = s.chars().collect();
    let start = ((pos as usize).saturating_sub(1)).min(chars.len());
    let slice = &chars[start..];
    match len {
        Some(l) => slice.iter().take(l.max(0.0) as usize).collect(),
        None    => slice.iter().collect(),
    }
}

/// MID$(var$, pos[, len]) = val — in-place substring replacement.
/// Replaces up to `len` characters in `s` starting at 1-based `pos` with
/// characters from `val`. The string length is never changed.
pub fn qb_mid_assign(s: &mut String, pos: f64, len: Option<f64>, val: &str) {
    let mut chars: Vec<char> = s.chars().collect();
    let start = (pos as usize).saturating_sub(1);
    if start >= chars.len() { return; }
    let max_replace = chars.len() - start;
    let replace_len = len.map(|l| (l as usize).min(max_replace)).unwrap_or(max_replace);
    for (i, c) in val.chars().take(replace_len).enumerate() {
        chars[start + i] = c;
    }
    *s = chars.into_iter().collect();
}

pub fn qb_ucase(s: &str) -> String { s.to_uppercase() }
pub fn qb_lcase(s: &str) -> String { s.to_lowercase() }
pub fn qb_ltrim(s: &str) -> String { s.trim_start().to_string() }
pub fn qb_rtrim(s: &str) -> String { s.trim_end().to_string() }
pub fn qb_trim(s: &str)  -> String { s.trim().to_string() }
pub fn qb_space(n: f64)  -> String { " ".repeat(n.max(0.0) as usize) }

pub fn qb_chr(n: f64) -> String {
    char::from_u32(n as u32).map(|c| c.to_string()).unwrap_or_default()
}

pub fn qb_asc(s: &str) -> f64 {
    s.chars().next().map(|c| c as u32 as f64).unwrap_or(0.0)
}

pub fn qb_val(s: &str) -> f64 {
    // QB VAL parses the longest valid numeric prefix (after leading whitespace)
    // and ignores the rest. A sign/exponent char is only valid in its grammatical
    // position — e.g. VAL("1-2") = 1, VAL("12e") = 12, VAL("-.5") = -0.5.
    //
    // Also handles QB hex/octal prefixes:
    //   &H or &h  →  parse hex digits that follow
    //   &O or &o  →  parse octal digits that follow
    let trimmed = s.trim_start();
    if trimmed.starts_with("&H") || trimmed.starts_with("&h") {
        let hex = &trimmed[2..];
        let end = hex.bytes().take_while(|b| b.is_ascii_hexdigit()).count();
        return if end > 0 {
            i64::from_str_radix(&hex[..end], 16).unwrap_or(0) as f64
        } else { 0.0 };
    }
    if trimmed.starts_with("&O") || trimmed.starts_with("&o") {
        let oct = &trimmed[2..];
        let end = oct.bytes().take_while(|b| matches!(b, b'0'..=b'7')).count();
        return if end > 0 {
            i64::from_str_radix(&oct[..end], 8).unwrap_or(0) as f64
        } else { 0.0 };
    }

    let bytes = s.trim_start().as_bytes();
    let mut i = 0;
    let n = bytes.len();
    let is_digit = |b: u8| b.is_ascii_digit();

    // Optional leading sign.
    if i < n && (bytes[i] == b'+' || bytes[i] == b'-') { i += 1; }
    // Integer part.
    while i < n && is_digit(bytes[i]) { i += 1; }
    // Fractional part.
    if i < n && bytes[i] == b'.' {
        i += 1;
        while i < n && is_digit(bytes[i]) { i += 1; }
    }
    // Exponent: e/E, optional sign, then at least one digit (else don't consume it).
    if i < n && (bytes[i] == b'e' || bytes[i] == b'E') {
        let mut j = i + 1;
        if j < n && (bytes[j] == b'+' || bytes[j] == b'-') { j += 1; }
        if j < n && is_digit(bytes[j]) {
            j += 1;
            while j < n && is_digit(bytes[j]) { j += 1; }
            i = j;
        }
    }

    std::str::from_utf8(&bytes[..i]).ok()
        .and_then(|t| t.parse::<f64>().ok())
        .unwrap_or(0.0)
}

pub fn qb_instr(start: f64, haystack: &str, needle: &str) -> f64 {
    let hchars: Vec<char> = haystack.chars().collect();
    let nchars: Vec<char> = needle.chars().collect();
    // QB: `start` is 1-based; values < 1 are treated as 1.
    let start1 = if start < 1.0 { 1 } else { start as usize };
    // QB: if `start` is past the end of the string, INSTR returns 0
    // (this is what previously leaked a `len+1` result for an empty needle).
    if start1 > hchars.len() { return 0.0; }
    let start_idx = start1 - 1;
    // QB: a null search string returns the start position.
    if nchars.is_empty() { return start1 as f64; }
    if hchars.len() < nchars.len() { return 0.0; }
    for i in start_idx..=hchars.len() - nchars.len() {
        if hchars[i..i + nchars.len()] == nchars[..] {
            return (i + 1) as f64;
        }
    }
    0.0
}

pub fn qb_string(n: f64, c: f64) -> String {
    let ch = char::from_u32(c as u32).unwrap_or(' ');
    ch.to_string().repeat(n.max(0.0) as usize)
}

/// STRING$(n, s$) — n copies of the first character of s$
pub fn qb_string_s(n: f64, s: &str) -> String {
    let ch = s.chars().next().unwrap_or(' ');
    ch.to_string().repeat(n.max(0.0) as usize)
}

pub fn qb_hex(n: f64) -> String { format!("{:X}", n as i64) }
pub fn qb_oct(n: f64) -> String { format!("{:o}", n as i64) }

#[inline] pub fn qb_xor(a: f64, b: f64) -> f64  { ((a as i64) ^ (b as i64)) as f64 }
#[inline] pub fn qb_csng(x: f64) -> f64          { x }
#[inline] pub fn qb_cdbl(x: f64) -> f64          { x }
// POKE/PEEK are now methods on Runtime so they share the poke_mem store.
// The free function stub is kept for any remaining call sites.
#[inline] pub fn qb_peek(_addr: f64) -> f64      { 0.0 } // stub — not used when Runtime available

pub fn qb_environ(name: &str) -> String {
    std::env::var(name).unwrap_or_default()
}

pub fn qb_read_data(data: &[&str], ptr: &std::sync::atomic::AtomicUsize) -> String {
    let idx = ptr.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
    data.get(idx).copied().unwrap_or("").to_string()
}

// ── QB file-I/O and binary-data helpers ──────────────────────────────────────

/// CVD — convert 8-byte IEEE 754 little-endian binary string to f64.
pub fn CVD(s: impl AsRef<str>) -> f64 {
    let bytes: Vec<u8> = s.as_ref().chars().take(8).map(|c| c as u32 as u8).collect();
    if bytes.len() < 8 { return 0.0; }
    f64::from_le_bytes(bytes.try_into().unwrap())
}
/// CVS — convert 4-byte binary string (Latin-1) to f64 (via f32).
pub fn CVS(s: impl AsRef<str>) -> f64 {
    let bytes: Vec<u8> = s.as_ref().chars().take(4).map(|c| c as u32 as u8).collect();
    if bytes.len() < 4 { return 0.0; }
    f32::from_le_bytes(bytes.try_into().unwrap()) as f64
}
/// CVI — convert 2-byte little-endian binary string (Latin-1) to integer.
pub fn CVI(s: impl AsRef<str>) -> f64 {
    let bytes: Vec<u8> = s.as_ref().chars().take(2).map(|c| c as u32 as u8).collect();
    if bytes.len() < 2 { return 0.0; }
    i16::from_le_bytes([bytes[0], bytes[1]]) as f64
}
/// CVL — convert 4-byte little-endian binary string (Latin-1) to long.
pub fn CVL(s: impl AsRef<str>) -> f64 {
    let bytes: Vec<u8> = s.as_ref().chars().take(4).map(|c| c as u32 as u8).collect();
    if bytes.len() < 4 { return 0.0; }
    i32::from_le_bytes(bytes.try_into().unwrap()) as f64
}

/// MKD$ — encode f64 as 8-byte Latin-1 binary string (each byte → one char U+0000..U+00FF).
pub fn MKD(n: f64) -> String {
    n.to_le_bytes().iter().map(|&b| char::from_u32(b as u32).unwrap()).collect()
}
/// MKS$ — encode f64 (as f32) as 4-byte Latin-1 binary string.
pub fn MKS(n: f64) -> String {
    (n as f32).to_le_bytes().iter().map(|&b| char::from_u32(b as u32).unwrap()).collect()
}
/// MKI$ — encode integer as 2-byte Latin-1 binary string.
pub fn MKI(n: f64) -> String {
    (n as i16).to_le_bytes().iter().map(|&b| char::from_u32(b as u32).unwrap()).collect()
}
/// MKL$ — encode long as 4-byte Latin-1 binary string.
pub fn MKL(n: f64) -> String {
    (n as i32).to_le_bytes().iter().map(|&b| char::from_u32(b as u32).unwrap()).collect()
}

/// LSET — left-justify src into dest's length (pad with spaces; truncate if longer).
/// Lengths are in chars (not UTF-8 bytes) because binary strings use Latin-1 encoding.
pub fn qb_lset(dest: &str, src: &str) -> String {
    let n = dest.chars().count();
    let src_n = src.chars().count();
    if src_n >= n {
        src.chars().take(n).collect()
    } else {
        let mut s: String = src.chars().collect();
        for _ in 0..(n - src_n) { s.push(' '); }
        s
    }
}

/// RSET — right-justify src into dest's length (pad with spaces on left).
pub fn qb_rset(dest: &str, src: &str) -> String {
    let n = dest.chars().count();
    let src_n = src.chars().count();
    if src_n >= n {
        src.chars().skip(src_n - n).collect()
    } else {
        let mut s = String::new();
        for _ in 0..(n - src_n) { s.push(' '); }
        s.push_str(src);
        s
    }
}

/// qb_field_get — extract `len` bytes starting at `offset` from a record buffer.
/// Returns a Latin-1 encoded String of exactly `len` chars.
pub fn qb_field_get(buf: &[u8], offset: usize, len: usize) -> String {
    let mut s = String::with_capacity(len);
    for i in 0..len {
        let b = if offset + i < buf.len() { buf[offset + i] } else { b' ' };
        s.push(char::from_u32(b as u32).unwrap());
    }
    s
}

/// qb_field_put — write a Latin-1 string into a record buffer at (offset, len).
pub fn qb_field_put(buf: &mut Vec<u8>, offset: usize, s: &str, len: usize) {
    let chars: Vec<u8> = s.chars().map(|c| c as u32 as u8).collect();
    for i in 0..len {
        if offset + i < buf.len() {
            buf[offset + i] = *chars.get(i).unwrap_or(&b' ');
        }
    }
}

// ── Random-access TYPE-record (GET/PUT #n, rec, var) pack/unpack ───────────────
//
// Helpers for serializing a QB user-TYPE variable to/from a fixed-length record
// buffer. Fixed strings are raw ASCII bytes (space-padded); numerics use
// little-endian: INTEGER=i16, LONG=i32, SINGLE=f32 IEEE, DOUBLE=f64 IEEE.
// (QuickBASIC 1.1 stored SINGLE/DOUBLE in MBF, not IEEE — see CLAUDE.md. The
// integer and fixed-string encodings are byte-exact with DOS.) All getters/
// setters are bounds-checked: a short buffer pads/zeros gracefully, matching the
// padding done by read_record/write_record.

/// Write a fixed-length string field: `n` raw bytes at `off`, space-padded.
pub fn qb_rec_put_str(buf: &mut [u8], off: usize, s: &str, n: usize) {
    let bytes = s.as_bytes();
    for i in 0..n {
        if off + i < buf.len() {
            buf[off + i] = *bytes.get(i).unwrap_or(&b' ');
        }
    }
}

/// Read a fixed-length string field: `n` bytes at `off`, space-padded if short.
pub fn qb_rec_get_str(buf: &[u8], off: usize, n: usize) -> String {
    let end = (off + n).min(buf.len());
    let slice = if off < buf.len() { &buf[off..end] } else { &[][..] };
    let s = String::from_utf8_lossy(slice).into_owned();
    if s.len() < n { format!("{:<width$}", s, width = n) } else { s }
}

fn rec_write(buf: &mut [u8], off: usize, bytes: &[u8]) {
    for (i, b) in bytes.iter().enumerate() {
        if off + i < buf.len() { buf[off + i] = *b; }
    }
}
fn rec_read<const N: usize>(buf: &[u8], off: usize) -> [u8; N] {
    let mut out = [0u8; N];
    for (i, slot) in out.iter_mut().enumerate() {
        if let Some(b) = buf.get(off + i) { *slot = *b; }
    }
    out
}

/// INTEGER — 2-byte little-endian. QB rounds to nearest on store.
pub fn qb_rec_put_i16(buf: &mut [u8], off: usize, v: f64) {
    rec_write(buf, off, &(v.round() as i64 as i16).to_le_bytes());
}
pub fn qb_rec_get_i16(buf: &[u8], off: usize) -> f64 {
    i16::from_le_bytes(rec_read::<2>(buf, off)) as f64
}

/// LONG — 4-byte little-endian.
pub fn qb_rec_put_i32(buf: &mut [u8], off: usize, v: f64) {
    rec_write(buf, off, &(v.round() as i64 as i32).to_le_bytes());
}
pub fn qb_rec_get_i32(buf: &[u8], off: usize) -> f64 {
    i32::from_le_bytes(rec_read::<4>(buf, off)) as f64
}

/// SINGLE — 4-byte IEEE little-endian.
pub fn qb_rec_put_f32(buf: &mut [u8], off: usize, v: f64) {
    rec_write(buf, off, &(v as f32).to_le_bytes());
}
pub fn qb_rec_get_f32(buf: &[u8], off: usize) -> f64 {
    f32::from_le_bytes(rec_read::<4>(buf, off)) as f64
}

/// DOUBLE — 8-byte IEEE little-endian.
pub fn qb_rec_put_f64(buf: &mut [u8], off: usize, v: f64) {
    rec_write(buf, off, &v.to_le_bytes());
}
pub fn qb_rec_get_f64(buf: &[u8], off: usize) -> f64 {
    f64::from_le_bytes(rec_read::<8>(buf, off))
}

/// EOF(n) — returns -1 (true) when file n is exhausted, 0 otherwise.
/// Called as a QB function returning f64; the runtime's eof_check does the work.
pub fn qb_eof_fn(_file_num: f64) -> f64 { 0.0 } // conservative: let read fail naturally

/// LOF(n) — length of open file in bytes (returns 0 if not available).
pub fn qb_lof_fn(_file_num: f64) -> f64 { 0.0 }

/// VARPTR — returns a fake memory address (stub: always 0)
pub fn varptr(_: f64) -> f64 { 0.0 }
/// ABSOLUTE — call machine-code at address (stub: no-op)
pub fn absolute(_addr: f64) {}

/// Old stubs kept for backward compat with any emitted code that still uses them.
pub fn qb_open(_path: &str, _mode: &str, _file_num: f64, _len: f64) {}
pub fn qb_close(_file_num: f64) {}

/// BEEP without runtime access
pub fn qb_beep_free() {}

/// ON ERROR GOTO — stub (just note the handler, don't install it)
pub fn qb_on_error(_label: f64) {}

/// WIDTH — set terminal width (stub)
pub fn qb_width(_cols: f64, _rows: f64) {}

/// LPRINT — print to printer (stub: print to stdout)
pub fn qb_lprint(s: &str) { println!("{s}"); }

#[cfg(test)]
mod rng_and_logic_tests {
    use super::*;

    #[test]
    fn qb_lcg_first_value_is_the_famous_7055475() {
        // DOS QBasic 1.1: first RND without RANDOMIZE = .7055475
        let mut rt = Runtime::headless();
        let v = rt.rnd();
        assert!((v - 0.7055475).abs() < 1e-6, "got {v}");
    }

    #[test]
    fn rnd_zero_repeats_last_value() {
        let mut rt = Runtime::headless();
        let a = rt.rnd();
        assert_eq!(rt.rnd_arg(0.0), a);
        assert_eq!(rt.rnd_arg(0.0), a);
        let b = rt.rnd();
        assert_ne!(a, b);
        assert_eq!(rt.rnd_arg(0.0), b);
    }

    #[test]
    fn rnd_negative_reseeds_deterministically() {
        let mut rt = Runtime::headless();
        let a = rt.rnd_arg(-3.5);
        let _ = rt.rnd();
        let b = rt.rnd_arg(-3.5);
        assert_eq!(a, b);
    }

    #[test]
    fn rnd_positive_advances() {
        let mut rt = Runtime::headless();
        let a = rt.rnd_arg(1.0);
        let b = rt.rnd_arg(1.0);
        assert_ne!(a, b);
    }

    #[test]
    fn eqv_imp_truth_table() {
        // QB EQV = bitwise NOT(a XOR b); IMP = (NOT a) OR b
        assert_eq!(qb_eqv(5.0, 3.0), -7.0);
        assert_eq!(qb_eqv(-1.0, -1.0), -1.0);
        assert_eq!(qb_eqv(0.0, -1.0), 0.0);
        assert_eq!(qb_imp(5.0, 3.0), -5.0);
        assert_eq!(qb_imp(0.0, 5.0), -1.0);
        assert_eq!(qb_imp(-1.0, 0.0), 0.0);
    }
}

#[cfg(test)]
mod print_using_tests {
    use super::{qb_print_using, QbVal};

    fn pu(fmt: &str, n: f64) -> String {
        qb_print_using(fmt, &[QbVal::Num(n)])
    }

    // ── Basic numeric formatting (regression guard) ──────────────────────────
    #[test]
    fn basic_integer_field() {
        assert_eq!(pu("###", 42.0), " 42");
    }
    #[test]
    fn basic_fixed_point() {
        assert_eq!(pu("##.##", 3.14159), " 3.14");
    }
    #[test]
    fn comma_grouping() {
        assert_eq!(pu("#,###,###", 1234567.0), "1,234,567");
    }
    #[test]
    fn negative_leading_sign() {
        assert_eq!(pu("##.##", -3.5), "-3.50");
    }

    // ── Exponential `^^^^` ───────────────────────────────────────────────────
    #[test]
    fn exp_one_int_digit() {
        assert_eq!(pu("#.##^^^^", 234.56), "2.35E+02");
    }
    #[test]
    fn exp_two_int_digits_pads_sign_space() {
        // Mantissa normalizes to one integer digit; the extra `#` becomes the
        // (positive) sign space.
        assert_eq!(pu("##.##^^^^", 234.56), " 2.35E+02");
    }
    #[test]
    fn exp_negative_uses_sign_slot() {
        assert_eq!(pu("##.##^^^^", -234.56), "-2.35E+02");
    }
    #[test]
    fn exp_leading_decimal_form() {
        assert_eq!(pu(".####^^^^", 888888.0), ".8889E+06");
    }
    #[test]
    fn exp_zero() {
        assert_eq!(pu("#.##^^^^", 0.0), "0.00E+00");
    }
    #[test]
    fn exp_small_negative_exponent() {
        // 0.00123 → 1.23E-03
        assert_eq!(pu("#.##^^^^", 0.00123), "1.23E-03");
    }
    #[test]
    fn exp_rounding_carry_bumps_exponent() {
        // 9.999 with one fractional digit rounds to 10.0 → 1.0E+01
        assert_eq!(pu("#.#^^^^", 9.99), "1.0E+01");
    }
    #[test]
    fn exp_five_carets_widens_exponent() {
        assert_eq!(pu("#.##^^^^^", 234.56), "2.35E+002");
    }
    #[test]
    fn three_carets_are_literal_not_exponent() {
        // Fewer than 4 carets: the integer field prints, then literal '^^^'.
        assert_eq!(pu("##^^^", 5.0), " 5^^^");
    }

    // ── Overflow `%` (the "wide field" edge case) ────────────────────────────
    #[test]
    fn overflow_integer_field() {
        assert_eq!(pu("##", 123.0), "%123");
    }
    #[test]
    fn overflow_with_fraction() {
        assert_eq!(pu("##.#", 1234.5), "%1234.5");
    }
    #[test]
    fn overflow_negative() {
        assert_eq!(pu("#", -12.0), "%-12");
    }
    #[test]
    fn no_overflow_when_it_fits() {
        assert_eq!(pu("###", 12.0), " 12");
    }

    // ── Literal escape `_X` ──────────────────────────────────────────────────
    #[test]
    fn literal_escape_hash() {
        assert_eq!(qb_print_using("_#", &[]), "#");
    }
    #[test]
    fn literal_escape_mixed_with_field() {
        // `_#` → literal '#', then `###` numeric field for 5 → "  5".
        assert_eq!(pu("_####", 5.0), "#  5");
    }

    // ── String fields still work alongside the new code ──────────────────────
    #[test]
    fn string_amp_field() {
        assert_eq!(qb_print_using("&", &[QbVal::Str("hi")]), "hi");
    }
}

#[cfg(test)]
mod print_using_prefix_tests {
    use super::{qb_print_using, QbVal};
    fn pu(fmt: &str, n: f64) -> String { qb_print_using(fmt, &[QbVal::Num(n)]) }

    // ── $$ floating dollar ───────────────────────────────────────────────────
    #[test] fn dollar_dollar_one()      { assert_eq!(pu("$$###", 1.0),    "   $1"); }
    #[test] fn dollar_dollar_two()      { assert_eq!(pu("$$###", 12.0),   "  $12"); }
    #[test] fn dollar_dollar_full()     { assert_eq!(pu("$$###", 123.0),  " $123"); }
    #[test] fn dollar_dollar_negative() { assert_eq!(pu("$$###", -12.0),  " -$12"); }
    #[test] fn dollar_dollar_frac()     { assert_eq!(pu("$$##.##", 1.23), "  $1.23"); }
    #[test] fn dollar_dollar_overflow() { assert_eq!(pu("$$###", 1234.0), "%1234"); }

    // ── ** asterisk fill ─────────────────────────────────────────────────────
    #[test] fn star_star_one()          { assert_eq!(pu("**###", 1.0),      "****1"); }
    #[test] fn star_star_two()          { assert_eq!(pu("**###", 12.0),     "***12"); }
    #[test] fn star_star_three()        { assert_eq!(pu("**###", 123.0),    "**123"); }
    #[test] fn star_star_fill_all()     { assert_eq!(pu("**###", 12345.0),  "12345"); }
    #[test] fn star_star_negative()     { assert_eq!(pu("**###", -12.0),    "**-12"); }
    #[test] fn star_star_overflow()     { assert_eq!(pu("**###", 123456.0), "%123456"); }

    // ── **$ combination ──────────────────────────────────────────────────────
    #[test] fn star_dollar_basic()      { assert_eq!(pu("**$###", 12.0),    "***$12"); }
    #[test] fn star_dollar_full()       { assert_eq!(pu("**$###", 12345.0), "$12345"); }
    #[test] fn star_dollar_overflow()   { assert_eq!(pu("**$###", 123456.0),"%123456"); }

    // ── money.bas-style format ───────────────────────────────────────────────
    #[test] fn money_format() {
        assert_eq!(pu("$$###,###.##", 1234.56), "  $1,234.56");
    }
}

#[cfg(test)]
mod numeric_tests {
    use super::{qb_cint, qb_idiv, qb_instr, qb_mod, qb_val};

    // ── INSTR edge cases (QB rules) ──────────────────────────────────────────
    #[test]
    fn instr_found_and_not_found() {
        assert_eq!(qb_instr(1.0, "Hello, World!", "World"), 8.0);
        assert_eq!(qb_instr(1.0, "Hello", "xyz"), 0.0);
    }
    #[test]
    fn instr_empty_needle_returns_start() {
        assert_eq!(qb_instr(1.0, "abc", ""), 1.0);
        assert_eq!(qb_instr(3.0, "abc", ""), 3.0);
    }
    #[test]
    fn instr_start_past_end_returns_zero() {
        // Previously leaked len+1 for an empty needle; QB returns 0.
        assert_eq!(qb_instr(4.0, "abc", ""), 0.0);
        assert_eq!(qb_instr(10.0, "abc", "a"), 0.0);
        assert_eq!(qb_instr(1.0, "", ""), 0.0);
    }
    #[test]
    fn instr_start_offsets_search() {
        assert_eq!(qb_instr(1.0, "abcabc", "bc"), 2.0);
        assert_eq!(qb_instr(3.0, "abcabc", "bc"), 5.0);
    }

    // ── CINT: banker's rounding (ties to even) ───────────────────────────────
    #[test]
    fn cint_ties_round_to_even() {
        assert_eq!(qb_cint(0.5), 0.0);
        assert_eq!(qb_cint(1.5), 2.0);
        assert_eq!(qb_cint(2.5), 2.0);
        assert_eq!(qb_cint(3.5), 4.0);
        assert_eq!(qb_cint(-0.5), 0.0);
        assert_eq!(qb_cint(-1.5), -2.0);
        assert_eq!(qb_cint(-2.5), -2.0);
        assert_eq!(qb_cint(-3.5), -4.0);
    }
    #[test]
    fn cint_non_ties_round_to_nearest() {
        assert_eq!(qb_cint(2.4), 2.0);
        assert_eq!(qb_cint(2.6), 3.0);
        assert_eq!(qb_cint(-2.4), -2.0);
        assert_eq!(qb_cint(-2.6), -3.0);
        assert_eq!(qb_cint(7.0), 7.0);
    }

    // ── `\` integer divide: operands CINT-rounded, result truncates to zero ──
    #[test]
    fn idiv_basic_and_truncation() {
        assert_eq!(qb_idiv(7.0, 2.0), 3.0);
        assert_eq!(qb_idiv(-7.0, 2.0), -3.0); // truncate toward zero
        assert_eq!(qb_idiv(7.0, -2.0), -3.0);
    }
    #[test]
    fn idiv_rounds_operands_first() {
        // 2.6 \ 1  →  CINT(2.6)=3, 3\1 = 3   (old code truncated 2.6→2 giving 2)
        assert_eq!(qb_idiv(2.6, 1.0), 3.0);
        // 10.6 \ 3 → 11 \ 3 = 3
        assert_eq!(qb_idiv(10.6, 3.0), 3.0);
    }

    // ── MOD: operands CINT-rounded, remainder takes dividend's sign ──────────
    #[test]
    fn mod_sign_follows_dividend() {
        assert_eq!(qb_mod(-7.0, 3.0), -1.0);
        assert_eq!(qb_mod(7.0, -3.0), 1.0);
        assert_eq!(qb_mod(7.0, 3.0), 1.0);
    }
    #[test]
    fn mod_rounds_operands_first() {
        // 2.7 MOD 2 → CINT(2.7)=3, 3 MOD 2 = 1  (old float % gave 0.7)
        assert_eq!(qb_mod(2.7, 2.0), 1.0);
    }

    // ── VAL: longest valid numeric prefix ────────────────────────────────────
    #[test]
    fn val_basic() {
        assert_eq!(qb_val("3.14"), 3.14);
        assert_eq!(qb_val("  42"), 42.0);
        assert_eq!(qb_val("-.5"), -0.5);
        assert_eq!(qb_val("1.5e3"), 1500.0);
        assert_eq!(qb_val("2E-2"), 0.02);
        // hex / octal prefixes
        assert_eq!(qb_val("&H6F"), 111.0);
        assert_eq!(qb_val("&hFF"), 255.0);
        assert_eq!(qb_val("&H0"),  0.0);
        assert_eq!(qb_val("&O10"), 8.0);
        assert_eq!(qb_val("&o77"), 63.0);
    }
    #[test]
    fn val_stops_at_invalid_chars() {
        assert_eq!(qb_val("1-2"), 1.0);     // mid-string sign not consumed
        assert_eq!(qb_val("12e"), 12.0);    // bare exponent marker dropped
        assert_eq!(qb_val("12e+"), 12.0);   // exponent with no digit dropped
        assert_eq!(qb_val(" 3.14abc"), 3.14);
        assert_eq!(qb_val("abc"), 0.0);
        assert_eq!(qb_val(""), 0.0);
    }
}

#[cfg(test)]
mod screen13_tests {
    use super::{Runtime, vga256_default, dac18_to_rgb, DEFAULT_PALETTE_256, EGA};

    // SCREEN 13 keeps color indices 0–255 (no mod-16 wrap); EGA modes still wrap.
    #[test]
    fn mode13_preserves_high_indices() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        rt.pset(10.0, 10.0, 50.0);
        rt.pset(20.0, 20.0, 200.0);
        assert_eq!(rt.point(10.0, 10.0), 50.0);
        assert_eq!(rt.point(20.0, 20.0), 200.0);
    }

    #[test]
    fn ega_mode_wraps_mod16() {
        let mut rt = Runtime::headless();
        rt.screen(9.0);
        rt.pset(10.0, 10.0, 50.0); // 50 % 16 == 2
        assert_eq!(rt.point(10.0, 10.0), 2.0);
    }

    // SCREEN 13 PALETTE uses an 18-bit DAC value (channels 0–63), not EGA irgb.
    #[test]
    fn mode13_palette_dac_decode() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        // pure red at full intensity: red=63 → 8-bit 255.
        rt.palette(50.0, 63.0);
        assert_eq!(rt.palette_rgb[50], (255, 0, 0));
        // green=63 → encoded as 63*256; blue=63 → 63*65536.
        rt.palette(51.0, 63.0 * 256.0);
        assert_eq!(rt.palette_rgb[51], (0, 255, 0));
        rt.palette(52.0, 63.0 * 65536.0);
        assert_eq!(rt.palette_rgb[52], (0, 0, 255));
        // mid value 31 → (31<<2)|(31>>4) = 124|1 = 125.
        assert_eq!(dac18_to_rgb(31), (125, 0, 0));
    }

    // The default tables: slot 0–15 are EGA in both; mode-13 default fills 16–255.
    #[test]
    fn default_palettes_shape() {
        assert_eq!(DEFAULT_PALETTE_256[0], EGA[0]);
        assert_eq!(DEFAULT_PALETTE_256[15], EGA[15]);
        assert_eq!(DEFAULT_PALETTE_256[200], (0, 0, 0)); // EGA-mode default: black above 15
        let vga = vga256_default();
        assert_eq!(vga[0], EGA[0]);            // EGA colors preserved
        assert_eq!(vga[31], (255, 255, 255));  // grayscale ramp ends at white
        assert_eq!(vga[32], (0, 0, 255));      // first HSV cycle starts at full blue
        assert_eq!(vga[255], (0, 0, 0));       // tail is black
    }
}

#[cfg(test)]
mod step_tests {
    use super::Runtime;

    // The graphics cursor (QB "last point referenced") tracks PSET and LINE.
    #[test]
    fn pset_updates_cursor() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        rt.pset(10.0, 20.0, 4.0);
        assert_eq!((rt.cur_x(), rt.cur_y()), (10.0, 20.0));
    }

    #[test]
    fn line_updates_cursor_to_endpoint() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        rt.line(5.0, 5.0, 40.0, 25.0, 7.0);
        assert_eq!((rt.cur_x(), rt.cur_y()), (40.0, 25.0));
    }

    // QB moves the last point referenced to the CIRCLE center (needed for a
    // following STEP coordinate). Our circle() now does this.
    #[test]
    fn circle_updates_cursor_to_center() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        rt.pset(0.0, 0.0, 1.0);
        rt.circle(50.0, 60.0, 8.0, 3.0);
        assert_eq!((rt.cur_x(), rt.cur_y()), (50.0, 60.0));
    }
}

#[cfg(test)]
mod window_tests {
    use super::Runtime;

    // QB: WINDOW without an explicit VIEW maps the logical rect onto the WHOLE
    // screen. The old bug left view_x1..view_x2 = 0, collapsing every point to
    // (0,0) so nothing rendered (this is what broke torus.bas).
    #[test]
    fn window_without_view_maps_to_full_screen() {
        let mut rt = Runtime::headless();
        rt.screen(12.0); // 640x480
        rt.set_window(-4.0, -4.0, 4.0, 4.0, false); // no set_view
        // Two distinct logical corners must land on distinct framebuffer pixels.
        rt.pset(-4.0, -4.0, 5.0); // → fb (0,0)
        rt.pset(4.0, 4.0, 9.0); // → fb (639,479)
        // Under the old (degenerate-viewport) bug both collapsed to (0,0) and the
        // second pset overwrote the first — both reads would return 9.0.
        assert_eq!(rt.point(-4.0, -4.0), 5.0);
        assert_eq!(rt.point(4.0, 4.0), 9.0);
    }

    // Mirror torus TileDraw's paint path: draw a quad outline in the border color,
    // PRESET an interior point, PAINT it with the tile color bounded by the border.
    // The interior must end up the tile color (not background) — i.e. PAINT fills
    // a region that LINE closed, under WINDOW logical coords without VIEW.
    #[test]
    fn paint_fills_quad_under_window() {
        let mut rt = Runtime::headless();
        rt.screen(12.0);
        rt.set_window(-4.0, -4.0, 4.0, 4.0, false);
        let border = 15.0;
        let tcolor = 9.0;
        // Quad in logical coords (well inside the window so it spans many pixels).
        rt.line(-2.0, -2.0, 2.0, -2.0, border);
        rt.line(2.0, -2.0, 2.0, 2.0, border);
        rt.line(2.0, 2.0, -2.0, 2.0, border);
        rt.line(-2.0, 2.0, -2.0, -2.0, border);
        // Fill the interior bounded by the border color.
        rt.paint(0.0, 0.0, tcolor, border);
        // Center is now the tile color; outside the quad is still background (0).
        assert_eq!(rt.point(0.0, 0.0), tcolor);
        assert_eq!(rt.point(3.5, 3.5), 0.0);
    }

    // SCREEN 12 PALETTE takes an 18-bit DAC value (channels 0–63), like SCREEN 13 —
    // not the EGA irgb nibble. torus mixes Pal() = 65536*B + 256*G + R this way.
    #[test]
    fn palette_screen12_uses_dac18() {
        let mut rt = Runtime::headless();
        rt.screen(12.0);
        // Full red: R=63 → DAC value 63. dac6_to_8(63) = 255.
        rt.palette(1.0, 63.0);
        assert_eq!(rt.palette_rgb[1], (255, 0, 0));
        // Full blue: B=63 → 65536*63. → (0,0,255).
        rt.palette(2.0, 65536.0 * 63.0);
        assert_eq!(rt.palette_rgb[2], (0, 0, 255));
    }

    // QB `WINDOW` (no SCREEN) inverts Y: larger logical y is higher on screen.
    // torus's Inside() scan depends on this; without it every tile reads as
    // "not inside" and gets erased to the background (black screen).
    #[test]
    fn window_inverts_y_axis() {
        let mut rt = Runtime::headless();
        rt.screen(12.0); // 640x480
        rt.set_window(-4.0, -4.0, 4.0, 4.0, false);
        // Top of window (logical y = +4) → physical row near 0.
        // Bottom (logical y = -4) → physical row near 479.
        // PMAP mode 1 = logical y → physical y.
        assert!(rt.pmap(4.0, 1.0) < 2.0, "top y → {}", rt.pmap(4.0, 1.0));
        assert!(rt.pmap(-4.0, 1.0) > 477.0, "bottom y → {}", rt.pmap(-4.0, 1.0));
        // mode 1 → mode 3 round-trips a logical Y through physical.
        let phys = rt.pmap(1.5, 1.0);
        assert!((rt.pmap(phys, 3.0) - 1.5).abs() < 0.05, "round-trip {}", rt.pmap(phys, 3.0));
    }

    // `WINDOW SCREEN` keeps screen orientation: NO Y inversion (used by reversi).
    // Plain WINDOW (above) inverts; this guards that the SCREEN variant doesn't.
    #[test]
    fn window_screen_no_y_invert() {
        let mut rt = Runtime::headless();
        rt.screen(12.0); // 640x480
        // reversi uses WINDOW SCREEN (640,480)-(0,0); use the simple identity-ish
        // form here to assert orientation directly.
        rt.set_window(0.0, 0.0, 100.0, 100.0, true); // screen = true → no invert
        // With screen orientation, logical y=0 → top (row ~0), y=100 → bottom (~479).
        assert!(rt.pmap(0.0, 1.0) < 2.0, "y=0 → {}", rt.pmap(0.0, 1.0));
        assert!(rt.pmap(100.0, 1.0) > 477.0, "y=100 → {}", rt.pmap(100.0, 1.0));
        // Contrast: the same window WITHOUT screen mode inverts.
        rt.set_window(0.0, 0.0, 100.0, 100.0, false);
        assert!(rt.pmap(0.0, 1.0) > 477.0, "inverted y=0 → {}", rt.pmap(0.0, 1.0));
        // mode 1 ↔ mode 3 still round-trips under screen mode.
        rt.set_window(0.0, 0.0, 100.0, 100.0, true);
        let phys = rt.pmap(40.0, 1.0);
        assert!((rt.pmap(phys, 3.0) - 40.0).abs() < 0.1, "round-trip {}", rt.pmap(phys, 3.0));
    }

    // reversi uses `WINDOW SCREEN (640,480)-(0,0)` — reversed corners. These must
    // NOT flip the image (the board would render rotated 180°, on the wrong side,
    // with backwards arrow keys). WINDOW SCREEN maps by magnitude: min → top-left.
    #[test]
    fn window_screen_reversed_corners_no_flip() {
        let mut rt = Runtime::headless();
        rt.screen(12.0); // 640x480
        rt.set_window(640.0, 480.0, 0.0, 0.0, true);
        // logical (0,0) → top-left pixel; logical (640,480) → bottom-right.
        assert!(rt.pmap(0.0, 0.0) < 2.0, "x=0 → {}", rt.pmap(0.0, 0.0));
        assert!(rt.pmap(640.0, 0.0) > 637.0, "x=640 → {}", rt.pmap(640.0, 0.0));
        assert!(rt.pmap(0.0, 1.0) < 2.0, "y=0 → {}", rt.pmap(0.0, 1.0));
        assert!(rt.pmap(480.0, 1.0) > 477.0, "y=480 → {}", rt.pmap(480.0, 1.0));
        // Board cell (1,1) at logical (290,90) must be above-and-left of (8,8) at (570,390).
        assert!(rt.pmap(90.0, 1.0) < rt.pmap(390.0, 1.0), "row1 above row8");
        assert!(rt.pmap(290.0, 0.0) < rt.pmap(570.0, 0.0), "col1 left of col8");
    }

    // PMAP must use the full-screen viewport when VIEW is inactive, and round-trip.
    #[test]
    fn pmap_uses_full_screen_without_view() {
        let mut rt = Runtime::headless();
        rt.screen(12.0);
        rt.set_window(0.0, 0.0, 100.0, 100.0, false); // no set_view
        // logical X 100 → physical right edge (639)
        let phys = rt.pmap(100.0, 0.0);
        assert!((phys - 639.0).abs() < 1.0, "mode 0 gave {phys}");
        // mode 2: viewport-relative physical → logical, round-trips back to 100
        let logical = rt.pmap(phys, 2.0);
        assert!((logical - 100.0).abs() < 1.0, "mode 2 gave {logical}");
    }
}

#[cfg(test)]
mod record_tests {
    use super::*;

    // Round-trip a HALLFAMEREC-shaped record (STRING*20, INTEGER, LONG = 26 bytes).
    #[test]
    fn round_trip_str_i16_i32() {
        let mut buf = vec![b' '; 26];
        qb_rec_put_str(&mut buf, 0, "LT CMDR CHICKEN", 20);
        qb_rec_put_i16(&mut buf, 20, 20.0);
        qb_rec_put_i32(&mut buf, 22, 100000.0);

        assert_eq!(qb_rec_get_str(&buf, 0, 20), "LT CMDR CHICKEN     "); // space-padded to 20
        assert_eq!(qb_rec_get_i16(&buf, 20), 20.0);
        assert_eq!(qb_rec_get_i32(&buf, 22), 100000.0);
    }

    #[test]
    fn integer_layout_is_little_endian_2_bytes() {
        let mut buf = vec![0u8; 4];
        qb_rec_put_i16(&mut buf, 0, 258.0); // 0x0102
        assert_eq!(&buf[0..2], &[0x02, 0x01]);
        assert_eq!(qb_rec_get_i16(&buf, 0), 258.0);
    }

    #[test]
    fn long_holds_negative_values() {
        let mut buf = vec![0u8; 4];
        qb_rec_put_i32(&mut buf, 0, -9999.0);
        assert_eq!(qb_rec_get_i32(&buf, 0), -9999.0);
    }

    #[test]
    fn single_and_double_ieee_round_trip() {
        let mut buf = vec![0u8; 12];
        qb_rec_put_f32(&mut buf, 0, 3.5);
        qb_rec_put_f64(&mut buf, 4, 2.718281828459045);
        assert_eq!(qb_rec_get_f32(&buf, 0), 3.5);
        assert_eq!(qb_rec_get_f64(&buf, 4), 2.718281828459045);
    }

    // A short/missing buffer must not panic — getters pad/zero, setters no-op.
    #[test]
    fn out_of_bounds_is_graceful() {
        let mut buf = vec![b' '; 4];
        qb_rec_put_i32(&mut buf, 2, 12345.0); // straddles end — partial write, no panic
        assert_eq!(qb_rec_get_str(&[], 0, 5), "     ");
        assert_eq!(qb_rec_get_i16(&buf, 100), 0.0);
    }
}

#[cfg(test)]
mod sprite_tests {
    use super::*;

    // A 2×1 EGA sprite: pixel(0)=color 1, pixel(1)=color 2.
    // header = (width-1)|((height-1)<<16) = 1; one data long packs 4 planes:
    // plane0 col0 (bit7)=0x80, plane1 col1 (bit6)=0x40 → 0x80 | 0x40<<8 = 0x4080.
    fn sprite_2x1() -> Vec<f64> { vec![1.0, 0x4080 as f64] }

    fn setup() -> Runtime {
        let mut rt = Runtime::headless();
        rt.screen(9.0); // EGA, sprite_color_mask = 15
        rt
    }

    #[test]
    fn pset_overwrites() {
        let mut rt = setup();
        rt.pset(0.0, 0.0, 7.0);
        rt.pset(1.0, 0.0, 7.0);
        rt.put_sprite(&sprite_2x1(), 0.0, 0.0, PutAction::Pset);
        assert_eq!(rt.point(0.0, 0.0), 1.0);
        assert_eq!(rt.point(1.0, 0.0), 2.0);
    }

    #[test]
    fn preset_inverts_within_mask() {
        let mut rt = setup();
        rt.put_sprite(&sprite_2x1(), 0.0, 0.0, PutAction::Preset);
        // !1 & 15 = 14, !2 & 15 = 13
        assert_eq!(rt.point(0.0, 0.0), 14.0);
        assert_eq!(rt.point(1.0, 0.0), 13.0);
    }

    #[test]
    fn xor_toggles() {
        let mut rt = setup();
        rt.pset(0.0, 0.0, 3.0); // 3 ^ 1 = 2
        rt.pset(1.0, 0.0, 3.0); // 3 ^ 2 = 1
        rt.put_sprite(&sprite_2x1(), 0.0, 0.0, PutAction::Xor);
        assert_eq!(rt.point(0.0, 0.0), 2.0);
        assert_eq!(rt.point(1.0, 0.0), 1.0);
        // XORing the same sprite again restores the background (draw/erase).
        rt.put_sprite(&sprite_2x1(), 0.0, 0.0, PutAction::Xor);
        assert_eq!(rt.point(0.0, 0.0), 3.0);
        assert_eq!(rt.point(1.0, 0.0), 3.0);
    }

    #[test]
    fn and_or_combine() {
        let mut rt = setup();
        rt.pset(0.0, 0.0, 3.0); // 3 & 1 = 1
        rt.pset(1.0, 0.0, 1.0); // 1 | 2 = 3
        let mut rt2 = setup();
        rt2.pset(1.0, 0.0, 1.0); // 1 | 2 = 3 (OR path)
        rt.put_sprite(&sprite_2x1(), 0.0, 0.0, PutAction::And);
        assert_eq!(rt.point(0.0, 0.0), 1.0); // 3 AND 1
        rt2.put_sprite(&sprite_2x1(), 0.0, 0.0, PutAction::Or);
        assert_eq!(rt2.point(1.0, 0.0), 3.0); // 1 OR 2
    }

    // DRAW "M-1,1": signed x, bare y. QB makes the whole move relative (the x
    // sign governs the pair), so the cursor ends at (start-1, start+1), NOT at
    // absolute y=1. (Regression guard for donkey's outline.)
    #[test]
    fn draw_m_relative_x_makes_pair_relative() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        rt.draw("S4");            // scale 4 → 1 pixel per unit
        rt.draw("BM20,20");       // blind absolute move to (20,20)
        rt.draw("M-1,1");         // relative: → (19, 21)
        assert_eq!((rt.cur_x(), rt.cur_y()), (19.0, 21.0));
        rt.draw("BM20,20");
        rt.draw("M14,18");        // unsigned x → absolute move to (14,18)
        assert_eq!((rt.cur_x(), rt.cur_y()), (14.0, 18.0));
    }

    // DRAW with no `C` verb paints in the current COLOR foreground (so a
    // following PAINT whose border matches that color sees the outline). Donkey
    // regression: COLOR sets fg, then DRAW "S08" (no C) must use it — not a
    // stale default — else PAINT floods. Here CGA COLOR fixes fg=3.
    #[test]
    fn draw_uses_current_color_foreground() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        rt.color(8.0, Some(1.0)); // CGA: foreground becomes 3
        rt.draw("R4");            // draw 4px right from (0,0) in the fg color
        assert_eq!(rt.point(2.0, 0.0), 3.0); // outline is color 3, not a stale default
    }

    // DRAW "N" no-advance modifier: the segment is drawn but the cursor must
    // return to where it started. `self.line()` advances the cursor internally,
    // so N has to RESTORE it (not just skip a re-advance). Car sprites draw
    // spurs with `ND2`; a drifting cursor leaves gaps that make PAINT flood.
    #[test]
    fn draw_n_modifier_does_not_advance_cursor() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        rt.color(8.0, Some(1.0)); // CGA foreground = 3
        rt.draw("S4");        // scale 4 → 1 px/unit
        rt.draw("BM10,10");   // cursor at (10,10)
        rt.draw("ND4");       // draw a spur down 4, but do NOT advance
        assert_eq!((rt.cur_x(), rt.cur_y()), (10.0, 10.0)); // cursor preserved
        assert_eq!(rt.point(10.0, 12.0), 3.0);              // …yet the spur was drawn
        // A following move continues from the preserved origin, not the spur end.
        rt.draw("R3");
        assert_eq!((rt.cur_x(), rt.cur_y()), (13.0, 10.0));
    }

    // CGA mode 1 uses a 2-bit mask: PRESET of color 1 → !1 & 3 = 2.
    // (CGA sprite layout: data[0]=width*2, data[1]=height, then 2-bpp bytes.
    // Two pixels colors 1,2 in one byte = (1<<6)|(2<<4) = 0x60.)
    #[test]
    fn preset_uses_cga_mask_in_mode1() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        let spr = vec![4.0, 1.0, 0x60 as f64]; // 2×1 CGA sprite: pixels 1, 2
        rt.put_sprite(&spr, 0.0, 0.0, PutAction::Preset);
        assert_eq!(rt.point(0.0, 0.0), 2.0); // !1 & 3
        assert_eq!(rt.point(1.0, 0.0), 1.0); // !2 & 3
    }

    // Two distinct sprites packed into ONE array at element offsets 0 and N
    // (QB's `GET …, Arr(n)` / `PUT …, Arr(n)` — the qblocks BlockImage idiom).
    // Each must round-trip from its own offset without clobbering the other.
    #[test]
    fn get_put_at_offset_packs_independent_sprites() {
        let mut rt = setup();
        // Sprite A at (0,0): pixels colors 1, 2.  Sprite B at (0,2): colors 4, 8.
        rt.pset(0.0, 0.0, 1.0); rt.pset(1.0, 0.0, 2.0);
        rt.pset(0.0, 2.0, 4.0); rt.pset(1.0, 2.0, 8.0);

        let mut buf: Vec<f64> = Vec::new();
        const N: usize = 64; // well past sprite A's footprint
        rt.get_sprite_at(0.0, 0.0, 1.0, 0.0, &mut buf, 0); // A → offset 0
        rt.get_sprite_at(0.0, 2.0, 1.0, 2.0, &mut buf, N); // B → offset N

        // Packing B at N must not have shrunk/clobbered A at 0.
        assert!(buf.len() >= N + 2, "buffer must grow to hold the offset sprite");
        assert_eq!(buf[0], 1.0, "sprite A header survives");

        // Wipe and blit each back from its own offset.
        rt.screen(9.0); // clears the framebuffer
        rt.put_sprite_at(&buf, 10.0, 10.0, PutAction::Pset, 0); // A
        rt.put_sprite_at(&buf, 10.0, 12.0, PutAction::Pset, N); // B
        assert_eq!(rt.point(10.0, 10.0), 1.0);
        assert_eq!(rt.point(11.0, 10.0), 2.0);
        assert_eq!(rt.point(10.0, 12.0), 4.0);
        assert_eq!(rt.point(11.0, 12.0), 8.0);
    }

    // get_sprite_at with offset > 0 is grow-only: a pre-sized buffer must NOT shrink.
    #[test]
    fn get_at_offset_does_not_shrink_buffer() {
        let mut rt = setup();
        rt.pset(0.0, 0.0, 5.0); rt.pset(1.0, 0.0, 6.0);
        let mut buf: Vec<f64> = vec![0.0; 4096]; // pre-DIM'd large, like qblocks BlockImage
        rt.get_sprite_at(0.0, 0.0, 1.0, 0.0, &mut buf, 100);
        assert_eq!(buf.len(), 4096, "grow-only resize must never shrink the array");
        rt.screen(9.0);
        rt.put_sprite_at(&buf, 0.0, 0.0, PutAction::Pset, 100);
        assert_eq!(rt.point(0.0, 0.0), 5.0);
        assert_eq!(rt.point(1.0, 0.0), 6.0);
    }
}

#[cfg(test)]
mod cga_sprite_tests {
    use super::*;

    // donkey.bas hand-builds B%: a 1-px-wide × 193-tall white strip.
    // data[0]=2 (width 1 × 2bpp), data[1]=193, data = 0xC0C0 (each byte 0xC0 →
    // pixel (0xC0>>6)&3 = 3 = white). This is the road-dash scroll sprite.
    fn b_strip() -> Vec<f64> {
        let mut v = vec![2.0, 193.0];
        v.extend(std::iter::repeat((-16192i64) as f64).take(120)); // 0xC0C0 fill
        v
    }

    #[test]
    fn hand_built_b_strip_is_a_1px_white_column() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        rt.put_sprite(&b_strip(), 140.0, 6.0, PutAction::Pset);
        // The column at x=140 is white for the strip's height…
        for y in 6..6 + 193 {
            assert_eq!(rt.point(140.0, y as f64), 3.0, "y={y}");
        }
        // …and exactly one pixel wide.
        assert_eq!(rt.point(141.0, 6.0), 0.0);
        assert_eq!(rt.point(139.0, 6.0), 0.0);
    }

    #[test]
    fn b_strip_xor_draws_and_erases() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        rt.put_sprite(&b_strip(), 140.0, 6.0, PutAction::Xor); // draw
        assert_eq!(rt.point(140.0, 50.0), 3.0);
        rt.put_sprite(&b_strip(), 140.0, 6.0, PutAction::Xor); // erase (toggle back)
        assert_eq!(rt.point(140.0, 50.0), 0.0);
    }

    // GET then PUT must round-trip a CGA region (the CAR%/DNK% path).
    #[test]
    fn cga_get_put_round_trip() {
        let mut rt = Runtime::headless();
        rt.screen(1.0);
        // Paint a small known 2-bpp pattern (colors 0..3) across a 6×3 region.
        let pat = [[1u8,2,3,0,2,1],[3,3,0,1,2,2],[0,1,2,3,1,0]];
        for (y, row) in pat.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                rt.pset((10 + x) as f64, (20 + y) as f64, c as f64);
            }
        }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(10.0, 20.0, 15.0, 22.0, &mut spr);
        assert_eq!(spr[0], 12.0); // width 6 × 2
        assert_eq!(spr[1], 3.0);  // height 3
        // Clear and blit it back at a new origin.
        rt.cls(0);
        rt.put_sprite(&spr, 100.0, 40.0, PutAction::Pset);
        for (y, row) in pat.iter().enumerate() {
            for (x, &c) in row.iter().enumerate() {
                assert_eq!(rt.point((100 + x) as f64, (40 + y) as f64), c as f64,
                           "pixel ({x},{y})");
            }
        }
    }
}

#[cfg(test)]
mod pace_tests {
    use super::*;

    // REM QBC PACE n maps to a per-frame sleep interval of 1000/n ms; 0 disables.
    #[test]
    fn set_pace_maps_fps_to_interval() {
        let mut rt = Runtime::headless();
        rt.set_pace(20.0);
        assert_eq!(rt.pace_ms, 50);
        rt.set_pace(30.0);
        assert_eq!(rt.pace_ms, 33);
        rt.set_pace(0.0);
        assert_eq!(rt.pace_ms, 0); // disabled
    }

    // Paced auto_present must actually block (sleep) so the draw is watchable;
    // the default throttle never blocks. Drive enough psets to cross the gate.
    #[test]
    fn pace_blocks_but_default_does_not() {
        let mut rt = Runtime::headless();
        rt.screen(9.0);
        rt.set_pace(50.0); // 20ms interval
        let t0 = std::time::Instant::now();
        for i in 0..200 { rt.pset((i % 100) as f64, 0.0, 1.0); } // > 64-call gate
        assert!(t0.elapsed() >= std::time::Duration::from_millis(15),
                "paced psets should sleep, took {:?}", t0.elapsed());

        let mut rt2 = Runtime::headless();
        rt2.screen(9.0); // no pace set
        let t1 = std::time::Instant::now();
        for i in 0..200 { rt2.pset((i % 100) as f64, 0.0, 1.0); }
        assert!(t1.elapsed() < std::time::Duration::from_millis(15),
                "default throttle must not block, took {:?}", t1.elapsed());
    }
}

#[cfg(test)]
mod headless_tests {
    use super::*;

    // export_ppm writes a valid P6 header + RGB pixel bytes at native resolution.
    #[test]
    fn export_ppm_round_trips() {
        let mut rt = Runtime::headless();
        rt.screen(13.0); // 320x200, known VGA palette
        rt.pset(0.0, 0.0, 1.0);
        let path = std::env::temp_dir().join(format!("qbc_ppm_{}.ppm", std::process::id()));
        rt.export_ppm(path.to_str().unwrap()).unwrap();
        let bytes = std::fs::read(&path).unwrap();
        let header = format!("P6\n{} {}\n255\n", rt.width, rt.height);
        assert!(bytes.starts_with(header.as_bytes()), "bad PPM header");
        assert_eq!(bytes.len(), header.len() + (rt.width * rt.height) as usize * 3);
        // Pixel (0,0) is color index 1; its RGB must be the first pixel triple.
        let (r, g, b) = rt.palette_rgb[1];
        let off = header.len();
        assert_eq!((bytes[off], bytes[off + 1], bytes[off + 2]), (r, g, b));
        let _ = std::fs::remove_file(&path);
    }

    // fb_checksum is stable across calls and changes when a pixel changes.
    #[test]
    fn fb_checksum_stable_and_sensitive() {
        let mut rt = Runtime::headless();
        rt.screen(9.0);
        let a = rt.fb_checksum();
        assert_eq!(a, rt.fb_checksum(), "checksum must be stable");
        rt.pset(10.0, 10.0, 5.0);
        assert_ne!(a, rt.fb_checksum(), "checksum must change with the framebuffer");
    }

    // inject_key feeds scripted keys that inkey() returns in order, then "".
    #[test]
    fn inject_key_feeds_inkey_in_order() {
        let mut rt = Runtime::headless();
        rt.inject_key("a");
        rt.inject_key(&normalize_key("DOWN"));
        rt.inject_key(&normalize_key("ENTER"));
        assert_eq!(rt.inkey(), "a");
        assert_eq!(rt.inkey(), "\u{0}P"); // DOWN scan code (ASC of last byte = 80)
        assert_eq!(rt.inkey(), "\r");
        assert_eq!(rt.inkey(), ""); // queue drained
    }

    // DRAIN sentinel: inkey() returns "" when it pops the \x00 sentinel injected
    // by normalize_key("DRAIN"), allowing WHILE INKEY$<>"":WEND drain-loops to
    // exit while leaving subsequent scripted keys intact.
    #[test]
    fn drain_sentinel_stops_drain_loop_without_consuming_later_keys() {
        let mut rt = Runtime::headless();
        rt.inject_key(&normalize_key("DRAIN")); // \x00 sentinel
        rt.inject_key(&normalize_key("ENTER")); // \r — next real key
        assert_eq!(rt.inkey(), "");  // DRAIN → "" (drain-loop exits)
        assert_eq!(rt.inkey(), "\r"); // ENTER is still in the queue
        assert_eq!(rt.inkey(), "");  // queue now truly empty
    }

    // normalize_key maps BARRIER as an alias for DRAIN.
    #[test]
    fn barrier_is_alias_for_drain() {
        assert_eq!(normalize_key("DRAIN"), normalize_key("BARRIER"));
        let mut rt = Runtime::headless();
        rt.inject_key(&normalize_key("BARRIER"));
        assert_eq!(rt.inkey(), ""); // sentinel returns ""
    }

    // normalize_key matches the windowed minifb_key_to_qb mapping exactly, so a
    // scripted run behaves identically to real keypresses.
    #[test]
    fn normalize_key_matches_real_input() {
        assert_eq!(normalize_key("UP"), minifb_key_to_qb(Key::Up));
        assert_eq!(normalize_key("DOWN"), minifb_key_to_qb(Key::Down));
        assert_eq!(normalize_key("LEFT"), minifb_key_to_qb(Key::Left));
        assert_eq!(normalize_key("RIGHT"), minifb_key_to_qb(Key::Right));
        assert_eq!(normalize_key("ENTER"), minifb_key_to_qb(Key::Enter));
        assert_eq!(normalize_key("Q"), minifb_key_to_qb(Key::Q)); // lowercased "q"
        assert_eq!(normalize_key("5"), minifb_key_to_qb(Key::Key5));
    }
}

#[cfg(test)]
mod mode13_sprite_tests {
    use super::*;

    // GET→PUT round-trips an 8-bit (256-color) sprite pixel-exact. The old EGA
    // planar path truncated colors to `& 15` and mispacked the layout.
    #[test]
    fn mode13_sprite_round_trips_8bit_color() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        let (w, h) = (6usize, 4usize);
        // Pattern includes colors > 15 (e.g. up to ~207) that 4-bit would lose.
        for y in 0..h {
            for x in 0..w {
                rt.pset((5 + x) as f64, (5 + y) as f64, ((x * 40 + y * 7) % 256) as f64);
            }
        }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(5.0, 5.0, (5 + w - 1) as f64, (5 + h - 1) as f64, &mut spr);
        // Header: word0 = width*8 (bits), word1 = height; 2-word header + data.
        assert_eq!(spr[0], (w * 8) as f64);
        assert_eq!(spr[1], h as f64);
        assert_eq!(spr.len(), 2 + (w * h + 1) / 2);

        rt.put_sprite(&spr, 50.0, 50.0, PutAction::Pset);
        let mut saw_high = false;
        for y in 0..h {
            for x in 0..w {
                let src = rt.point((5 + x) as f64, (5 + y) as f64);
                let dst = rt.point((50 + x) as f64, (50 + y) as f64);
                assert_eq!(src, dst, "pixel ({x},{y}) blit mismatch");
                if src > 15.0 { saw_high = true; }
            }
        }
        assert!(saw_high, "test must include a color > 15 to be meaningful");
    }

    // PUT … XOR twice at the same spot restores whatever was underneath.
    #[test]
    fn mode13_put_xor_is_self_inverse() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        // Non-blank background under the blit target.
        for y in 0..8 {
            for x in 0..8 {
                rt.pset((30 + x) as f64, (30 + y) as f64, ((x + y * 8) % 256) as f64);
            }
        }
        // A separate 4x4 sprite.
        for y in 0..4 {
            for x in 0..4 {
                rt.pset(x as f64, y as f64, (100 + x + y) as f64);
            }
        }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 3.0, 3.0, &mut spr);

        let snapshot = |rt: &Runtime| -> Vec<u8> {
            let mut v = Vec::new();
            for y in 0..8u32 {
                for x in 0..8u32 {
                    v.push(rt.fb[((30 + y) * rt.width + (30 + x)) as usize]);
                }
            }
            v
        };
        let before = snapshot(&rt);
        rt.put_sprite(&spr, 30.0, 30.0, PutAction::Xor);
        rt.put_sprite(&spr, 30.0, 30.0, PutAction::Xor);
        assert_eq!(before, snapshot(&rt), "XOR twice must restore the background");
    }

    // PRESET inverts within the mode's 8-bit depth (color → 255 - color).
    #[test]
    fn mode13_put_preset_inverts_8bit() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        rt.pset(0.0, 0.0, 10.0);
        rt.pset(1.0, 0.0, 200.0);
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 1.0, 0.0, &mut spr);
        rt.put_sprite(&spr, 100.0, 0.0, PutAction::Preset);
        assert_eq!(rt.point(100.0, 0.0), 245.0); // 255 - 10
        assert_eq!(rt.point(101.0, 0.0), 55.0);  // 255 - 200
    }

    // Odd-width sprite (5×3): 15 bytes, no alignment padding issues.
    #[test]
    fn mode13_odd_width_sprite_round_trips() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        let colors: &[(f64, f64, f64)] = &[
            (0.0, 0.0, 0.0), (1.0, 0.0, 127.0), (2.0, 0.0, 128.0),
            (3.0, 0.0, 200.0), (4.0, 0.0, 255.0),
            (0.0, 1.0, 254.0), (1.0, 1.0, 1.0), (2.0, 1.0, 129.0),
            (3.0, 1.0, 201.0), (4.0, 1.0, 253.0),
            (0.0, 2.0, 2.0), (1.0, 2.0, 126.0), (2.0, 2.0, 130.0),
            (3.0, 2.0, 202.0), (4.0, 2.0, 252.0),
        ];
        for &(x, y, c) in colors { rt.pset(x, y, c); }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 4.0, 2.0, &mut spr);
        assert_eq!(spr[0], 40.0); // width=5, 5*8=40 bits
        assert_eq!(spr[1], 3.0);  // height=3
        // expected elements: 2 + ceil(15/2) = 2 + 8 = 10
        assert_eq!(spr.len(), 10);

        rt.put_sprite(&spr, 50.0, 50.0, PutAction::Pset);
        for &(x, y, c) in colors {
            let got = rt.point(50.0 + x, 50.0 + y);
            assert_eq!(got, c, "odd-width pixel ({x},{y}) mismatch: expected {c}, got {got}");
        }
    }

    // Colors 0, 128, 255 round-trip: these hit sign-extension boundaries in
    // the i16 packing used by get_sprite_mode13.
    #[test]
    fn mode13_boundary_colors_round_trip() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        // 4×1 sprite: colors 0, 128, 255, 200
        for (x, &c) in [0.0f64, 128.0, 255.0, 200.0].iter().enumerate() {
            rt.pset(x as f64, 0.0, c);
        }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 3.0, 0.0, &mut spr);
        rt.put_sprite(&spr, 100.0, 100.0, PutAction::Pset);
        for (x, &c) in [0.0f64, 128.0, 255.0, 200.0].iter().enumerate() {
            let got = rt.point(100.0 + x as f64, 100.0);
            assert_eq!(got, c, "boundary color pixel {x} mismatch: expected {c}, got {got}");
        }
    }

    // AND verb: result = fb & sprite. With a solid fb = 255 and sprite colors,
    // the AND leaves the sprite color (255 & c = c).
    #[test]
    fn mode13_put_and_verb() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        // Background = 0xFF = 255 everywhere in target region
        for y in 0..4 { for x in 0..4 { rt.pset((20+x) as f64, (20+y) as f64, 255.0); } }
        // Sprite = gradient
        for y in 0..4 { for x in 0..4 { rt.pset(x as f64, y as f64, (x*10+y*3) as f64); } }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 3.0, 3.0, &mut spr);
        rt.put_sprite(&spr, 20.0, 20.0, PutAction::And);
        for y in 0..4u32 { for x in 0..4u32 {
            let expected = x*10 + y*3; // 255 & sprite = sprite
            assert_eq!(rt.point((20+x) as f64, (20+y) as f64), expected as f64,
                       "AND pixel ({x},{y})");
        }}
    }

    // OR verb: result = fb | sprite. With fb = 0, OR gives the sprite color.
    #[test]
    fn mode13_put_or_verb() {
        let mut rt = Runtime::headless();
        rt.screen(13.0);
        // Target region already black (default), sprite = some colors
        for y in 0..4 { for x in 0..4 { rt.pset(x as f64, y as f64, (x*17+y*5) as f64); } }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 3.0, 3.0, &mut spr);
        rt.put_sprite(&spr, 30.0, 30.0, PutAction::Or);
        for y in 0..4u32 { for x in 0..4u32 {
            let expected = x*17 + y*5; // 0 | sprite = sprite
            assert_eq!(rt.point((30+x) as f64, (30+y) as f64), expected as f64,
                       "OR pixel ({x},{y})");
        }}
    }

    // Clipping: a sprite PUT partially off each screen edge must not panic and
    // must write only the visible pixels.
    #[test]
    fn mode13_put_clips_at_edges() {
        let mut rt = Runtime::headless();
        rt.screen(13.0); // 320×200
        for y in 0..4 { for x in 0..4 { rt.pset(x as f64, y as f64, 99.0); } }
        let mut spr: Vec<f64> = Vec::new();
        rt.get_sprite(0.0, 0.0, 3.0, 3.0, &mut spr);
        // Clip off left edge (-2,0): cols 2..3 should be written, cols 0..1 off-screen
        rt.put_sprite(&spr, -2.0, 50.0, PutAction::Pset);
        assert_eq!(rt.point(0.0, 50.0), 99.0, "clip-left visible");
        assert_eq!(rt.point(1.0, 50.0), 99.0, "clip-left visible");
        // Clip off right edge (318,0): cols 0..1 visible, cols 2..3 off-screen
        rt.put_sprite(&spr, 318.0, 60.0, PutAction::Pset);
        assert_eq!(rt.point(318.0, 60.0), 99.0, "clip-right visible");
        assert_eq!(rt.point(319.0, 60.0), 99.0, "clip-right visible");
        // Clip off top edge (0,-2): rows 2..3 visible
        rt.put_sprite(&spr, 100.0, -2.0, PutAction::Pset);
        assert_eq!(rt.point(100.0, 0.0), 99.0, "clip-top visible");
        // Clip off bottom edge (0,198): rows 0..1 visible
        rt.put_sprite(&spr, 100.0, 198.0, PutAction::Pset);
        assert_eq!(rt.point(100.0, 198.0), 99.0, "clip-bottom visible");
        assert_eq!(rt.point(100.0, 199.0), 99.0, "clip-bottom visible");
    }
}
