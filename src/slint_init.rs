#![no_std]
#![no_main]
extern crate core;

extern crate alloc;
use alloc::boxed::Box;
use alloc::rc::Rc;
use critical_section::Mutex;
use mipidsi::options::{Orientation, Rotation};
use slint::platform::software_renderer::MinimalSoftwareWindow;

// use crate::COUNT;
use crate::sw::*;
use core::cell::RefCell;
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
    systimer::SystemTimer,
};
use mipidsi::{models::ST7796, Display};
slint::include_modules!();
pub fn slint_init() {
    let window = MinimalSoftwareWindow::new(
        slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
        // Default::default()
    );

    slint::platform::set_platform(Box::new(EspBackend {
        window: window.clone(),
        counter: 1,
    }))
    .expect("backend already initialized");

    let main_window = Recipe::new().unwrap();

    let strong = main_window.clone_strong();
    // let timer = slint::Timer::default();
    let cmd_timer = slint::Timer::default();
    // timer.start(
    //     slint::TimerMode::Repeated,
    //     core::time::Duration::from_millis(1000),
    //     move || {
    //         if strong.get_counter() <= 0 {
    //             strong.set_counter(25);
    //         } else {
    //             strong.set_counter(0);
    //         }
    //     },
    // );
    cmd_timer.start(
        slint::TimerMode::Repeated,
        core::time::Duration::from_millis(20),
        move || {
            critical_section::with(|cs| {
                let cmd = ControlCMD.borrow_ref_mut(cs).consume();
                let counter = strong.get_counter();
                match cmd {
                    CMD::Plus => {
                        strong.set_counter(counter + 1);
                        println!("{}", counter);
                    }
                    CMD::Reduce => {
                        strong.set_counter(counter - 1);
                        println!("{}", counter);
                    }
                    CMD::None => {}
                    CMD::Reset => {
                        strong.set_counter(0);
                        println!("{}", counter);
                    }
                }
            });
        },
    );

    main_window.run().unwrap();
}

// #[derive(Default)]
pub struct EspBackend {
    // window: RefCell<Option<Rc<slint::platform::software_renderer::MinimalSoftwareWindow>>>,
    window: Rc<MinimalSoftwareWindow>,
    counter: u8,
}

impl slint::platform::Platform for EspBackend {
    fn create_window_adapter(
        &self,
    ) -> Result<Rc<dyn slint::platform::WindowAdapter>, slint::PlatformError> {
        // let window = slint::platform::software_renderer::MinimalSoftwareWindow::new(
        //     slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
        // );
        // self.window.replace(Some(window.clone()));
        // Ok(window)
        Ok(self.window.clone())
    }

    fn duration_since_start(&self) -> core::time::Duration {
        core::time::Duration::from_millis(
            SystemTimer::now() / (SystemTimer::TICKS_PER_SECOND / 1000),
        )
    }

    fn run_event_loop(&self) -> Result<(), slint::PlatformError> {
        let peripherals = Peripherals::take();
        let system = peripherals.SYSTEM.split();
        let clocks = ClockControl::configure(system.clock_control, CpuClock::Clock160MHz).freeze();

        let mut delay = Delay::new(&clocks);
        let mut io = IO::new(peripherals.GPIO, peripherals.IO_MUX);

        //
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
        //
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

        println!("display init.");

        let size = slint::PhysicalSize::new(320, 480);

        self.window.set_size(size);

        let mut buffer_provider = DrawBuffer {
            display,
            buffer: &mut [slint::platform::software_renderer::Rgb565Pixel::default(); 320],
        };

        // let main_window = Recipe::new().unwrap();
        // let strong = main_window.clone_strong();
        // let counter = strong.get_counter();
        loop {
            slint::platform::update_timers_and_animations();
            // critical_section::with(|cs| {
            //     let cmd = ControlCMD.borrow_ref_mut(cs).consume();
            //     match cmd {
            //         CMD::Plus => {
            //             strong.set_counter(counter + 1);
            //             println!("{}", counter);
            //         }
            //         CMD::Reduce => {
            //             strong.set_counter(counter - 1);
            //             println!("{}", counter);
            //         }
            //         CMD::None => {}
            //     }
            // });
            self.window.draw_if_needed(|renderer| {
                renderer.render_by_line(&mut buffer_provider);
            });
            if self.window.has_active_animations() {
                continue;
            }
        }
    }

    fn debug_log(&self, arguments: core::fmt::Arguments) {
        println!("{}", arguments);
    }
}

struct DrawBuffer<'a, Display> {
    display: Display,
    buffer: &'a mut [slint::platform::software_renderer::Rgb565Pixel],
}

impl<DI: display_interface::WriteOnlyDataCommand, RST: embedded_hal::digital::OutputPin>
    slint::platform::software_renderer::LineBufferProvider
    for &mut DrawBuffer<'_, Display<DI, mipidsi::models::ST7796, RST>>
{
    type TargetPixel = slint::platform::software_renderer::Rgb565Pixel;

    fn process_line(
        &mut self,
        line: usize,
        range: core::ops::Range<usize>,
        render_fn: impl FnOnce(&mut [slint::platform::software_renderer::Rgb565Pixel]),
    ) {
        let buffer = &mut self.buffer[range.clone()];

        render_fn(buffer);

        self.display
            .set_pixels(
                range.start as u16,
                line as _,
                range.end as u16,
                line as u16,
                buffer
                    .iter()
                    .map(|x| embedded_graphics_core::pixelcolor::raw::RawU16::new(x.0).into()),
            )
            .unwrap();
    }
}
