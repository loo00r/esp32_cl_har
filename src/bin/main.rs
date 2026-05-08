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
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
use esp_hal::uart::{Config as UartConfig, Uart};
use esp32_cl_har::{
    model::{INPUT_TENSOR_SIZE, SAMPLE_RATE_HZ, WINDOW_STRIDE},
    mpu6050::{ALT_ADDRESS, DEFAULT_ADDRESS, Mpu6050},
    quant::quantize_window,
    window::SlidingWindow,
};
#[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
use esp32_cl_har::{
    inference::{FrozenInferenceBackend, InferenceError},
    model::{FEATURE_TENSOR_SIZE, NUM_CLASSES},
    quant::dequantize_feature_tensor,
};
#[cfg(feature = "microflow_backend")]
use esp32_cl_har::inference_microflow::MicroflowFeatureBackend;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
use esp32_cl_har::inference_microflow32::Microflow32FeatureBackend as MicroflowFeatureBackend;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
use esp32_cl_har::model::CLASS_LABELS;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
use esp32_cl_har::online_layer::OnlineLayer32;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
use esp32_cl_har::replay_buffer::{ReplayBuffer32, ReplayStrategy};
use log::info;

const SAMPLE_PERIOD: Duration = Duration::from_millis(50);
const LOG_EVERY_SAMPLES: u32 = SAMPLE_RATE_HZ;
#[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
const LATENCY_REPORT_EVERY_ATTEMPTS: u32 = 10;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
const CL_TRAIN_EVERY_LABELS: u32 = 10;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
const CL_BATCH_SIZE: usize = 12;
#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
const ACTIVE_REPLAY_POLICY: ReplayStrategy = ReplayStrategy::Reservoir;

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

#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
fn argmax(values: &[f32]) -> usize {
    let mut best_idx = 0;
    let mut best_value = values[0];
    let mut idx = 1;
    while idx < values.len() {
        if values[idx] > best_value {
            best_value = values[idx];
            best_idx = idx;
        }
        idx += 1;
    }
    best_idx
}

#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
fn replay_policy_name(policy: ReplayStrategy) -> &'static str {
    match policy {
        ReplayStrategy::Fifo => "fifo",
        ReplayStrategy::Reservoir => "reservoir",
    }
}

