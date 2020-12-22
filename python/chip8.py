import contextlib
with contextlib.redirect_stdout(None):
    import pygame
import sys
import numpy as np
import time
import random

class Chip8:
    instrs = ['LD', 'ADD', 'SUB', 'SUBN', 'OR', 'AND', 'XOR', 'SHR', 'SHL', 'RND', # arithmetic
              'RET', 'JP', 'CALL', 'SE', 'SNE', # control flow
              'CLS', 'DRW', # screen
              'SKP', 'SKNP'] # keyboard
    pc_modifying = ['JP nnn', 'CALL nnn', 'RET']
    digits = np.array([
                0xF0, 0x90, 0x90, 0x90, 0xF0,
                0x20, 0x60, 0x20, 0x20, 0x70,
                0xF0, 0x10, 0xF0, 0x80, 0xF0,
                0xF0, 0x10, 0xF0, 0x10, 0xF0,
                0x90, 0x90, 0xF0, 0x10, 0x10,
                0xF0, 0x80, 0xF0, 0x10, 0xF0,
                0xF0, 0x80, 0xF0, 0x90, 0xF0,
                0xF0, 0x10, 0x20, 0x40, 0x40,
                0xF0, 0x90, 0xF0, 0x90, 0xF0,
                0xF0, 0x90, 0xF0, 0x10, 0xF0,
                0xF0, 0x90, 0xF0, 0x90, 0x90,
                0xE0, 0x90, 0xE0, 0x90, 0xE0,
                0xF0, 0x80, 0x80, 0x80, 0xF0,
                0xE0, 0x90, 0x90, 0x90, 0xE0,
                0xF0, 0x80, 0xF0, 0x80, 0xF0,
                0xF0, 0x80, 0xF0, 0x80, 0x80])
    keys=['1','2','3','4',
          'q','w','e','r',
          'a','s','d','f',
          'z','x','c','v']
    key_values = [ 1, 2,  3, 12,
                   4, 5,  6, 13,
                   7, 8,  9, 14,
                  10, 0, 11, 15]
    digit_loc = 0x0
    start = 0x200
    screen_size = np.array([64, 32])

    def __init__(self, path):
        pygame.init()

        self.RAM = np.zeros(0x1000, dtype=np.uint8)
        self.V = np.zeros(0x10, dtype=np.uint8)
        self.stack = np.zeros(0x10, dtype=np.uint16)
        self.I = 0
        self.PC = 0
        self.SP = -1
        self.DT = 0
        self.ST = 0
        self.scale_factor = 10
        self.keyboard = np.zeros(0x10, dtype=np.uint8)
        self.screen = np.zeros(self.screen_size, dtype=np.uint8)
        self.display = pygame.display.set_mode(self.screen_size * self.scale_factor, flags = pygame.DOUBLEBUF)

        self.RAM[self.digit_loc:self.digit_loc + len(self.digits)] = self.digits
        with open(path, 'rb') as f:
            binary = list(f.read())
            self.RAM[self.start:self.start + len(binary)] = binary

    def run(self):
        self.PC = self.start
        instr = None
        timer_clock = cpu_clock = time.time()
        while True:
            instr = self.fetch_instr(self.PC)
            output = hex(self.PC)[2:].zfill(4) + ' ' + hex(instr)[2:].zfill(4)
            result = self.run_instr(instr)
            if result == None: # invalid instruction
                print('INVALID INSTRUCTION:', hex(instr))
                return
            print(output, result.ljust(13, ' '), self.V)
            if result not in self.pc_modifying:
                self.PC += 2

            key, val = self.poll_keypress()
            if key:
                self.keyboard[key] = val

            # Timers
            cur_time = time.time()
            if cur_time - timer_clock > 1/60:
                if self.DT > 0: self.DT -= 1
                if self.ST > 0: self.ST -= 1
                timer_clock = cur_time

            while (cur_time := time.time()) and cur_time - cpu_clock < 1/500:
                continue
            cpu_clock = cur_time

    def draw(self):
        screen = np.repeat(np.expand_dims(self.screen * 255, 2), 3, axis=2)
        scale_matrix = np.ones((self.scale_factor, self.scale_factor, 1))
        pixels = np.kron(screen, scale_matrix).astype('uint8')

        self.display.blit(pygame.surfarray.make_surface(pixels), (0, 0))
        pygame.display.update()

    def poll_keypress(self):
        for event in pygame.event.get():
            if event.type == pygame.KEYDOWN:
                if event.key == pygame.K_ESCAPE:
                    exit()
                elif (k := self.get_key(event.key)) is not None:
                    return k, 1
            if event.type == pygame.KEYUP:
                if (k := self.get_key(event.key)) is not None:
                    return k, 0
            if event.type == pygame.QUIT:
                exit()
        return None, 0

    def get_key(self, n):
        if n < 0x80:
            k = chr(n)
            if k in self.keys:
                return self.key_values[self.keys.index(k)]

    def push(self, v):
        self.SP += 1
        self.stack[self.SP] = v

    def pop(self):
        res = self.stack[self.SP]
        self.SP -= 1
        return res

    def fetch_instr(self, addr):
        return (self.RAM[addr] << 8) + self.RAM[addr + 1]

    def run_instr(self, instr):
        s, x, y, n = (instr >> 12), ((instr >> 8) & 0xF), ((instr >> 4) & 0xF), instr & 0xF
        kk = instr & 0xff
        nnn = instr & 0xfff
        if s == 0x0:
            if x == 0x0:
                if kk == 0xE0: self.CLS(); return 'CLS'
                elif kk == 0xEE: self.RET(); return 'RET'
        elif s == 0x1: self.JP(nnn, mode=0); return 'JP nnn'
        elif s == 0x2: self.CALL(nnn); return 'CALL nnn'
        elif s == 0x3: self.SE(x, kk, mode=0); return 'SE Vx, kk'
        elif s == 0x4: self.SNE(x, kk, mode=0); return 'SNE Vx, kk'
        elif s == 0x5: self.SE(x, y, mode=1); return 'SE Vx, Vy'
        elif s == 0x6: self.LD(x, kk, mode=0); return 'LD Vx, kk'
        elif s == 0x7: self.ADD(x, kk, mode=0); return 'ADD Vx, kk'
        elif s == 0x8:
            if n == 0: self.LD(x, y, mode=1); return 'LD Vx, Vy'
            elif n == 1: self.OR(x, y); return 'OR Vx, Vy'
            elif n == 2: self.AND(x, y); return 'AND Vx, Vy'
            elif n == 3: self.XOR(x, y); return 'XOR Vx, Vy'
            elif n == 4: self.ADD(x, y, mode=1); return 'ADD Vx, Vy'
            elif n == 5: self.SUB(x, y); return 'SUB Vx, Vy'
            elif n == 6: self.SHR(x); return 'SHR Vx {, Vy}'
            elif n == 7: self.SUB(y, x); return 'SUBN Vx, Vy'
            elif n == 0xE: self.SHL(x); return 'SHL Vx {, Vy}'
        elif s == 0x9: self.SNE(x, y, mode=1); return 'SNE Vx, Vy'
        elif s == 0xA: self.LD(nnn, mode=2); return 'LD I, nnn'
        elif s == 0xB: self.JP(nnn, mode=1); return 'JP V0, nnn'
        elif s == 0xC: self.RND(x, kk); return 'RND Vx, kk'
        elif s == 0xD: self.DRW(x, y, n); return 'DRW Vx, Vy, n'
        elif s == 0xE:
            if kk == 0x9E: self.SKP(x); return 'SKP Vx'
            elif kk == 0xA1: self.SKNP(x); return 'SKNP Vx'
        elif s == 0xF:
            if kk == 0x07: self.LD(x, mode=3); return 'LD Vx, DT'
            elif kk == 0x0A: self.LD(x, mode=4); return 'LD Vx, K'
            elif kk == 0x15: self.LD(x, mode=5); return 'LD DT, Vx'
            elif kk == 0x18: self.LD(x, mode=6); return 'LD ST, Vx'
            elif kk == 0x1E: self.ADD(x, mode=2); return 'ADD I, Vx'
            elif kk == 0x29: self.LD(x, mode=7); return 'LD F, Vx'
            elif kk == 0x33: self.LD(x, mode=8); return 'LD B, Vx'
            elif kk == 0x55: self.LD(x, mode=9); return 'LD [I], Vx'
            elif kk == 0x65: self.LD(x, mode=10); return 'LD Vx, [I]'

    def LD(self, *args, mode):
        if mode == 0:
            x, kk = args
            self.V[x] = kk
        elif mode == 1:
            x, y = args
            self.V[x] = self.V[y]
        elif mode == 2:
            nnn, = args
            self.I = nnn
        else:
            x, = args
            if mode == 3: self.V[x] = self.DT
            elif mode == 4: # TODO: timers need to still tick down here
                while True:
                    key, val = self.poll_keypress()
                    if key and val:
                        self.V[x] = key
                        break
            elif mode == 5: self.DT = self.V[x]
            elif mode == 6: self.ST = self.V[x]
            elif mode == 7: self.I = self.digit_loc + 5 * self.V[x]
            elif mode == 8:
                BCD = [self.V[x] // 100, (self.V[x] % 100) // 10, self.V[x] % 10]
                self.RAM[self.I:self.I+3] = BCD
            elif mode == 9: self.RAM[self.I:self.I+x+1] = self.V[:x+1]
            elif mode == 10: self.V[:x+1] = self.RAM[self.I:self.I+x+1]
            else: raise Exception('Invalid LD operation mode:', mode)

    def ADD(self, *args, mode):
        if mode == 0:
            x, kk = args
            self.V[x] += kk
        elif mode == 1:
            x, y = args
            res = int(self.V[x]) + int(self.V[y])
            self.V[0xF] = (res > 0xff)
            self.V[x] = res
        elif mode == 2:
            x, = args
            self.I += self.V[x]
            self.I &= 0xfff
        else: raise Exception('Invalid ADD operation mode:', mode)

    def SUB(self, x, y):
        res = int(self.V[x]) - int(self.V[y])
        self.V[0xF] = (res > 0)
        self.V[x] = res

    def OR(self, x, y):
        self.V[x] |= self.V[y]

    def AND(self, x, y):
        self.V[x] &= self.V[y]

    def XOR(self, x, y):
        self.V[x] ^= self.V[y]

    def SHR(self, x):
        self.V[0xF] = self.V[x] & 0x1
        self.V[x] >>= 1

    def SHL(self, x):
        self.V[0xF] = (self.V[x] & 0x80 != 0)
        self.V[x] <<= 1

    def RND(self, x, kk):
        self.V[x] = random.randint(0, 255) & kk

    def SE(self, *args, mode):
        if mode == 0:
            x, kk = args
            if self.V[x] == kk:
                self.PC += 2
        elif mode == 1:
            x, y = args
            if self.V[x] == self.V[y]:
                self.PC += 2
        else: raise Exception('Invalid SE operation mode:', mode)

    def SNE(self, *args, mode):
        if mode == 0:
            x, kk = args
            if self.V[x] != kk:
                self.PC += 2
        elif mode == 1:
            x, y = args
            if self.V[x] != self.V[y]:
                self.PC += 2
        else: raise Exception('Invalid SNE operation mode:', mode)

    def SKP(self, x):
        if self.keyboard[self.V[x]]:
            self.PC += 2

    def SKNP(self, x):
        if not self.keyboard[self.V[x]]:
            self.PC += 2

    def JP(self, nnn, mode):
        if mode == 0:
            self.PC = nnn
        elif mode == 1:
            self.PC == nnn + self.V[0]
        else: raise Exception('Invalid JP operation mode:', mode)

    def CALL(self, nnn):
        self.push(self.PC + 2)
        self.PC = nnn

    def RET(self):
        self.PC = self.pop()

    def CLS(self):
        self.screen[:,:] = 0

    def DRW(self, x, y, n):
        sprite = np.array(self.RAM[self.I:self.I+n])
        bitarray = np.unpackbits(np.expand_dims(sprite, 1), axis=1)

        Vx, Vy = self.V[x], self.V[y]
        block = self.screen[Vx:Vx+8, Vy:Vy+n].T
        bitarray = bitarray[tuple(map(slice, block.shape))]
        block ^= bitarray
        self.V[0xF] = (np.any(block != bitarray))
        self.draw()

c = Chip8(sys.argv[1])
c.run()
