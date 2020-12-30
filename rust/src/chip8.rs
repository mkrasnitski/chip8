#![allow(non_snake_case, unused_variables, dead_code)]

use path_dedot::*;
use rand::Rng;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::display::Display;

pub const CHIP8_WIDTH: usize = 64;
pub const CHIP8_HEIGHT: usize = 32;
const DIGITS: [u8; 80] = [
    0xF0, 0x90, 0x90, 0x90, 0xF0, 0x20, 0x60, 0x20, 0x20, 0x70, 0xF0, 0x10, 0xF0, 0x80, 0xF0, 0xF0,
    0x10, 0xF0, 0x10, 0xF0, 0x90, 0x90, 0xF0, 0x10, 0x10, 0xF0, 0x80, 0xF0, 0x10, 0xF0, 0xF0, 0x80,
    0xF0, 0x90, 0xF0, 0xF0, 0x10, 0x20, 0x40, 0x40, 0xF0, 0x90, 0xF0, 0x90, 0xF0, 0xF0, 0x90, 0xF0,
    0x10, 0xF0, 0xF0, 0x90, 0xF0, 0x90, 0x90, 0xE0, 0x90, 0xE0, 0x90, 0xE0, 0xF0, 0x80, 0x80, 0x80,
    0xF0, 0xE0, 0x90, 0x90, 0x90, 0xE0, 0xF0, 0x80, 0xF0, 0x80, 0xF0, 0xF0, 0x80, 0xF0, 0x80, 0x80,
];
const KEYS: [&str; 16] = [
    "1", "2", "3", "4", "Q", "W", "E", "R", "A", "S", "D", "F", "Z", "X", "C", "V",
];
const KEY_VALS: [u8; 16] = [1, 2, 3, 12, 4, 5, 6, 13, 7, 8, 9, 14, 10, 0, 11, 15];
const DIGITS_LOC: u16 = 0;
const CLOCK_HZ: u64 = 1000;
const LIMIT_FREQ: bool = true;
const DEBUG: bool = false;

pub struct Chip8 {
    start: u16,
    rng: rand::rngs::ThreadRng,

    RAM: [u8; 0x1000],
    V: [u8; 0x10],
    stack: [u16; 0x10],
    PC: u16,
    I: u16,
    SP: i8,
    DT: u8,
    ST: u8,

    keyboard: [bool; 16],
    screen: [[u8; CHIP8_WIDTH]; CHIP8_HEIGHT],
    display: Display,
}

impl Chip8 {
    pub fn new(loc: &str) -> Result<Self, String> {
        let path = Path::new(loc).parse_dot().unwrap();
        let binary = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(_) => return Err(format!("Unable to read file {}", path.to_str().unwrap())),
        };

