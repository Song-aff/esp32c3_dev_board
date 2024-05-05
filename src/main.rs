#![no_std]
#![no_main]
extern crate core;

extern crate alloc;

use core::mem::MaybeUninit;
use display_interface_spi::SPIInterface;
use esp_backtrace as _;
use esp_println::println;
use hal::spi::master::Spi;
use hal::{
    clock::{ClockControl, CpuClock},
    delay::Delay,
    gpio::{self, Event, IO},
    peripherals::Peripherals,
    prelude::*,
    spi::SpiMode,
};
mod slint_init;
use mipidsi::options::{Orientation, Rotation};
use slint_init::slint_init;
mod cmd;
use cmd::*;
use mipidsi::models::ST7796;

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

    // 初始化资源

    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();
    let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

    let mut delay = Delay::new(&clocks);
    let mut io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

    // 设置中断
    io.set_interrupt_handler(handler);
    let mut sw_a = io.pins.gpio7.into_pull_down_input();
    let sw_b = io.pins.gpio11.into_pull_down_input();
    let mut sw_key = io.pins.gpio5.into_pull_up_input();
    critical_section::with(|cs| {
        sw_key.listen(Event::FallingEdge);
        sw_a.listen(Event::RisingEdge);
        SW_A.borrow_ref_mut(cs).replace(sw_a);
        SW_B.borrow_ref_mut(cs).replace(sw_b);
        SW_KEY.borrow_ref_mut(cs).replace(sw_key);
    });
    let clk = io.pins.gpio2.into_push_pull_output();
    let sdo = io.pins.gpio10.into_push_pull_output();
    let cs = io.pins.gpio19.into_push_pull_output();

    let spi = Spi::new(peripherals.SPI2, 60u32.MHz(), SpiMode::Mode0, &clocks).with_pins(
        Some(clk),
        Some(sdo),
        gpio::NO_PIN,
        gpio::NO_PIN,
    );
    println!("spi init.");

    let dc = io.pins.gpio6.into_push_pull_output();
    let rst = io.pins.gpio3.into_push_pull_output();
    let spi_device = embedded_hal_bus::spi::ExclusiveDevice::new(spi, cs, delay);

    let di = SPIInterface::new(spi_device, dc);

    let display = mipidsi::Builder::new(ST7796, di)
        .orientation(Orientation {
            rotation: Rotation::Deg0,
            mirrored: true,
        })
        .reset_pin(rst)
        .color_order(mipidsi::options::ColorOrder::Rgb)
        .display_size(320, 480)
        .init(&mut delay)
        .unwrap();

    // 初始化slint
    slint_init(display);
    panic!("The MCU demo should not quit");
}
