#![no_std]
#![no_main]
extern crate core;

extern crate alloc;

use core::cell::RefCell;
use core::mem::MaybeUninit;
use critical_section::Mutex;
use esp_backtrace as _;
use esp_println::println;
use hal::{
    clock::{ClockControl, CpuClock},
    delay::Delay,
    gpio::{Gpio11, Gpio5, Gpio7, Input, PullUp},
    peripherals::{Peripherals, GPIO},
    prelude::*,
};

mod slint_init;
use slint_init::slint_init;
mod sw;
use sw::init_sw;
#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

fn init_heap() {
    const HEAP_SIZE: usize = 190 * 1024;
    static mut HEAP: MaybeUninit<[u8; HEAP_SIZE]> = MaybeUninit::uninit();

    unsafe {
        ALLOCATOR.init(HEAP.as_mut_ptr() as *mut u8, HEAP_SIZE);
    }
}

slint::include_modules!();

#[entry]
fn main() -> ! {
    init_heap();
    // slint_init();
    init_sw();
    // panic!("The MCU demo should not quit");
    loop {}
}
