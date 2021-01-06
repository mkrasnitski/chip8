#![allow(non_snake_case)]

use anyhow::{bail, Context, Result};
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
    0xF0, 0x90, 0x90, 0x90, 0xF0, // 0
    0x20, 0x60, 0x20, 0x20, 0x70, // 1
    0xF0, 0x10, 0xF0, 0x80, 0xF0, // 2
    0xF0, 0x10, 0xF0, 0x10, 0xF0, // 3
    0x90, 0x90, 0xF0, 0x10, 0x10, // 4
    0xF0, 0x80, 0xF0, 0x10, 0xF0, // 5
    0xF0, 0x80, 0xF0, 0x90, 0xF0, // 6
    0xF0, 0x10, 0x20, 0x40, 0x40, // 7
    0xF0, 0x90, 0xF0, 0x90, 0xF0, // 8
    0xF0, 0x90, 0xF0, 0x10, 0xF0, // 9
    0xF0, 0x90, 0xF0, 0x90, 0x90, // A
    0xE0, 0x90, 0xE0, 0x90, 0xE0, // B
    0xF0, 0x80, 0x80, 0x80, 0xF0, // C
    0xE0, 0x90, 0x90, 0x90, 0xE0, // D
    0xF0, 0x80, 0xF0, 0x80, 0xF0, // E
    0xF0, 0x80, 0xF0, 0x80, 0x80, // F
];
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
    pub fn new(loc: &str) -> Result<Self> {
        let path = Path::new(loc);
        let binary = fs::read(&path)
            .with_context(|| format!("Couldn't read file `{}`", path.to_str().unwrap()))?;
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

    pub fn run(&mut self) -> Result<()> {
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
            let instr = self.parse_instr(opcode)?;
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
        let (key, v) = self.display.poll_events()?;
        let key_val = self.get_key(&key[..])?;
        self.keyboard[key_val as usize] = v;
        Some((key_val, v))
    }

    fn get_key(&self, key: &str) -> Option<u8> {
        let val = match key {
            "1" => 1,
            "2" => 2,
            "3" => 3,
            "4" => 12,
            "Q" => 4,
            "W" => 5,
            "E" => 6,
            "R" => 13,
            "A" => 7,
            "S" => 8,
            "D" => 9,
            "F" => 14,
            "Z" => 10,
            "X" => 0,
            "C" => 11,
            "V" => 15,
            _ => return None,
        };
        Some(val)
    }

    fn get_state(&self) -> String {
        format!("{:x} {: >2} {} {:?}", self.I, self.DT, self.ST, self.V)
    }

    fn fetch_instr(&self, addr: u16) -> u16 {
        let addr = addr as usize;
        ((self.RAM[addr] as u16) << 8) + (self.RAM[addr + 1] as u16)
    }

    fn parse_instr(&self, instr: u16) -> Result<Instr> {
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
            [3, x, _, _] => SE(x, SEMode::Imm8(kk)),
            [4, x, _, _] => SNE(x, SEMode::Imm8(kk)),
            [5, x, y, 0] => SE(x, SEMode::Reg(y)),
            [6, x, _, _] => LD(x, LDMode::Imm8(kk)),
            [7, x, _, _] => ADD(x, ADDMode::Imm8(kk)),
            [8, x, y, 0] => LD(x, LDMode::Reg(y)),
            [8, x, y, 1] => OR(x, y),
            [8, x, y, 2] => AND(x, y),
            [8, x, y, 3] => XOR(x, y),
            [8, x, y, 4] => ADD(x, ADDMode::Reg(y)),
            [8, x, y, 5] => SUB(x, y),
            [8, x, _, 6] => SHR(x),
            [8, x, y, 7] => SUBN(x, y),
            [8, x, _, 0xE] => SHL(x),
            [9, x, y, 0] => SNE(x, SEMode::Reg(y)),
            [0xA, _, _, _] => LD(0, LDMode::Imm12(nnn)),
            [0xB, _, _, _] => JP(nnn, JPMode::Offset),
            [0xC, x, _, _] => RND(x, kk),
            [0xD, x, y, n] => DRW(x, y, n),
            [0xE, x, 9, 0xE] => SKP(x),
            [0xE, x, 0xA, 1] => SKNP(x),
            [0xF, x, 0, 7] => LD(x, LDMode::FromDT),
            [0xF, x, 0, 0xA] => LD(x, LDMode::K),
            [0xF, x, 1, 5] => LD(x, LDMode::DT),
            [0xF, x, 1, 8] => LD(x, LDMode::ST),
            [0xF, x, 1, 0xE] => ADD(x, ADDMode::ToI),
            [0xF, x, 2, 9] => LD(x, LDMode::F),
            [0xF, x, 3, 3] => LD(x, LDMode::B),
            [0xF, x, 5, 5] => LD(x, LDMode::ToI),
            [0xF, x, 6, 5] => LD(x, LDMode::FromI),
            _ => bail!("INVALID INSTRUCTION: {:04x}", instr),
        };
        Ok(parsed_instr)
    }

    fn run_instr(&mut self, instr: Instr) {
        let I = self.I as usize;
        match instr {
            // Arithmetic
            LD(x, mode) => match mode {
                LDMode::Imm8(kk) => self.V[x] = kk,
                LDMode::Imm12(nnn) => self.I = nnn,
                LDMode::Reg(y) => self.V[x] = self.V[y],
                LDMode::FromDT => self.V[x] = self.DT,
                LDMode::DT => self.DT = self.V[x],
                LDMode::ST => self.ST = self.V[x],
                LDMode::K => loop {
                    if let Some((key_val, true)) = self.poll_keyboard() {
                        self.V[x] = key_val as u8;
                        break;
                    }
                },
                LDMode::F => self.I = DIGITS_LOC + 5 * self.V[x] as u16,
                LDMode::B => {
                    let B = [self.V[x] / 100, (self.V[x] % 100) / 10, self.V[x] % 10];
                    self.RAM[I..I + 3].copy_from_slice(&B);
                }
                LDMode::ToI => self.RAM[I..I + x + 1].copy_from_slice(&self.V[..x + 1]),
                LDMode::FromI => self.V[..x + 1].copy_from_slice(&self.RAM[I..I + x + 1]),
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
            SE(x, SEMode::Imm8(kk)) => self.skip(self.V[x] == kk),
            SE(x, SEMode::Reg(y)) => self.skip(self.V[x] == self.V[y]),
            SNE(x, SEMode::Imm8(kk)) => self.skip(self.V[x] != kk),
            SNE(x, SEMode::Reg(y)) => self.skip(self.V[x] != self.V[y]),

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
