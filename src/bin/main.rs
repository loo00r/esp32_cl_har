#![no_std]
#![no_main]
#![deny(
    clippy::mem_forget,
    reason = "mem::forget is generally not safe to do with esp_hal types, especially those \
    holding buffers for the duration of a data transfer."
)]
#![deny(clippy::large_stack_frames)]

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
};
use log::info;

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

fn init() -> esp_hal::peripherals::Peripherals {
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    esp_hal::init(config)
}

#[allow(clippy::large_stack_frames, reason = "main")]
#[main]
fn main() -> ! {
    let peripherals = init();

    info!("ESP32 HAR started");

    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());

    let mut count: u32 = 0;
    loop {
        led.set_high();
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(500) {}

        led.set_low();
        let t = Instant::now();
        while t.elapsed() < Duration::from_millis(500) {}

        count += 1;
        info!("blink {}", count);

        if count >= 50 {
            break;
        }
    }

    info!("done after {} blinks", count);
    loop {}
}