#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
fn read_uart_label(rx: &mut Uart<'_, esp_hal::Blocking>) -> Option<u8> {
    let mut byte = [0_u8; 1];
    loop {
        match rx.read(&mut byte) {
            Ok(1) => {
                let value = byte[0];
                if (b'0'..=b'5').contains(&value) {
                    return Some(value - b'0');
                }
            }
            Ok(_) => return None,
            Err(err) => {
                info!("LABEL_READ error={}", err);
                return None;
            }
        }
    }
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
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    let mut label_rx = match Uart::new(peripherals.UART0, UartConfig::default()) {
        Ok(uart) => uart.with_rx(peripherals.GPIO3),
        Err(err) => {
            info!("uart label rx init error: {}", err);
            loop {}
        }
    };
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
        "phase 3 streaming path ready: backend={}, classifier_artifact={}, feature_artifact={}",
        phase3_backend_name(),
        phase3_classifier_artifact_name(),
        phase3_feature_artifact_name(),
    );
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    {
        info!(
            "RESOURCE policy={} replay_ram_est=12288 online_head_ram_est=792 persistence=0 train_every_labels={} batch_size={}",
            replay_policy_name(ACTIVE_REPLAY_POLICY),
            CL_TRAIN_EVERY_LABELS,
            CL_BATCH_SIZE,
        );
        info!("LABEL_PROTOCOL send one ASCII digit 0..5 followed by newline; 0=Walking 1=Jogging 2=Upstairs 3=Downstairs 4=Sitting 5=Standing");
    }

    let mut next_sample_at = Instant::now() + SAMPLE_PERIOD;
    let mut sample_count: u32 = 0;
    let mut led_on = false;
    let mut window = SlidingWindow::new();
    #[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
    let inference = FrozenInferenceBackend::new();
    #[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
    let inference = MicroflowFeatureBackend::new();
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    let mut online_layer = OnlineLayer32::new_microflow32_pretrained();
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    let mut replay = ReplayBuffer32::new();
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    let mut cl_batch_features =
        [[0.0_f32; esp32_cl_har::model::MICROFLOW32_FEATURE_TENSOR_SIZE]; CL_BATCH_SIZE];
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    let mut cl_batch_labels = [0_u8; CL_BATCH_SIZE];
    #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
    let mut labels_since_train = 0_u32;
    let mut quantized_input = [0_i8; INPUT_TENSOR_SIZE];
    #[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
    let mut classifier_output = [0_i8; NUM_CLASSES];
    #[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
    let mut quantized_features = [0_i8; FEATURE_TENSOR_SIZE];
    #[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
    let mut dequantized_features = [0.0_f32; FEATURE_TENSOR_SIZE];
    let mut samples_since_inference: usize = 0;
    let mut inference_attempts: u32 = 0;
    let mut logged_full_window = false;
    #[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
    let mut latency_min_us: u64 = u64::MAX;
    #[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
    let mut latency_max_us: u64 = 0;
    #[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
    let mut latency_sum_us: u64 = 0;

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

                        #[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
                        {
                            let inference_started = Instant::now();
                            let features = inference.extract_features_quantized(&quantized_input);
                            let inference_us = inference_started.elapsed().as_micros();
                            latency_min_us = latency_min_us.min(inference_us);
                            latency_max_us = latency_max_us.max(inference_us);
                            latency_sum_us += inference_us;

                            info!(
                                "microflow feature ok: attempt={}, inference_us={}, input_q0={}, feat0={}, feat1={}, feat2={}, feat3={}",
                                inference_attempts,
                                inference_us,
                                quantized_input[0],
                                features[0],
                                features[1],
                                features[2],
                                features[3],
                            );

                            #[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
                            {
                                let online_started = Instant::now();
                                let probs = online_layer.forward(&features);
                                let online_us = online_started.elapsed().as_micros();
                                let predicted_idx = argmax(&probs);

                                info!(
                                    "PRED attempt={} class={} label={} conf={} infer_us={} head_us={}",
                                    inference_attempts,
                                    predicted_idx,
                                    CLASS_LABELS[predicted_idx],
                                    probs[predicted_idx],
                                    inference_us,
                                    online_us,
                                );

                                if let Some(label) = read_uart_label(&mut label_rx) {
                                    let fill_started = Instant::now();
                                    let added =
                                        replay.push(label, features, ACTIVE_REPLAY_POLICY).is_some();
                                    let fill_us = fill_started.elapsed().as_micros();
                                    let buffer_len = replay.total_len();

                                    if added {
                                        labels_since_train += 1;
                                    }

                                    info!(
                                        "LABEL label={} name={} added={} buffer_len={} total_seen={} fill_us={}",
                                        label,
                                        CLASS_LABELS[usize::from(label)],
                                        if added { 1 } else { 0 },
                                        buffer_len,
                                        replay.total_seen(),
                                        fill_us,
                                    );

                                    if added && labels_since_train >= CL_TRAIN_EVERY_LABELS {
                                        let sample_started = Instant::now();
                                        let batch_len = replay.sample_balanced_batch(
                                            &mut cl_batch_features,
                                            &mut cl_batch_labels,
                                        );
                                        let sample_us = sample_started.elapsed().as_micros();

                                        if batch_len > 0 {
                                            let update_started = Instant::now();
                                            online_layer.backward_batch(
                                                &cl_batch_features[..batch_len],
                                                &cl_batch_labels[..batch_len],
                                                0.001,
                                            );
                                            let update_us = update_started.elapsed().as_micros();
                                            labels_since_train = 0;

                                            info!(
                                                "TRAIN policy={} batch_len={} sample_us={} update_us={} total_seen={} buffer_len={}",
                                                replay_policy_name(ACTIVE_REPLAY_POLICY),
                                                batch_len,
                                                sample_us,
                                                update_us,
                                                replay.total_seen(),
                                                replay.total_len(),
                                            );
                                        }
                                    }
                                }
                            }

                            if inference_attempts % LATENCY_REPORT_EVERY_ATTEMPTS == 0 {
                                let latency_mean_us = latency_sum_us / u64::from(inference_attempts);
                                info!(
                                    "microflow latency stats: attempts={}, min_us={}, mean_us={}, max_us={}",
                                    inference_attempts,
                                    latency_min_us,
                                    latency_mean_us,
                                latency_max_us,
                                );
                            }
                        }

                        #[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
                        {
                            let class_result =
                                inference.classify(&quantized_input, &mut classifier_output);
                            let feature_result = inference
                                .extract_features(&quantized_input, &mut quantized_features);

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
                                        "frozen inference backend stub hit: attempt={}, input_q0={}",
                                        inference_attempts,
                                        quantized_input[0],
                                    );
                                }
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

#[cfg(any(feature = "microflow_backend", feature = "microflow32_backend"))]
fn phase3_backend_name() -> &'static str {
    MicroflowFeatureBackend::new().backend_name()
}

#[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
fn phase3_backend_name() -> &'static str {
    FrozenInferenceBackend::new().backend_name()
}

#[cfg(feature = "microflow_backend")]
fn phase3_classifier_artifact_name() -> &'static str {
    esp32_cl_har::model::BASELINE_CLASSIFIER_ARTIFACT
}

#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
fn phase3_classifier_artifact_name() -> &'static str {
    esp32_cl_har::model::MICROFLOW32_CLASSIFIER_ARTIFACT
}

#[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
fn phase3_classifier_artifact_name() -> &'static str {
    esp32_cl_har::model::BASELINE_CLASSIFIER_ARTIFACT
}

#[cfg(feature = "microflow_backend")]
fn phase3_feature_artifact_name() -> &'static str {
    esp32_cl_har::model::FEATURE_EXTRACTOR_ARTIFACT
}

#[cfg(all(feature = "microflow32_backend", not(feature = "microflow_backend")))]
fn phase3_feature_artifact_name() -> &'static str {
    esp32_cl_har::model::MICROFLOW32_FEATURE_EXTRACTOR_ARTIFACT
}

#[cfg(not(any(feature = "microflow_backend", feature = "microflow32_backend")))]
fn phase3_feature_artifact_name() -> &'static str {
    esp32_cl_har::model::FEATURE_EXTRACTOR_ARTIFACT
}
