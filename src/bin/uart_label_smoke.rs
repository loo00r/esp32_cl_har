#![no_std]
#![no_main]

// Phase 4 UART label smoke test.
// Verifies non-blocking supervised label input only.
// No sensor, no MicroFlow, no ReplayBuffer, no OnlineLayer, no persistence.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    main,
    time::{Duration, Instant},
    uart::{Config as UartConfig, Uart},
};
use esp32_cl_har::model::CLASS_LABELS;
use log::{info, warn};

const IDLE_DELAY: Duration = Duration::from_millis(10);
const TICK_INTERVAL: Duration = Duration::from_millis(1_000);

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

fn busy_wait(duration: Duration) {
    let started = Instant::now();
    while started.elapsed() < duration {}
}

fn parse_label(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'5' => Some(byte - b'0'),
        b'\r' | b'\n' | b' ' | b'\t' => None,
        _ => None,
    }
}

#[main]
fn main() -> ! {
    let peripherals = init();
    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let mut uart = Uart::new(peripherals.UART0, UartConfig::default())
        .expect("UART0 init failed")
        .with_rx(peripherals.GPIO3);

    let mut led_on = false;
    let mut buf = [0_u8; 8];
    let mut labels_seen = 0_u32;
    let mut invalid_seen = 0_u32;
    let mut ticks = 0_u32;
    let mut last_tick = Instant::now();

    info!("uart label smoke started");
    info!("send one-character labels over UART0/USB serial: 0..5");
    info!("labels: 0=Walking 1=Jogging 2=Upstairs 3=Downstairs 4=Sitting 5=Standing");
    info!("no sensor, no MicroFlow, no ReplayBuffer, no flash writes");

    loop {
        match uart.read_buffered(&mut buf) {
            Ok(0) => {}
            Ok(count) => {
                for &byte in &buf[..count] {
                    if let Some(label) = parse_label(byte) {
                        let label_idx = usize::from(label);
                        labels_seen = labels_seen.saturating_add(1);
                        info!(
                            "LABEL_RX label={} name={} total_labels={}",
                            label, CLASS_LABELS[label_idx], labels_seen
                        );
                    } else if !matches!(byte, b'\r' | b'\n' | b' ' | b'\t') {
                        invalid_seen = invalid_seen.saturating_add(1);
                        warn!("LABEL_INVALID byte={} total_invalid={}", byte, invalid_seen);
                    }
                }
            }
            Err(_) => {
                invalid_seen = invalid_seen.saturating_add(1);
                warn!("LABEL_RX_ERROR total_invalid={}", invalid_seen);
            }
        }

        if last_tick.elapsed() >= TICK_INTERVAL {
            ticks = ticks.saturating_add(1);
            info!(
                "UART_SMOKE tick={} labels={} invalid={}",
                ticks, labels_seen, invalid_seen
            );

            led_on = !led_on;
            if led_on {
                led.set_high();
            } else {
                led.set_low();
            }
            last_tick = Instant::now();
        }

        busy_wait(IDLE_DELAY);
    }
}
