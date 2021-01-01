#![allow(non_snake_case)]

use rand::Rng;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::display::Display;
use crate::enums::Instr::*;
use crate::enums::*;

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
    screen: [[bool; CHIP8_WIDTH]; CHIP8_HEIGHT],
    display: Display,
}

impl Chip8 {
    pub fn new(loc: &str) -> Result<Self, String> {
        let path = Path::new(loc);
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
            screen: [[false; CHIP8_WIDTH]; CHIP8_HEIGHT],
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
            let opcode = self.fetch_instr(self.PC);
            let instr = match self.parse_instr(opcode) {
                Ok(instr) => instr,
                Err(e) => {
                    println!("{}", e);
                    break;
                }
            };
            if DEBUG {
                println!(
                    "{:04x} {:04x} {: <13} | {}",
                    self.PC,
                    opcode,
                    instr_name(&instr),
                    self.get_state()
                );
            }
            match instr {
                JP(_, _) | CALL(_) | RET => (),
                _ => self.PC += 2,
            };
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

    fn parse_instr(&self, instr: u16) -> Result<Instr, String> {
        let nibbles: [usize; 4] = [
            (instr >> 12).into(),
            ((instr >> 8) & 0xF).into(),
            ((instr >> 4) & 0xF).into(),
            (instr & 0xF).into(),
        ];
        let kk: u8 = instr as u8;
        let nnn: u16 = instr & 0x0FFF;
        let parsed_instr = match nibbles {
            [0, 0, 0xE, 0] => CLS,
            [0, 0, 0xE, 0xE] => RET,
            [1, _, _, _] => JP(nnn, JPMode::NoOffset),
            [2, _, _, _] => CALL(nnn),
            [3, x, _, _] => SE(SEMode::Imm8(x, kk)),
            [4, x, _, _] => SNE(SEMode::Imm8(x, kk)),
            [5, x, y, 0] => SE(SEMode::Reg(x, y)),
            [6, x, _, _] => LD(LDMode::Imm8(x, kk)),
            [7, x, _, _] => ADD(x, ADDMode::Imm8(kk)),
            [8, x, y, 0] => LD(LDMode::Reg(x, y)),
            [8, x, y, 1] => OR(x, y),
            [8, x, y, 2] => AND(x, y),
            [8, x, y, 3] => XOR(x, y),
            [8, x, y, 4] => ADD(x, ADDMode::Reg(y)),
            [8, x, y, 5] => SUB(x, y),
            [8, x, _, 6] => SHR(x),
            [8, x, y, 7] => SUBN(x, y),
            [8, x, _, 0xE] => SHL(x),
            [9, x, y, 0] => SNE(SEMode::Reg(x, y)),
            [0xA, _, _, _] => LD(LDMode::Imm12(nnn)),
            [0xB, _, _, _] => JP(nnn, JPMode::Offset),
            [0xC, x, _, _] => RND(x, kk),
            [0xD, x, y, n] => DRW(x, y, n),
            [0xE, x, 9, 0xE] => SKP(x),
            [0xE, x, 0xA, 1] => SKNP(x),
            [0xF, x, 0, 7] => LD(LDMode::FromDT(x)),
            [0xF, x, 0, 0xA] => LD(LDMode::K(x)),
            [0xF, x, 1, 5] => LD(LDMode::DT(x)),
            [0xF, x, 1, 8] => LD(LDMode::ST(x)),
            [0xF, x, 1, 0xE] => ADD(x, ADDMode::ToI),
            [0xF, x, 2, 9] => LD(LDMode::F(x)),
            [0xF, x, 3, 3] => LD(LDMode::B(x)),
            [0xF, x, 5, 5] => LD(LDMode::ToI(x)),
            [0xF, x, 6, 5] => LD(LDMode::FromI(x)),
            _ => return Err(format!("INVALID INSTRUCTION: {:04x}", instr)),
        };
        Ok(parsed_instr)
    }

    fn run_instr(&mut self, instr: Instr) {
        let I = self.I as usize;
        match instr {
            // Arithmetic
            LD(mode) => match mode {
                LDMode::Imm8(x, kk) => self.V[x] = kk,
                LDMode::Imm12(nnn) => self.I = nnn,
                LDMode::Reg(x, y) => self.V[x] = self.V[y],
                LDMode::FromDT(x) => self.V[x] = self.DT,
                LDMode::DT(x) => self.DT = self.V[x],
                LDMode::ST(x) => self.ST = self.V[x],
                LDMode::K(x) => loop {
                    if let Some((key_val, true)) = self.poll_keyboard() {
                        self.V[x] = key_val as u8;
                        break;
                    }
                },
                LDMode::F(x) => self.I = DIGITS_LOC + 5 * self.V[x] as u16,
                LDMode::B(x) => {
                    let B = [self.V[x] / 100, (self.V[x] % 100) / 10, self.V[x] % 10];
                    self.RAM[I..I + 3].copy_from_slice(&B);
                }
                LDMode::ToI(x) => self.RAM[I..I + x + 1].copy_from_slice(&self.V[..x + 1]),
                LDMode::FromI(x) => self.V[..x + 1].copy_from_slice(&self.RAM[I..I + x + 1]),
            },
            ADD(x, mode) => match mode {
                ADDMode::Imm8(kk) => self.V[x] = self.V[x].wrapping_add(kk),
                ADDMode::ToI => self.I = self.I.wrapping_add(self.V[x] as u16),
                ADDMode::Reg(y) => {
                    let (res, overflow) = self.V[x].overflowing_add(self.V[y]);
                    self.V[x] = res;
                    self.V[0xF] = overflow as u8;
                }
            },
            SUB(x, y) => self.sub(x, y),
            SUBN(x, y) => self.sub(y, x),
            OR(x, y) => self.V[x] |= self.V[y],
            AND(x, y) => self.V[x] &= self.V[y],
            XOR(x, y) => self.V[x] ^= self.V[y],
            SHR(x) => {
                self.V[0xF] = self.V[x] & 1;
                self.V[x] >>= 1;
            }
            SHL(x) => {
                self.V[0xF] = ((self.V[x] & 0x80) > 0) as u8;
                self.V[x] <<= 1
            }
            RND(x, kk) => self.V[x] = self.rng.gen::<u8>() & kk,

            // Control Flow
            RET => self.PC = self.pop(),
            JP(nnn, JPMode::NoOffset) => self.PC = nnn,
            JP(nnn, JPMode::Offset) => self.PC = nnn + self.V[0] as u16,
            CALL(nnn) => {
                self.push(self.PC + 2);
                self.PC = nnn;
            }
            SKP(x) => self.skip(self.keyboard[self.V[x] as usize]),
            SKNP(x) => self.skip(!self.keyboard[self.V[x] as usize]),
            SE(SEMode::Imm8(x, kk)) => self.skip(self.V[x] == kk),
            SE(SEMode::Reg(x, y)) => self.skip(self.V[x] == self.V[y]),
            SNE(SEMode::Imm8(x, kk)) => self.skip(self.V[x] != kk),
            SNE(SEMode::Reg(x, y)) => self.skip(self.V[x] != self.V[y]),

            // Drawing
            DRW(x, y, n) => self.draw(x, y, n),
            CLS => self.screen = [[false; CHIP8_WIDTH]; CHIP8_HEIGHT],
        }
    }

    // Subtract V[y] from V[x], and set VF if NO BORROW occurs
    fn sub(&mut self, x: usize, y: usize) {
        let (res, borrow) = self.V[x].overflowing_sub(self.V[y]);
        self.V[x] = res;
        self.V[0xF] = !borrow as u8;
    }

    fn skip(&mut self, expr: bool) {
        if expr {
            self.PC += 2;
        }
    }

    // Draw an 8xN Sprite at the location (Vx, Vy) on the screen by XORing
    // the screen with the sprite. Set VF if any set pixels on the screen
    // are erased during this process. Any pixels that would be drawn out
    // of bounds are wrapped around to the other side of the screen.
    fn draw(&mut self, x: usize, y: usize, n: usize) {
        self.V[0xF] = 0;
        for j in 0..n {
            let y = (self.V[y] as usize + j) % CHIP8_HEIGHT;
            let val = self.RAM[self.I as usize + j];
            for i in 0..8 {
                let x = (self.V[x] as usize + i) % CHIP8_WIDTH;
                let bit = ((val >> (7 - i)) & 1) != 0;
                self.V[0xF] |= (bit & self.screen[y][x]) as u8;
                self.screen[y][x] ^= bit;
            }
        }
    }
}
