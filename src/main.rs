/// Chippy, a CHIP-8 emulator
///
/// Notes:
///
/// Recommended input mapping:
/// ```
/// Keypad       Keyboard
/// +-+-+-+-+    +-+-+-+-+
/// |1|2|3|C|    |1|2|3|4|
/// +-+-+-+-+    +-+-+-+-+
/// |4|5|6|D|    |Q|W|E|R|
/// +-+-+-+-+ => +-+-+-+-+
/// |7|8|9|E|    |A|S|D|F|
/// +-+-+-+-+    +-+-+-+-+
/// |A|0|B|F|    |Z|X|C|V|
/// +-+-+-+-+    +-+-+-+-+
/// ```

use std::fs;
use std::fs::File;
use std::io::Write;
use std::time::Duration;

use rand::Rng; 
use rand::rngs::ThreadRng;

use pixels::{ Pixels, SurfaceTexture };

use winit::dpi::LogicalSize;
use winit::event::{ Event, VirtualKeyCode };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

const VIDEO_WIDTH: u32 = 64;
const VIDEO_HEIGHT: u32 = 32;
const SCALE: u8 = 10;

const FONTSET_START_ADDRESS: u8 = 0x50;
const ROM_START_ADDRESS: u16 = 0x200;

/// The CHIP-8 has 4KiB of memory (4096 bytes) and
/// various other things to keep track of things.
struct Chip8 {
    index: usize,
    program_counter: usize,
    stack: [usize; 16],
    stack_pointer: usize,
    delay_timer: u8,
    sound_timer: u8,
    registers: [u8; 16],
    memory: [u8; 4096],
    display_memory: [u8; (VIDEO_WIDTH * VIDEO_HEIGHT) as usize],
    keypad: [bool; 16],
    rng: ThreadRng,
}

impl Chip8 {
    fn new() -> Self {
        Self {
            index: 0,
            program_counter: ROM_START_ADDRESS as usize,
            stack: [0; 16],
            stack_pointer: 0,
            delay_timer: 0,
            sound_timer: 0,
            registers: [0; 16],
            memory: [0; 4096],
            display_memory: [0; (VIDEO_WIDTH * VIDEO_HEIGHT) as usize],
            keypad: [false; 16],
            rng: rand::thread_rng(),
        }
    }

    fn initialize() -> Self {
        let mut chippy = Self::new();

        chippy.load_fontset();

        chippy
    }

