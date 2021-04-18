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

use std::time::Duration;

use pixels::{ Pixels, SurfaceTexture };

use winit::dpi::LogicalSize;
use winit::event::{ Event, VirtualKeyCode };
use winit::event_loop::{ ControlFlow, EventLoop };
use winit::window::WindowBuilder;
use winit_input_helper::WinitInputHelper;

mod chip8;
use chip8::Chip8;

const SCALE: u8 = 10;

fn main() { 
    // initialization //

    let mut chippy = Chip8::initialize("test_roms/Tetris [Fran Dachille, 1991].ch8");

    let event_loop = EventLoop::new();
    let mut input = WinitInputHelper::new();
    let window = {
        let size = LogicalSize::new(
            chip8::VIDEO_WIDTH * SCALE as u32, 
            chip8::VIDEO_HEIGHT * SCALE as u32);

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
            chip8::VIDEO_WIDTH * SCALE as u32, 
            chip8::VIDEO_HEIGHT * SCALE as u32, 
            surface_texture).unwrap()
    };

    // event loop //

    event_loop.run(move |event, _, control_flow| {
        chippy.cycle();

        // draw the current frame
        if let Event::RedrawRequested(_) = event {
            let frame = pixels.get_frame();
            for (i, pixel) in frame.chunks_exact_mut(4).enumerate() {
                let x = (i % (chip8::VIDEO_WIDTH * SCALE as u32) as usize) / SCALE as usize;
                let y = (i / (chip8::VIDEO_WIDTH * SCALE as u32) as usize) / SCALE as usize;

                let rgba = if chippy.display_memory[
                    (y * chip8::VIDEO_WIDTH as usize) + x as usize
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
