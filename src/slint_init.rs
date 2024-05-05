#![no_std]
#![no_main]
extern crate core;

extern crate alloc;
use alloc::boxed::Box;
use alloc::rc::Rc;
use slint::platform::software_renderer::MinimalSoftwareWindow;

use crate::cmd::*;
use core::cell::RefCell;
use esp_println::println;
use hal::spi::master::Spi;
use hal::{
    gpio::{self, Event, IO},
    systimer::SystemTimer,
};
use mipidsi::{models::ST7796, Display};
slint::include_modules!();
pub fn slint_init<
    DI: display_interface::WriteOnlyDataCommand,
    RST: embedded_hal::digital::OutputPin,
>(
    display: Display<DI, ST7796, RST>,
) {
    let window = MinimalSoftwareWindow::new(
        slint::platform::software_renderer::RepaintBufferType::ReusedBuffer,
        // Default::default()
    );

    slint::platform::set_platform(Box::new(EspBackend {
        window: window.clone(),
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
                let cmd = CONTROL_CMD.borrow_ref_mut(cs).consume();
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
                    CMD::Reset => {
                        strong.set_counter(0);
                        println!("{}", counter);
                    }
                    _ => {}
                }
            });
        },
    );

    println!("display init.");

    let size = slint::PhysicalSize::new(320, 480);

    window.set_size(size);

    let mut buffer_provider = DrawBuffer {
        display,
        buffer: &mut [slint::platform::software_renderer::Rgb565Pixel::default(); 320],
    };

    loop {
        slint::platform::update_timers_and_animations();
        window.draw_if_needed(|renderer| {
            renderer.render_by_line(&mut buffer_provider);
        });
        if window.has_active_animations() {
            continue;
        }
    }
}

// #[derive(Default)]
pub struct EspBackend {
    // window: RefCell<Option<Rc<slint::platform::software_renderer::MinimalSoftwareWindow>>>,
    window: Rc<MinimalSoftwareWindow>,
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
        todo!()
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
