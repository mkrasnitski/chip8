#![allow(unused_variables)]
use Instr::*;

pub enum Instr {
    // Arithmetic
    LD(usize, LDMode),
    ADD(usize, ADDMode),
    SUB(usize, usize),
    SUBN(usize, usize),
    OR(usize, usize),
    AND(usize, usize),
    XOR(usize, usize),
    SHL(usize),
    SHR(usize),
    RND(usize, u8),

    // Control Flow
    SKP(usize),
    SKNP(usize),
    SE(usize, SEMode),
    SNE(usize, SEMode),
    JP(u16, JPMode),
    CALL(u16),
    RET,

    // Drawing
    DRW(usize, usize, usize),
    CLS,
}

pub enum LDMode {
    Imm8(u8),
    Imm12(u16), // This one doesn't use the destination register unfortunately
    Reg(usize),
    FromDT,
    DT,
    ST,
    K,
    F,
    B,
    ToI,
    FromI,
}

pub enum ADDMode {
    Imm8(u8),
    ToI,
    Reg(usize),
}

pub enum SEMode {
    Imm8(u8),
    Reg(usize),
}

pub enum JPMode {
    NoOffset,
    Offset,
}

pub fn instr_name(instr: &Instr) -> &str {
    match instr {
        CLS => "CLS",                              // 00E0
        RET => "RET",                              // 00EE
        JP(nnn, JPMode::NoOffset) => "JP nnn",     // 1nnn
        CALL(nnn) => "CALL nnn",                   // 2nnn
        SNE(x, SEMode::Imm8(kk)) => "SNE Vx, kk",  // 3xkk
        SE(x, SEMode::Imm8(kk)) => "SE Vx, kk",    // 4xkk
        SE(x, SEMode::Reg(y)) => "SE Vx, Vy",      // 5xy0
        LD(x, LDMode::Imm8(kk)) => "LD Vx, kk",    // 6xkk
        ADD(x, ADDMode::Imm8(kk)) => "ADD Vx, kk", // 7xkk
        LD(x, LDMode::Reg(y)) => "LD Vx, Vy",      // 8xy0
        OR(x, y) => "OR Vx, Vy",                   // 8xy1
        AND(x, y) => "AND Vx, Vy",                 // 8xy2
        XOR(x, y) => "XOR Vx, Vy",                 // 8xy3
        ADD(x, ADDMode::Reg(y)) => "ADD Vx, Vy",   // 8xy4
        SUB(x, y) => "SUB Vx, Vy",                 // 8xy5
        SHR(x) => "SHR Vx",                        // 8xy6
        SUBN(x, y) => "SUB Vy, Vx",                // 8xy7
        SHL(x) => "SHL Vx",                        // 8xyE
        SNE(x, SEMode::Reg(y)) => "SNE Vx, Vy",    // 9xy0
        LD(_, LDMode::Imm12(nnn)) => "LD I, nnn",  // Annn
        JP(nnn, JPMode::Offset) => "JP V0, nnn",   // Bnnn
        RND(x, kk) => "RND Vx, kk",                // Cxkk
        DRW(x, y, n) => "DRW Vx, Vy, n",           // Dxyn
        SKP(x) => "SKP Vx",                        // Ex9E
        SKNP(x) => "SKNP Vx",                      // ExA1
        LD(x, LDMode::FromDT) => "LD Vx, DT",      // Fx07
        LD(x, LDMode::K) => "LD Vx, K",            // Fx0A
        LD(x, LDMode::DT) => "LD DT, Vx",          // Fx15
        LD(x, LDMode::ST) => "LD ST, Vx",          // Fx18
        ADD(x, ADDMode::ToI) => "ADD I, Vx",       // Fx1E
        LD(x, LDMode::F) => "LD Vx, F",            // Fx29
        LD(x, LDMode::B) => "LD Vx, B",            // Fx33
        LD(x, LDMode::ToI) => "LD [I], Vx",        // Fx55
        LD(x, LDMode::FromI) => "LD Vx, [I]",      // Fx65
    }
}
