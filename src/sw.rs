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

static SW_A: Mutex<RefCell<Option<Gpio7<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
static SW_B: Mutex<RefCell<Option<Gpio11<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
static SW_KEY: Mutex<RefCell<Option<Gpio5<Input<PullDown>>>>> = Mutex::new(RefCell::new(None));
pub static COUNT: Mutex<RefCell<i32>> = Mutex::new(RefCell::new(0));

pub fn init_sw() {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();
    // let mut delay = Delay::new(&clocks);
    let mut io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    io.set_interrupt_handler(handler1);
    let mut sw_a = io.pins.gpio7.into_pull_down_input();
    let sw_b = io.pins.gpio11.into_pull_down_input();
    let mut sw_key = io.pins.gpio5.into_pull_down_input();
    critical_section::with(|cs| {
        sw_key.listen(Event::FallingEdge);
        sw_a.listen(Event::FallingEdge);
        SW_A.borrow_ref_mut(cs).replace(sw_a);
        SW_B.borrow_ref_mut(cs).replace(sw_b);
        SW_KEY.borrow_ref_mut(cs).replace(sw_key);
    });
}

#[handler]
#[ram]
fn handler1() {
    critical_section::with(|cs| {
        // let mut binding = SW_A.borrow_ref_mut(cs);
        // let mut sw_a = binding.as_mut().unwrap();
        // let sw_b = SW_B.borrow_ref_mut(cs).as_mut().unwrap();
        let mut sw_a_ref = SW_A.borrow_ref_mut(cs);
        let mut sw_a = sw_a_ref.as_mut().unwrap();
        let sw_b_ref = SW_B.borrow_ref(cs);
        let sw_b = sw_b_ref.as_ref().unwrap();
        let sw_key_ref = SW_KEY.borrow_ref(cs);
        let sw_key = sw_key_ref.as_ref().unwrap();
        if sw_b.is_high() {
            println!("+++");
        } else {
            println!("---");
        }
        sw_a.clear_interrupt();
        // SW_A.borrow_ref_mut(cs).as_mut().unwrap().clear_interrupt();
    });
}