        let mut c = Chip8 {
            start: 0x200,
            rng: rand::thread_rng(),

            RAM: [0; 0x1000],
            V: [0; 0x10],
            stack: [0; 0x10],
            I: 0,
            SP: -1,
            PC: 0,
            DT: 0,
            ST: 0,

            keyboard: [false; 16],
            screen: [[0; CHIP8_WIDTH]; CHIP8_HEIGHT],
            display: Display::new(),
        };
        let s = c.start as usize;
        let digits_offset = DIGITS_LOC as usize;
        c.RAM[digits_offset..digits_offset + 80].clone_from_slice(&DIGITS);
        c.RAM[s..s + binary.len()].clone_from_slice(&binary);
        Ok(c)
    }

    pub fn run(&mut self) {
        self.PC = self.start;
        let pc_modifying = ["JP", "CALL", "RET"];
        let mut timer = Instant::now();
        let frametime = match LIMIT_FREQ {
            true => Some(Duration::from_nanos(1000000000 / CLOCK_HZ)),
            false => None,
        };

        loop {
            let start_time = Instant::now();

            // Fetch the next two bytes from RAM, and queue up what
            // instruction to run next. Based on the name of the instruction,
            // if the instr modifies the PC directly, don't auto-increment it.
            // Then, run the instruction and invoke a draw call.
            let instr = self.fetch_instr(self.PC);
            let s = self.instr_name(instr);
            if DEBUG {
                println!(
                    "{:04x} {:04x} {: <13} {}",
                    self.PC,
                    instr,
                    s,
                    self.get_state()
                );
            }
            let name: &str = s.split(' ').collect::<Vec<&str>>()[0];
            if !pc_modifying.contains(&name) {
                self.PC += 2
            }
            self.poll_keyboard();
            self.run_instr(instr);
            self.display.draw(&self.screen);

            // If enough time has passed, invoke a decrement of DT and ST.
            // These should decrement at 60Hz if they have values > 0.
            if timer.elapsed() > Duration::from_millis(1000 / 60) {
                if self.DT > 0 {
                    self.DT -= 1;
                }
                if self.ST > 0 {
                    self.ST -= 1;
                }
                timer = Instant::now();
            }

            // If LIMIT_FREQ was set, this will be Some(). This will then sleep
            // so that the total time for the loop is equal to `frametime`.
            if let Some(total) = frametime {
                let elapsed = start_time.elapsed();
                if elapsed < total {
                    std::thread::sleep(total - elapsed);
                } else if DEBUG {
                    println!("Frame time: {:?} > {:?}", elapsed, total);
                }
            }
        }
    }

    fn push(&mut self, val: u16) {
        self.SP += 1;
        self.stack[self.SP as usize] = val;
    }

    fn pop(&mut self) -> u16 {
        let res = self.stack[self.SP as usize];
        self.SP -= 1;
        res
    }

    // Poll the display's event pump, and if we find a keyboard event,
    // and the key maps to a valid emulated keyboard key, return the
    // keyboard value corresponding to it, otherwise return None
    fn poll_keyboard(&mut self) -> Option<(u8, bool)> {
        match self.display.poll_events() {
            Some((key, v)) => match KEYS.iter().position(|&s| s == key) {
                Some(index) => {
                    let key_val = KEY_VALS[index];
                    self.keyboard[key_val as usize] = v;
                    Some((key_val, v))
                }
                None => None,
            },
            None => None,
        }
    }

    fn get_state(&self) -> String {
        format!("{:x} {: >2} {} {:?}", self.I, self.DT, self.ST, self.V)
    }

    fn fetch_instr(&self, addr: u16) -> u16 {
        let addr = addr as usize;
        ((self.RAM[addr] as u16) << 8) + (self.RAM[addr + 1] as u16)
    }

    fn instr_name(&self, instr: u16) -> &str {
        let nibbles: [usize; 4] = [
            (instr >> 12).into(),
            ((instr >> 8) & 0xF).into(),
            ((instr >> 4) & 0xF).into(),
            (instr & 0xF).into(),
        ];
        match nibbles {
            [0, 0, 0xE, 0] => "CLS",
            [0, 0, 0xE, 0xE] => "RET",
            [1, _, _, _] => "JP nnn",
            [2, _, _, _] => "CALL nnn",
            [3, x, _, _] => "SE Vx, kk",
            [4, x, _, _] => "SNE Vx, kk",
            [5, x, y, 0] => "SE Vx, Vy",
            [6, x, _, _] => "LD Vx, kk",
            [7, x, _, _] => "ADD Vx, kk",
            [8, x, y, 0] => "LD Vx, Vy",
            [8, x, y, 1] => "OR Vx, Vy",
            [8, x, y, 2] => "AND Vx, Vy",
            [8, x, y, 3] => "XOR Vx, Vy",
            [8, x, y, 4] => "ADD Vx, Vy",
            [8, x, y, 5] => "SUB Vx, Vy",
            [8, x, y, 6] => "SHR Vx {, Vy}",
            [8, x, y, 7] => "SUBN Vx, Vy",
            [8, x, y, 0xE] => "SHL Vx {, Vy}",
            [9, x, y, 0] => "SNE Vx, Vy",
            [0xA, _, _, _] => "LD I, nnn",
            [0xB, _, _, _] => "JP V0, nnn",
            [0xC, x, _, _] => "RND Vx, kk",
            [0xD, x, y, n] => "DRW Vx, Vy, n",
            [0xE, x, 9, 0xE] => "SKP Vx",
            [0xE, x, 0xA, 1] => "SKNP Vx",
            [0xF, x, 0, 7] => "LD Vx, DT",
            [0xF, x, 0, 0xA] => "LD Vx, K",
            [0xF, x, 1, 5] => "LD DT, Vx",
            [0xF, x, 1, 8] => "LD ST, Vx",
            [0xF, x, 1, 0xE] => "ADD I, Vx",
            [0xF, x, 2, 9] => "LD F, Vx",
            [0xF, x, 3, 3] => "LD B, Vx",
            [0xF, x, 5, 5] => "LD [I], Vx",
            [0xF, x, 6, 5] => "LD Vx, [I]",
            _ => panic!("INVALID INSTRUCTION: {:04x}", instr),
        }
    }

    fn run_instr(&mut self, instr: u16) {
        let nibbles: [usize; 4] = [
            ((instr & 0xF000) >> 12).into(),
            ((instr & 0x0F00) >> 8).into(),
            ((instr & 0x00F0) >> 4).into(),
            (instr & 0x000F).into(),
        ];
        let kk: u8 = instr as u8;
        let nnn: u16 = instr & 0x0FFF;
        match nibbles {
            [0, 0, 0xE, 0] => self.screen = [[0; CHIP8_WIDTH]; CHIP8_HEIGHT],
            [0, 0, 0xE, 0xE] => self.PC = self.pop(),
            [1, _, _, _] => self.PC = nnn,
            [2, _, _, _] => self.CALL(nnn),
            [3, x, _, _] => self.SKIP(self.V[x] == kk),
            [4, x, _, _] => self.SKIP(self.V[x] != kk),
            [5, x, y, 0] => self.SKIP(self.V[x] == self.V[y]),
            [6, x, _, _] => self.V[x] = kk,
            [7, x, _, _] => self.ADD(x, kk, false),
            [8, x, y, 0] => self.V[x] = self.V[y],
            [8, x, y, 1] => self.V[x] |= self.V[y],
            [8, x, y, 2] => self.V[x] &= self.V[y],
            [8, x, y, 3] => self.V[x] ^= self.V[y],
            [8, x, y, 4] => self.ADD(x, self.V[y], true),
            [8, x, y, 5] => self.SUB(x, self.V[y]),
            [8, x, y, 6] => self.SH(x, 'R'),
            [8, x, y, 7] => self.SUB(y, self.V[x]),
            [8, x, y, 0xE] => self.SH(x, 'L'),
            [9, x, y, 0] => self.SKIP(self.V[x] != self.V[y]),
            [0xA, _, _, _] => self.I = nnn,
            [0xB, _, _, _] => self.PC = nnn + self.V[0] as u16,
            [0xC, x, _, _] => self.V[x] = self.rng.gen::<u8>() & kk,
            [0xD, x, y, n] => self.DRW(x, y, n),
            [0xE, x, 9, 0xE] => self.SKIP(self.keyboard[self.V[x] as usize]),
            [0xE, x, 0xA, 1] => self.SKIP(!self.keyboard[self.V[x] as usize]),
            [0xF, x, 0, 7] => self.V[x] = self.DT,
            [0xF, x, 0, 0xA] => self.LD(x, "K"),
            [0xF, x, 1, 5] => self.DT = self.V[x],
            [0xF, x, 1, 8] => self.ST = self.V[x],
            [0xF, x, 1, 0xE] => self.I = self.I.wrapping_add(self.V[x] as u16),
            [0xF, x, 2, 9] => self.LD(x, "F"),
            [0xF, x, 3, 3] => self.LD(x, "B"),
            [0xF, x, 5, 5] => self.LD(x, "->[I]"),
            [0xF, x, 6, 5] => self.LD(x, "<-[I]"),
            _ => (),
        };
    }

    fn LD(&mut self, x: usize, mode: &str) {
        let I = self.I as usize;
        match mode {
            "K" => loop {
                if let Some((key_val, true)) = self.poll_keyboard() {
                    self.V[x] = key_val as u8;
                    break;
                }
            },
            "F" => self.I = DIGITS_LOC + 5 * self.V[x] as u16,
            "B" => {
                let B = [self.V[x] / 100, (self.V[x] % 100) / 10, self.V[x] % 10];
                self.RAM[I..I + 3].copy_from_slice(&B);
            }
            "->[I]" => self.RAM[I..I + x + 1].copy_from_slice(&self.V[..x + 1]),
            "<-[I]" => self.V[..x + 1].copy_from_slice(&self.RAM[I..I + x + 1]),
            _ => panic!("Incorrect LD mode."),
        }
    }

    // Add src_val to V[dest_reg], and set VF if an overflow occurs
    fn ADD(&mut self, dest_reg: usize, src_val: u8, overflow_check: bool) {
        let (res, overflow) = self.V[dest_reg].overflowing_add(src_val);
        self.V[dest_reg] = res;
        if overflow_check {
            self.V[0xF] = overflow as u8;
        }
    }

    // Subtract src_val from V[dest_reg], and set VF if NO BORROW occurs
    fn SUB(&mut self, dest_reg: usize, src_val: u8) {
        let (res, borrow) = self.V[dest_reg].overflowing_sub(src_val);
        self.V[dest_reg] = res;
        self.V[0xF] = !borrow as u8;
    }

    // Shift the register left or right, and set VF if any set bits get
    // shifted out of range.
    fn SH(&mut self, reg: usize, direction: char) {
        let (res, mask) = match direction {
            'R' => (self.V[reg] >> 1, 1),
            'L' => (self.V[reg] << 1, 1 << 7),
            _ => panic!("Incorrect SH direction."),
        };
        self.V[0xF] = (self.V[reg] & mask != 0) as u8;
        self.V[reg] = res;
    }

    fn CALL(&mut self, nnn: u16) {
        self.push(self.PC + 2);
        self.PC = nnn;
    }

    fn SKIP(&mut self, expr: bool) {
        if expr {
            self.PC += 2;
        }
    }

    // Draw an 8xN Sprite at the location (Vx, Vy) on the screen by XORing
    // the screen with the sprite. Set VF if any set pixels on the screen
    // are erased during this process. Any pixels that would be drawn out
    // of bounds are wrapped around to the other side of the screen.
    fn DRW(&mut self, x: usize, y: usize, n: usize) {
        self.V[0xF] = 0;
        for j in 0..n {
            let y = (self.V[y] as usize + j) % CHIP8_HEIGHT;
            let val = self.RAM[self.I as usize + j];
            for i in 0..8 {
                let x = (self.V[x] as usize + i) % CHIP8_WIDTH;
                let bit = (val >> (7 - i)) & 1;
                self.V[0xF] |= bit & self.screen[y][x];
                self.screen[y][x] ^= bit;
            }
        }
    }
}
