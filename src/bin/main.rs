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
    i2c::master::{Config as I2cConfig, Error as I2cError, I2c},
    main,
    time::{Duration, Instant, Rate},
};
use esp32_cl_har::{
    inference::{InferenceError, MicroflowStub},
    model::{
        BASELINE_CLASSIFIER_ARTIFACT, FEATURE_EXTRACTOR_ARTIFACT, FEATURE_TENSOR_SIZE,
        INPUT_TENSOR_SIZE, NUM_CLASSES, SAMPLE_RATE_HZ, WINDOW_STRIDE,
    },
    mpu6050::{ALT_ADDRESS, DEFAULT_ADDRESS, Mpu6050},
    quant::{dequantize_feature_tensor, quantize_window},
    window::SlidingWindow,
};
use log::info;

const SAMPLE_PERIOD: Duration = Duration::from_millis(50);
const LOG_EVERY_SAMPLES: u32 = SAMPLE_RATE_HZ;

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
    let t = Instant::now();
    while t.elapsed() < duration {}
}

fn busy_wait_until(deadline: Instant) {
    while Instant::now() < deadline {}
}

fn probe_sensor<'d>(
    i2c: &mut I2c<'d, esp_hal::Blocking>,
) -> Result<(Mpu6050, u8), I2cError> {
    for address in [DEFAULT_ADDRESS, ALT_ADDRESS] {
        let sensor = Mpu6050::new(address);
        match sensor.init(i2c) {
            Ok(who_am_i) => return Ok((sensor, who_am_i)),
            Err(I2cError::AcknowledgeCheckFailed(_)) => continue,
            Err(err) => return Err(err),
        }
    }

    Err(I2cError::AcknowledgeCheckFailed(
        esp_hal::i2c::master::AcknowledgeCheckFailedReason::Address,
    ))
}

#[allow(clippy::large_stack_frames, reason = "main")]
#[main]
fn main() -> ! {
    let peripherals = init();

    info!("ESP32 HAR started");
    info!(
        "probing MPU6050 over I2C on GPIO21/GPIO22, target {} Hz accel sampling",
        SAMPLE_RATE_HZ
    );

    let mut led = Output::new(peripherals.GPIO2, Level::Low, OutputConfig::default());
    let i2c_config = I2cConfig::default().with_frequency(Rate::from_khz(100));
    let mut i2c = match I2c::new(peripherals.I2C0, i2c_config) {
        Ok(i2c) => i2c.with_sda(peripherals.GPIO21).with_scl(peripherals.GPIO22),
        Err(err) => {
            info!("i2c init error: {}", err);
            loop {}
        }
    };

    let (sensor, who_am_i) = match probe_sensor(&mut i2c) {
        Ok((sensor, who_am_i)) => (sensor, who_am_i),
        Err(err) => {
            info!("mpu6050 probe failed: {}", err);
            loop {
                led.set_high();
                busy_wait(Duration::from_millis(100));
                led.set_low();
                busy_wait(Duration::from_millis(100));
            }
        }
    };

    info!(
        "mpu6050 detected at 0x{:02X}, WHO_AM_I=0x{:02X}",
        sensor.address(),
        who_am_i
    );
    info!(
        "phase 3 skeleton ready: backend={}, classifier_artifact={}, feature_artifact={}",
        MicroflowStub::new().backend_name(),
        BASELINE_CLASSIFIER_ARTIFACT,
        FEATURE_EXTRACTOR_ARTIFACT,
    );

    let mut next_sample_at = Instant::now() + SAMPLE_PERIOD;
    let mut sample_count: u32 = 0;
    let mut led_on = false;
    let mut window = SlidingWindow::new();
    let inference = MicroflowStub::new();
    let mut quantized_input = [0_i8; INPUT_TENSOR_SIZE];
    let mut classifier_output = [0_i8; NUM_CLASSES];
    let mut quantized_features = [0_i8; FEATURE_TENSOR_SIZE];
    let mut dequantized_features = [0.0_f32; FEATURE_TENSOR_SIZE];
    let mut samples_since_inference: usize = 0;
    let mut inference_attempts: u32 = 0;
    let mut logged_full_window = false;

    loop {
        busy_wait_until(next_sample_at);
        let started_at = Instant::now();
        next_sample_at += SAMPLE_PERIOD;

        match sensor.read_accel(&mut i2c) {
            Ok(accel) => {
                sample_count += 1;
                window.push(accel.xyz);

                if window.is_full() {
                    if !logged_full_window {
                        logged_full_window = true;
                        info!(
                            "window buffer ready: {} samples collected, stride={}",
                            window.len(),
                            WINDOW_STRIDE,
                        );
                    }

                    samples_since_inference += 1;

                    if samples_since_inference >= WINDOW_STRIDE {
                        samples_since_inference = 0;
                        inference_attempts += 1;
                        quantize_window(&window, &mut quantized_input);

                        let class_result =
                            inference.classify(&quantized_input, &mut classifier_output);
                        let feature_result =
                            inference.extract_features(&quantized_input, &mut quantized_features);

                        match (class_result, feature_result) {
                            (Ok(()), Ok(())) => {
                                dequantize_feature_tensor(
                                    &quantized_features,
                                    &mut dequantized_features,
                                );
                                info!(
                                    "inference ok: attempt={}, cls_q0={}, feat_f32_0={}",
                                    inference_attempts,
                                    classifier_output[0],
                                    dequantized_features[0],
                                );
                            }
                            (Err(InferenceError::BackendUnavailable), _)
                            | (_, Err(InferenceError::BackendUnavailable)) => {
                                info!(
                                    "inference skeleton hit backend stub: attempt={}, input_q0={}",
                                    inference_attempts,
                                    quantized_input[0],
                                );
                            }
                        }
                    }
                }

                if sample_count % LOG_EVERY_SAMPLES == 0 {
                    led_on = !led_on;
                    if led_on {
                        led.set_high();
                    } else {
                        led.set_low();
                    }

                    let loop_time_us = started_at.elapsed().as_micros();
                    let elapsed_total_us = started_at.duration_since_epoch().as_micros();
                    info!(
                        "samples={}, t_ms={}, loop_us={}, window_len={}, accel=({}, {}, {})",
                        sample_count,
                        elapsed_total_us / 1_000,
                        loop_time_us,
                        window.len(),
                        accel.xyz[0],
                        accel.xyz[1],
                        accel.xyz[2],
                    );
                }
            }
            Err(err) => {
                info!(
                    "mpu6050 accel read error after {} samples: {}",
                    sample_count,
                    err
                );
            }
        }
    }
}
