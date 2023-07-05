const VRAM_WIDTH: usize = 64;
const VRAM_HEIGHT: usize = 32;

struct Chip8Emulator<'i> {
    input_subscriber: DynSubscriber<'i, InputEvent>,
    keymap: [usize; 12],
    vram: [bool; VRAM_WIDTH * VRAM_HEIGHT],
    active_keys: [bool; 16],
}

impl<'i> Chip8Emulator<'i> {
    pub fn new() -> Self {
        let input_subscriber = INPUT_CHANNEL.dyn_subscriber().unwrap();
        let keymap = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];
        let vram  = [false; VRAM_WIDTH * VRAM_HEIGHT];
        let active_keys = [false; 16];
    }
}