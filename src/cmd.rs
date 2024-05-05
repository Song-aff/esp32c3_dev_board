#![no_std]
#![no_main]
extern crate core;

extern crate alloc;

use core::{cell::RefCell, mem};
use critical_section::Mutex;
// use embedded_hal::digital::OutputPin;
use esp_backtrace as _;
// use esp_println::println;
use hal::{
    gpio::{Gpio11, Gpio5, Gpio7, Input, PullDown, PullUp},
    macros::ram,
    prelude::*,
};

pub static SW_A: Mutex<RefCell<Option<Gpio7<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
pub static SW_B: Mutex<RefCell<Option<Gpio11<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
pub static SW_KEY: Mutex<RefCell<Option<Gpio5<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));
pub static CONTROL_CMD: Mutex<RefCell<Control>> =
    Mutex::new(RefCell::new(Control { cmd: CMD::None }));

#[derive(Debug)]
pub enum CMD {
    Plus,
    Reduce,
    KeyDown,
    None,
    Touch(i32, i32),
}
pub struct Control {
    cmd: CMD,
}
impl Control {
    pub fn new() -> Self {
        Control { cmd: CMD::None }
    }
    pub fn set_plus(&mut self) {
        self.cmd = CMD::Plus;
    }
    pub fn set_reduce(&mut self) {
        self.cmd = CMD::Reduce;
    }
    pub fn set_key_down(&mut self) {
        self.cmd = CMD::KeyDown;
    }
    pub fn set_touch(&mut self, x: i32, y: i32) {
        self.cmd = CMD::Touch(x, y);
    }
    pub fn consume(&mut self) -> CMD {
        // 获取命令值返回，并将命令置空
        let mut output = CMD::None;
        mem::swap(&mut self.cmd, &mut output);
        // match self.cmd {
        //     CMD::Plus => {
        //         output = CMD::Plus;
        //     }
        //     CMD::Reduce => {
        //         output = CMD::Reduce;
        //     }
        //     CMD::KeyDown => {
        //         output = CMD::KeyDown;
        //     }
        //     CMD::None => {
        //         output = CMD::None;
        //     }
        // }
        // self.cmd = CMD::None;
        output
    }
}

#[handler]
#[ram]
pub fn handler() {
    critical_section::with(|cs| {
        let mut sw_a_ref = SW_A.borrow_ref_mut(cs);
        let sw_a = sw_a_ref.as_mut().unwrap();
        let sw_b_ref = SW_B.borrow_ref(cs);
        let sw_b = sw_b_ref.as_ref().unwrap();
        let mut sw_key_ref = SW_KEY.borrow_ref_mut(cs);
        let sw_key = sw_key_ref.as_mut().unwrap();
        let mut control = CONTROL_CMD.borrow_ref_mut(cs);
        if sw_a.is_interrupt_set() {
            if sw_a.is_high() {
                if sw_b.is_high() {
                    control.set_plus();
                } else {
                    control.set_reduce();
                }
            }
            sw_a.clear_interrupt();
        } else {
            control.set_key_down();
            sw_key.clear_interrupt();
        }
    });
}
