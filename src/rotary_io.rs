use embassy_rp::{pio::{Instance, StateMachine, Common, PioPin, Config, Direction, ShiftDirection}, relocate::RelocatedProgram, clocks};
use fixed::types::U24F8;

const DIVISOR: i32 = 4;

const TRANSITIONS: [i32; 16] = [
    0,    // 00 -> 00 no movement
    -1,   // 00 -> 01 3/4 ccw (11 detent) or 1/4 ccw (00 at detent)
    1,   // 00 -> 10 3/4 cw or 1/4 cw
    0,     // 00 -> 11 non-Gray-code transition
    1,   // 01 -> 00 2/4 or 4/4 cw
    0,    // 01 -> 01 no movement
    0,    // 01 -> 10 non-Gray-code transition
    -1,   // 01 -> 11 4/4 or 2/4 ccw
    -1,   // 10 -> 00 2/4 or 4/4 ccw
    0,    // 10 -> 01 non-Gray-code transition
    0,    // 10 -> 10 no movement
    1,   // 10 -> 11 4/4 or 2/4 cw
    0,    // 11 -> 00 non-Gray-code transition
    1,   // 11 -> 01 1/4 or 3/4 cw
    -1,   // 11 -> 10 1/4 or 3/4 ccw
    0,    // 11 -> 11 no movement
];

pub struct RotaryIO<'d, P: Instance, const S: usize> {
    sm: StateMachine<'d, P, S>,
    state: u32,
    sub_count: i32,
    position: i32,
}

impl<'d, P: Instance, const S: usize> RotaryIO<'d, P, S> {
    pub fn new(
        pio: &mut Common<'d, P>,
        mut sm: StateMachine<'d, P, S>,
        pin_a: impl PioPin,
        pin_b: impl PioPin,
    ) -> Self {
        let pio_program = pio_proc::pio_asm!(
            "set y, 31",
            "again:",
            ".wrap_target",
            "in pins, 2",
            "mov x, isr",
            "jmp x!=y, push_data",
            "mov isr, null",
            "jmp again",
            "push_data:",
            "push",
            "mov y, x",
            ".wrap",
        );
        let relocated_program = RelocatedProgram::new(&pio_program.program);
        let mut statemachine_config = Config::default();
        
        let loaded_program = pio.load_program(&relocated_program);
        statemachine_config.use_program(&loaded_program, &[]);
    
        let mut encoder_rot_a = pio.make_pio_pin(pin_a);
        encoder_rot_a.set_pull(embassy_rp::gpio::Pull::Up);
        let mut encoder_rot_b = pio.make_pio_pin(pin_b);
        encoder_rot_b.set_pull(embassy_rp::gpio::Pull::Up);
        sm.set_pin_dirs(Direction::In, &[&encoder_rot_a, &encoder_rot_b]);
        statemachine_config.set_in_pins(&[&encoder_rot_a, &encoder_rot_b]);
        statemachine_config.clock_divider = U24F8::from_num(clocks::clk_sys_freq() / 1_000_000);
        // statemachine_config.shift_in.auto_fill = true;
        statemachine_config.shift_in.direction = ShiftDirection::Left;
        sm.set_config(&statemachine_config);
        sm.set_enable(true);

        let state = 0;
        let sub_count = 0;
        let position = 0;

        RotaryIO {
            sm,
            state,
            sub_count,
            position,
        }
    }

    pub async fn wait_subcount_change(&mut self) -> i32 {
        loop {
            let new_state = self.sm.rx().wait_pull().await & 0x3;
    
            let idx = ((self.state << 2) | new_state) as usize;
            self.state = new_state;
        
            let sub_incr = TRANSITIONS[idx];
        
            self.sub_count += sub_incr;
    
            if sub_incr != 0 {
                return self.sub_count;
            }            
        }
    }

    pub async fn wait_position_change(&mut self) -> i32 {
        loop {
            let _ = self.wait_subcount_change().await;

            if self.sub_count >= DIVISOR {
                self.position += 1;
                self.sub_count = 0;
                return self.position;
            } else if self.sub_count <= -DIVISOR {
                self.position -= 1;
                self.sub_count = 0;
                return self.position;
            }
        }
    }
}
