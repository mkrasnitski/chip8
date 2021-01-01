pub enum Instr {
    LD(LDMode),
    ADD(ADDMode),
    SUB(usize, usize),
    SUBN(usize, usize),
    OR(usize, usize),
    AND(usize, usize),
    XOR(usize, usize),
    SHL(usize),
    SHR(usize),
    RND(usize, u8),

    SE(SEMode),
    SNE(SEMode),
    JP(JPMode),
    CALL(u16),
    RET,

    SKP(usize),
    SKNP(usize),

    CLS,
    DRW(usize, usize, usize),
}

pub enum LDMode {
    Imm8(usize, u8),
    Imm12(u16),
    Reg(usize, usize),
    FromDT(usize),
    DT(usize),
    ST(usize),
    K(usize),
    F(usize),
    B(usize),
    ToI(usize),
    FromI(usize),
}

pub enum ADDMode {
    Imm8(usize, u8),
    Reg(usize, usize),
    ToI(usize),
}

pub enum SEMode {
    Imm8(usize, u8),
    Reg(usize, usize),
}

pub enum JPMode {
    NoOffset(u16),
    Offset(u16),
}