    fn load_fontset(&mut self) { 
        let fontset = [
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

        for i in 0..fontset.len() {
            self.memory[FONTSET_START_ADDRESS as usize + i] = fontset[i];
        }
    }

    fn cycle(&mut self) {
        // fetch the opcode (by merging two consecutive bytes of memory)
        let opcode = ((self.memory[self.program_counter] as u16) << 8) 
            | self.memory[self.program_counter + 1] as u16;
        
        // increment the program counter before executing anything
        self.program_counter += 2; // increment by two since two bytes of memory were consumed

        // decode and execute the opcode
        self.decode_and_execute(opcode);

        // decrement the delay timer if it's been set
        if self.delay_timer > 0 { self.delay_timer -= 1; }

        // decrement the sound timer if it's been set
        if self.sound_timer > 0 { self.sound_timer -= 1; }
    }

    fn decode_and_execute(&mut self, opcode: u16) {
        match opcode {
            0x00E0 => self.opcode_00e0(), // clear the screen
            0x00EE => self.opcode_00ee(), // return from subroutine
            _ => {
                match opcode & 0xF000 {
                    0x1000 => self.opcode_1nnn(opcode),
                    0x2000 => self.opcode_2nnn(opcode),
                    0x3000 => self.opcode_3xkk(opcode),
                    0x4000 => self.opcode_4xkk(opcode),
                    0x5000 => self.opcode_5xy0(opcode),
                    0x6000 => self.opcode_6xkk(opcode),
                    0x7000 => self.opcode_7xkk(opcode),
                    0x8000 => {
                        match opcode & 0x000F {
                            0x0000 => self.opcode_8xy0(opcode),
                            0x0001 => self.opcode_8xy1(opcode),
                            0x0002 => self.opcode_8xy2(opcode),
                            0x0003 => self.opcode_8xy3(opcode),
                            0x0004 => self.opcode_8xy4(opcode),
                            0x0005 => self.opcode_8xy5(opcode),
                            0x0006 => self.opcode_8xy6(opcode),
                            0x0007 => self.opcode_8xy7(opcode),
                            0x000E => self.opcode_8xye(opcode),
                            _ => {},
                        }
                    },
                    0x9000 => self.opcode_9xy0(opcode),
                    0xA000 => self.opcode_annn(opcode),
                    0xB000 => self.opcode_bnnn(opcode),
                    0xC000 => self.opcode_cxkk(opcode),
                    0xD000 => self.opcode_dxyn(opcode),
                    0xE000 => {
                        match opcode & 0x00FF {
                            0x009E => self.opcode_ex9e(opcode),
                            0x00A1 => self.opcode_exa1(opcode),
                            _ => {},
                        }
                    },
                    0xF000 => {
                        match opcode & 0x00FF {
                            0x0007 => self.opcode_fx07(opcode),
                            0x000A => self.opcode_fx0a(opcode),
                            0x0015 => self.opcode_fx15(opcode),
                            0x0018 => self.opcode_fx18(opcode),
                            0x001e => self.opcode_fx1e(opcode),
                            0x0029 => self.opcode_fx29(opcode),
                            0x0033 => self.opcode_fx33(opcode),
                            0x0055 => self.opcode_fx55(opcode),
                            0x0065 => self.opcode_fx65(opcode),
                            _ => {},
                        }
                    },
                    _ => {},
                }
            },
        }
    }

    fn load_rom(&mut self, path: &str) {
        let rom_contents = fs::read(path).expect("failed to read ROM file");

        // read the ROM into memory
        for i in 0..rom_contents.len() {
            self.memory[ROM_START_ADDRESS as usize + i] = rom_contents[i];
        }
    }

    #[allow(unused)]
    fn dump_memory(&self) {
        println!("Memory dump:");
        for i in 0..&self.memory.len()-1 {
            print!("{:#x}, ", self.memory[i]);
        }
        println!("{:#x}", self.memory.iter().last().unwrap());
    }

    #[allow(unused)]
    fn dump_memory_to_file(&self, path: Option<&str>) {
        let mut memory_log_file: File;
        if path != None {
             memory_log_file = File::create(path.unwrap())
                .expect("failed to create memory dump file");
        } else { 
             memory_log_file = File::create("memory_dump")
                .expect("failed to create memory dump file");
        }
        memory_log_file.write_all(&self.memory).unwrap(); 
    }

    // INSTRUCTIONS //

    // CLS - clear the screen
    fn opcode_00e0(&mut self) {
        self.display_memory = [0; 64 * 32];
    }

    // RET - return from subroutine
    fn opcode_00ee(&mut self) {
        self.stack_pointer -= 1;
        self.program_counter = self.stack[self.stack_pointer];
    }

    // JP addr - 1NNN. A jump doesn't remember its origin, so 
    // no stack interaction is required.
    fn opcode_1nnn(&mut self, opcode: u16) {
        let address = opcode & 0x0FFF; // extracts the address at location NNN
        self.program_counter = address as usize;
    }

    // CALL addr - 2NNN. Call subroutine at NNN.
    fn opcode_2nnn(&mut self, opcode: u16) {
        let address = opcode & 0x0FFF; // extracts the address at location NNN 
        
        // to be able to return from this subroutine, we 
        // push the program counter to the stack.
        self.stack[self.stack_pointer] = self.program_counter; 
        self.stack_pointer += 1;

        self.program_counter = address as usize;

        // (NOTE: a call is basically just a jump that can remembers it's origin.)
    }

    // 3XKK - SE VX, byte. Skip the next instruction if VX = KK.
    fn opcode_3xkk(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let byte = opcode & 0x00FF;
        
        if self.registers[vx as usize] == byte as u8 { 
            self.program_counter += 2;
        }
    }
    
    // 4XKK - SNE VX, byte. Skip the next instruction if VX != KK.
    fn opcode_4xkk(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let byte = opcode & 0x00FF;
        
        if self.registers[vx as usize] != byte as u8 {
            self.program_counter += 2;
        }
    }

    // 5XY0 - SE VX, VY. Skip the next instruction if VX = VY.
    fn opcode_5xy0(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        if self.registers[vx as usize] == self.registers[vy as usize] {
            self.program_counter += 2;
        }
    }

    // 6XKK - LD VX, byte. Set VX = KK.
    fn opcode_6xkk(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let byte = opcode & 0x00FF;

        self.registers[vx as usize] = byte as u8;
    }

    // 7XKK - ADD VX, byte. Set VX = VX + KK.
    fn opcode_7xkk(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let byte = opcode & 0x00FF;

        // wrap the value if it exceeds 8 bits.
        self.registers[vx as usize] = 
            ((self.registers[vx as usize] as u16 + byte) % 256) as u8;
    }

    // 8XY0 - LD VX, VY. Set VX = VY.
    fn opcode_8xy0(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        self.registers[vx as usize] = self.registers[vy as usize];
    }

    // 8XY1 - OR VX, VY. Set VX = VX | VY.
    fn opcode_8xy1(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        self.registers[vx as usize] |= self.registers[vy as usize];
    }

    // 8XY2 - AND VX, VY. Set VX = VX & VY.
    fn opcode_8xy2(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        self.registers[vx as usize] &= self.registers[vy as usize];
    }

    // 8XY3 - XOR VX, VY. Set VX = VX ^ VY.
    fn opcode_8xy3(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        self.registers[vx as usize] ^= self.registers[vy as usize];
    }

    // 8XY4 - ADD VX, VY. Set VX = VX + VY, set VF = carry.
    fn opcode_8xy4(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        let sum = (self.registers[vx as usize] as u16 + self.registers[vy as usize] as u16) % 256;

        // toggle the carry bit in register VF if the 
        // sum would overflow 8 bits.
        if sum > 255 { 
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0; 
        }

        // set VX to the lowest 8 bits of the sum.
        self.registers[vx as usize] = sum as u8 & 0xFF;
    }

    // 8XY5 - SUB VX, VY. Set VX = VX - VY, set VF = !carry.
    fn opcode_8xy5(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        let diff: u8;
        if self.registers[vx as usize] > self.registers[vy as usize] {
            self.registers[0xF] = 1;

            // subtract if the sum is not negative.
            self.registers[vx as usize] -= self.registers[vy as usize];
        } else {
            self.registers[0xF] = 0;

            // otherwise get the difference and wrap around.
            diff = self.registers[vy as usize] - self.registers[vx as usize];
            self.registers[vx as usize] = (256 - diff as u16) as u8;
        }
    }

    // 8XY6 - SHR VX. Set VX = VX SHR 1.
    fn opcode_8xy6(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let lsb = vx & 0x1;

        // save the least significant bit in VF.
        self.registers[0xF] = lsb as u8;

        // one right-shift is equivalent to division by two.
        self.registers[vx as usize] >>= 1;
    }

    // 8XY7 - SUBN VX, VY. Set VX = VY - VX and set VF = !carry.
    fn opcode_8xy7(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        if self.registers[vy as usize] > self.registers[vx as usize] {
            self.registers[0xF] = 1;
        } else {
            self.registers[0xF] = 0;
        }

        self.registers[vx as usize] = 
            self.registers[vy as usize] - self.registers[vx as usize];
    }

    // 8XYE - SHL VX. Set VX = VX SHL 1.
    fn opcode_8xye(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let msb = (self.registers[vx as usize] & 0x80) >> 7;

        // save the most significant bit.
        self.registers[0xF] = msb;

        // one left-shift is equivalent to multiplication by two.
        self.registers[vx as usize] <<= 1;
    }

    // 9XY0 - SNE VX, VY. Skip next instruction if VX != VY.
    fn opcode_9xy0(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        if self.registers[vx as usize] != self.registers[vy as usize] {
            self.program_counter += 2;
        }
    }

    // ANNN - LD I, addr. Set I = NNN.
    fn opcode_annn(&mut self, opcode: u16) {
        let address = opcode & 0x0FFF;

        self.index = address as usize;
    }

    // BNNN - JP V0, addr. Jump to location NNN + V0.
    fn opcode_bnnn(&mut self, opcode: u16) {
        let address = opcode & 0x0FFF;

        self.program_counter = (self.registers[0x0] + address as u8) as usize;
    }

    // CXKK - RND VX, byte. Set Vx = random byte AND kk.
    fn opcode_cxkk(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let byte = opcode & 0x00FF;

        self.registers[vx as usize] = self.rng.gen::<u8>() & byte as u8;
    }

    // DXYN - DRW VX, VY, nibble. Display n-byte sprite starting at 
    // memory location I at (VX, VY), set VF = collision.
    fn opcode_dxyn(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let vy = (opcode & 0x00F0) >> 4;

        let height = opcode & 0x000F;

        // wrap if the sprite goes beyond the screen boundaries.
        let x_pos = self.registers[vx as usize] as u32 % VIDEO_WIDTH;
        let y_pos = self.registers[vy as usize] as u32 % VIDEO_HEIGHT;

        self.registers[0xF] = 0;

        for row in 0..height {
            let sprite_byte = self.memory[self.index + row as usize];

            for col in 0..8 {
                let sprite_pixel = sprite_byte & (0x80 >> col);

                let mut screen_pixel = 
                    self.display_memory[((y_pos + row as u32) * VIDEO_WIDTH + (x_pos + col)) as usize] as u32;

                // if both the sprite pixel and the screen pixel is on: collision
                if sprite_pixel != 0 { 
                    if screen_pixel == 0xFFFFFFFF { 
                        self.registers[0xF] = 1;
                    }

                    // effectively XOR with the sprite pixel
                    screen_pixel ^= 0xFFFFFFFF;
                    self.display_memory[
                        ((y_pos + row as u32) * VIDEO_WIDTH + (x_pos + col)) as usize
                    ] = screen_pixel as u8;
                }
            }
        }
    }

    // EX9E - SKP VX. Skip next instruction if key with 
    // the value of VX is pressed.
    fn opcode_ex9e(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let key = self.registers[vx as usize];

        if self.keypad[key as usize] { // the key was pressed
            self.program_counter += 2;
        }
    }

    // EXA1 - SKNP VX. Skip next instruction if key with
    // the value of VX is not pressed.  
    fn opcode_exa1(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let key = self.registers[vx as usize];

        if !self.keypad[key as usize] { // the key was pressed
            self.program_counter += 2;
        }
    }

    // FX07 - LD VX, DT. Set VX = delay timer value.
    fn opcode_fx07(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        self.registers[vx as usize] = self.delay_timer;
    }

    // FX0A - LD VX, K. Wait for a key press, store the
    // value of the key in VX.
    fn opcode_fx0a(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        if self.keypad[0x0] {
            self.registers[vx as usize] = 0x0;
        } else if self.keypad[0x1] {
            self.registers[vx as usize] = 0x1;
        } else if self.keypad[0x2] {
            self.registers[vx as usize] = 0x2;
        } else if self.keypad[0x3] {
            self.registers[vx as usize] = 0x3;
        } else if self.keypad[0x4] {
            self.registers[vx as usize] = 0x4;
        } else if self.keypad[0x5] {
            self.registers[vx as usize] = 0x5;
        } else if self.keypad[0x6] {
            self.registers[vx as usize] = 0x6;
        } else if self.keypad[0x7] {
            self.registers[vx as usize] = 0x7;
        } else if self.keypad[0x8] {
            self.registers[vx as usize] = 0x8;
        } else if self.keypad[0x9] {
            self.registers[vx as usize] = 0x9;
        } else if self.keypad[0xA] {
            self.registers[vx as usize] = 0xA;
        } else if self.keypad[0xB] {
            self.registers[vx as usize] = 0xB;
        } else if self.keypad[0xC] {
            self.registers[vx as usize] = 0xC;
        } else if self.keypad[0xD] {
            self.registers[vx as usize] = 0xD;
        } else if self.keypad[0xE] {
            self.registers[vx as usize] = 0xE;
        } else if self.keypad[0xF] {
            self.registers[vx as usize] = 0xF;
        } else { // decrease the PC to reiterate the same instruction.
            self.program_counter -= 2;
        }
    }

    // FX15 - LD DT, VX. Set delay timer = VX.
    fn opcode_fx15(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        self.delay_timer = self.registers[vx as usize];
    }

    // FX18 - LD ST, VX. Set sound timer = VX.
    fn opcode_fx18(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        self.sound_timer = self.registers[vx as usize];
    }

    // FX1E - ADD I, VX. Set I = I + VX.
    fn opcode_fx1e(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        self.index += self.registers[vx as usize] as usize;
    }

    // FX29 - LD F, VX. Set I = location of sprite for digit VX.
    fn opcode_fx29(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let digit = self.registers[vx as usize];

        self.index = (FONTSET_START_ADDRESS + (5 * digit)) as usize;
    }

    // FX33 - LD B, VX. Store BCD representation of VX in 
    // memory locations I, I+1, and I+2.
    fn opcode_fx33(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;
        let mut value = self.registers[vx as usize];

        // ones-place
        self.memory[self.index + 2] = value % 10;
        value /= 10;
        
        // tens-place
        self.memory[self.index + 1] = value % 10;
        value /= 10;

        // hundreds-place
        self.memory[self.index] = value % 10;
    }

    // FX55 - LD [I], VX. Store registers V0 through VX in 
    // memory starting at location I.
    fn opcode_fx55(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        for i in 0..=vx as usize {
            self.memory[self.index + i] = self.registers[i];
        }
    }

    // FX65 - LD VX, [I]. Read registers V0 through VX from
    // memory starting at location I.
    fn opcode_fx65(&mut self, opcode: u16) {
        let vx = (opcode & 0x0F00) >> 8;

        for i in 0..=vx as usize {
            self.registers[i] = self.memory[self.index + i];
        }
    }
}

fn main() { 
    // initialization //

    let mut chippy = Chip8::initialize();
    
    chippy.load_rom("test_roms/test_opcode.ch8");

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(
            VIDEO_WIDTH * SCALE as u32, 
            VIDEO_HEIGHT * SCALE as u32);

        WindowBuilder::new()
            .with_title("Chippy")
            .with_resizable(false)
            .with_inner_size(size)
            .build(&event_loop)
            .unwrap()
    };

