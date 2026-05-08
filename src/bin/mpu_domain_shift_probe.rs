#![no_std]
#![no_main]

// Phase 3 domain-shift probe.
// Samples real MPU6050 accelerometer data and reports raw/mps2/z-score/int8 stats
// against the WISDM normalization constants. No inference, CL, replay, or writes.

use esp_hal::{
    clock::CpuClock,
    gpio::{Level, Output, OutputConfig},
    i2c::master::{Config as I2cConfig, Error as I2cError, I2c},
    main,
    time::{Duration, Instant, Rate},
};
use esp32_cl_har::{
    model::{FEATURE_COUNT, SAMPLE_RATE_HZ},
    mpu6050::{ALT_ADDRESS, DEFAULT_ADDRESS, Mpu6050},
    quant::{INPUT_SCALE, INPUT_ZERO_POINT, WISDM_ZSCORE_STATS, quantize_scalar, raw_accel_to_mps2},
};
use log::info;

const SAMPLE_PERIOD: Duration = Duration::from_millis(50);
const SAMPLE_LIMIT: u32 = 200;
const IDLE_DELAY: Duration = Duration::from_millis(1_000);

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}

esp_bootloader_esp_idf::esp_app_desc!();

#[derive(Clone, Copy)]
struct AxisStats {
    count: u32,
    sum: f32,
    sum_sq: f32,
    min: f32,
    max: f32,
}

impl AxisStats {
    const fn new() -> Self {
        Self {
            count: 0,
            sum: 0.0,
            sum_sq: 0.0,
            min: f32::INFINITY,
            max: f32::NEG_INFINITY,
        }
    }

    fn push(&mut self, value: f32) {
        self.count += 1;
        self.sum += value;
        self.sum_sq += value * value;
        self.min = self.min.min(value);
        self.max = self.max.max(value);
    }

    fn mean(&self) -> f32 {
        self.sum / self.count as f32
    }

    fn std(&self) -> f32 {
        let mean = self.mean();
        let variance = (self.sum_sq / self.count as f32) - (mean * mean);
        libm::sqrtf(variance.max(0.0))
    }
}

#[derive(Clone, Copy)]
struct QuantStats {
    min: i8,
    max: i8,
    sat_min: u32,
    sat_max: u32,
}

impl QuantStats {
    const fn new() -> Self {
        Self {
            min: i8::MAX,
            max: i8::MIN,
            sat_min: 0,
            sat_max: 0,
        }
    }

    fn push(&mut self, value: i8) {
        self.min = self.min.min(value);
        self.max = self.max.max(value);
        if value == i8::MIN {
            self.sat_min += 1;
        }
        if value == i8::MAX {
            self.sat_max += 1;
        }
    }
}

fn init() -> esp_hal::peripherals::Peripherals {
    esp_println::logger::init_logger(log::LevelFilter::Info);
    let config = esp_hal::Config::default().with_cpu_clock(CpuClock::max());
    esp_hal::init(config)
}

fn busy_wait(duration: Duration) {
    let started = Instant::now();
    while started.elapsed() < duration {}
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

fn log_axis_stats(label: &str, axis: usize, stats: AxisStats) {
    info!(
        "{} axis{}: mean={}, std={}, min={}, max={}",
        label,
        axis,
        stats.mean(),
        stats.std(),
        stats.min,
        stats.max,
    );
}

#[main]
fn main() -> ! {
    let peripherals = init();
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
            loop {}
        }
    };

    info!(
        "mpu domain-shift probe started: addr=0x{:02X}, WHO_AM_I=0x{:02X}, samples={}, rate_hz={}",
        sensor.address(),
        who_am_i,
        SAMPLE_LIMIT,
        SAMPLE_RATE_HZ,
    );
    info!(
        "WISDM z-score means={:?}, stds={:?}, input_scale={}, input_zero_point={}",
        WISDM_ZSCORE_STATS.means,
        WISDM_ZSCORE_STATS.stds,
        INPUT_SCALE,
        INPUT_ZERO_POINT,
    );

    let mut raw_stats = [AxisStats::new(); FEATURE_COUNT];
    let mut mps2_stats = [AxisStats::new(); FEATURE_COUNT];
    let mut z_stats = [AxisStats::new(); FEATURE_COUNT];
    let mut q_stats = [QuantStats::new(); FEATURE_COUNT];

    let mut next_sample_at = Instant::now() + SAMPLE_PERIOD;
    let mut sample_count = 0_u32;

    while sample_count < SAMPLE_LIMIT {
        busy_wait_until(next_sample_at);
        next_sample_at += SAMPLE_PERIOD;

        match sensor.read_accel(&mut i2c) {
            Ok(accel) => {
                sample_count += 1;
                for axis in 0..FEATURE_COUNT {
                    let raw = accel.xyz[axis] as f32;
                    let mps2 = raw_accel_to_mps2(accel.xyz[axis]);
                    let z = (mps2 - WISDM_ZSCORE_STATS.means[axis])
                        / WISDM_ZSCORE_STATS.stds[axis];
                    let q = quantize_scalar(z, INPUT_SCALE, INPUT_ZERO_POINT);

                    raw_stats[axis].push(raw);
                    mps2_stats[axis].push(mps2);
                    z_stats[axis].push(z);
                    q_stats[axis].push(q);
                }

                if sample_count % SAMPLE_RATE_HZ == 0 {
                    info!("collected samples={}", sample_count);
                }
            }
            Err(err) => {
                info!(
                    "mpu6050 accel read error after {} samples: {}",
                    sample_count,
                    err,
                );
            }
        }
    }

    info!("mpu domain-shift probe summary");
    for axis in 0..FEATURE_COUNT {
        log_axis_stats("raw_lsb", axis, raw_stats[axis]);
        log_axis_stats("mps2", axis, mps2_stats[axis]);
        log_axis_stats("zscore_vs_wisdm", axis, z_stats[axis]);
        info!(
            "quant_i8 axis{}: min={}, max={}, sat_min={}, sat_max={}",
            axis,
            q_stats[axis].min,
            q_stats[axis].max,
            q_stats[axis].sat_min,
            q_stats[axis].sat_max,
        );
    }

    let mut led_on = false;
    loop {
        led_on = !led_on;
        if led_on {
            led.set_high();
        } else {
            led.set_low();
        }
        busy_wait(IDLE_DELAY);
    }
}
