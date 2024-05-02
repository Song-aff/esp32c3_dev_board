#![no_std]
#![no_main]
extern crate core;

extern crate alloc;

use core::cell::RefCell;
use core::mem::MaybeUninit;
use critical_section::Mutex;
use embedded_hal::digital::OutputPin;
use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::{ClockControl, CpuClock},
    delay::Delay,
    gpio::{Event, Gpio11, Gpio5, Gpio7, Input, PullDown, PullUp, IO},
    macros::ram,
    peripherals::{Peripherals, GPIO},
    prelude::*,
};

pub static SW_A: Mutex<RefCell<Option<Gpio7<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
pub static SW_B: Mutex<RefCell<Option<Gpio11<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
pub static SW_KEY: Mutex<RefCell<Option<Gpio5<Input<PullUp>>>>> = Mutex::new(RefCell::new(None));
pub static ControlCMD: Mutex<RefCell<Control>> =
    Mutex::new(RefCell::new(Control { cmd: CMD::None }));

#[derive(Debug)]
pub enum CMD {
    Plus,
    Reduce,
    Reset,
    None,
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
    pub fn set_reset(&mut self) {
        self.cmd = CMD::Reset;
    }
    pub fn consume(&mut self) -> CMD {
        let output;
        match self.cmd {
            CMD::Plus => {
                output = CMD::Plus;
            }
            CMD::Reduce => {
                output = CMD::Reduce;
            }
            CMD::Reset => {
                output = CMD::Reset;
            }
            CMD::None => {
                output = CMD::None;
            }
        }
        self.cmd = CMD::None;
        output
    }
}

#[handler]
#[ram]
pub fn handler() {
    critical_section::with(|cs| {
        // let mut binding = SW_A.borrow_ref_mut(cs);
        // let mut sw_a = binding.as_mut().unwrap();
        // let sw_b = SW_B.borrow_ref_mut(cs).as_mut().unwrap();
        let mut sw_a_ref = SW_A.borrow_ref_mut(cs);
        let mut sw_a = sw_a_ref.as_mut().unwrap();
        let sw_b_ref = SW_B.borrow_ref(cs);
        let sw_b = sw_b_ref.as_ref().unwrap();
        let mut sw_key_ref = SW_KEY.borrow_ref_mut(cs);
        let mut sw_key = sw_key_ref.as_mut().unwrap();
        let mut control = ControlCMD.borrow_ref_mut(cs);
        if sw_a.is_interrupt_set() {
            if sw_a.is_high() {
                if sw_b.is_high() {
                    control.set_plus();
                    // println!("+++");
                } else {
                    control.set_reduce();
                    // println!("---");
                }
            }
            sw_a.clear_interrupt();
        } else {
            control.set_reset();
            sw_key.clear_interrupt();
        }

        // SW_A.borrow_ref_mut(cs).as_mut().unwrap().clear_interrupt();
    });
}