    let mut pixels = {
        let window_size = window.inner_size();
        let surface_texture = SurfaceTexture::new(
            window_size.width, 
            window_size.height, 
            &window);

        Pixels::new(
            VIDEO_WIDTH * SCALE as u32, 
            VIDEO_HEIGHT * SCALE as u32, 
            surface_texture).unwrap()
    };

    // event loop //

    event_loop.run(move |event, _, control_flow| {
        chippy.cycle();

        // draw the current frame
        if let Event::RedrawRequested(_) = event {
            let frame = pixels.get_frame();
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let x = (i % (VIDEO_WIDTH * SCALE as u32) as usize) / SCALE as usize;
                let y = (i / (VIDEO_WIDTH * SCALE as u32) as usize) / SCALE as usize;

                let rgba = if chippy.display_memory[
                    (y * VIDEO_WIDTH as usize) + x as usize
                ] == 0xFF {
                    [0x5E, 0x48, 0xE8, 0xFF]
                } else {
                    [0x48, 0xB2, 0xE8, 0xFF] 
                };

                pixel.copy_from_slice(&rgba);
            }
            if pixels
                .render()
                .map_err(|e| eprintln!("pixels.render() failed: {}", e))
                .is_err()
            {
                *control_flow = ControlFlow::Exit;
                return;
            }
        }

        if input.update(&event) {
            // close events
            if input.key_pressed(VirtualKeyCode::Escape) || input.quit() {
                *control_flow = ControlFlow::Exit;
                return ()
            }
            
            // resize the window
            if let Some(size) = input.window_resized() {
                pixels.resize(size.width, size.height);
            }

            // if a key is pressed, set the corresponding keypad mappings.
            if input.key_pressed(VirtualKeyCode::Key1) {
                chippy.keypad[0x1] = true;
            }
            if input.key_pressed(VirtualKeyCode::Key2) {
                chippy.keypad[0x2] = true;
            }
            if input.key_pressed(VirtualKeyCode::Key3) {
                chippy.keypad[0x3] = true;
            }
            if input.key_pressed(VirtualKeyCode::Key4) {
                chippy.keypad[0xC] = true;
            }
            if input.key_pressed(VirtualKeyCode::Q) {
                chippy.keypad[0x4] = true;
            }
            if input.key_pressed(VirtualKeyCode::W) {
                chippy.keypad[0x5] = true;
            }
            if input.key_pressed(VirtualKeyCode::E) {
                chippy.keypad[0x6] = true;
            }
            if input.key_pressed(VirtualKeyCode::R) {
                chippy.keypad[0xD] = true;
            }
            if input.key_pressed(VirtualKeyCode::A) {
                chippy.keypad[0x7] = true;
            }
            if input.key_pressed(VirtualKeyCode::S) {
                chippy.keypad[0x8] = true;
            }
            if input.key_pressed(VirtualKeyCode::D) {
                chippy.keypad[0x9] = true;
            }
            if input.key_pressed(VirtualKeyCode::F) {
                chippy.keypad[0xE] = true;
            }
            if input.key_pressed(VirtualKeyCode::Z) {
                chippy.keypad[0xA] = true;
            }
            if input.key_pressed(VirtualKeyCode::X) {
                chippy.keypad[0x0] = true;
            }
            if input.key_pressed(VirtualKeyCode::C) {
                chippy.keypad[0xB] = true;
            }
            if input.key_pressed(VirtualKeyCode::V) {
                chippy.keypad[0xF] = true;
            }

            // same for key releases.
            if input.key_released(VirtualKeyCode::Key1) {
                chippy.keypad[0x1] = false;
            }
            if input.key_released(VirtualKeyCode::Key2) {
                chippy.keypad[0x2] = false;
            }
            if input.key_released(VirtualKeyCode::Key3) {
                chippy.keypad[0x3] = false;
            }
            if input.key_released(VirtualKeyCode::Key4) {
                chippy.keypad[0xC] = false;
            }
            if input.key_released(VirtualKeyCode::Q) {
                chippy.keypad[0x4] = false;
            }
            if input.key_released(VirtualKeyCode::W) {
                chippy.keypad[0x5] = false;
            }
            if input.key_released(VirtualKeyCode::E) {
                chippy.keypad[0x6] = false;
            }
            if input.key_released(VirtualKeyCode::R) {
                chippy.keypad[0xD] = false;
            }
            if input.key_released(VirtualKeyCode::A) {
                chippy.keypad[0x7] = false;
            }
            if input.key_released(VirtualKeyCode::S) {
                chippy.keypad[0x8] = false;
            }
            if input.key_released(VirtualKeyCode::D) {
                chippy.keypad[0x9] = false;
            }
            if input.key_released(VirtualKeyCode::F) {
                chippy.keypad[0xE] = false;
            }
            if input.key_released(VirtualKeyCode::Z) {
                chippy.keypad[0xA] = false;
            }
            if input.key_released(VirtualKeyCode::X) {
                chippy.keypad[0x0] = false;
            }
            if input.key_released(VirtualKeyCode::C) {
                chippy.keypad[0xB] = false;
            }
            if input.key_released(VirtualKeyCode::V) {
                chippy.keypad[0xF] = false;
            } 
        }

        // request a redraw and sleep for some duration
        window.request_redraw(); 
        std::thread::sleep(Duration::from_millis(1000/60));
    });
}
