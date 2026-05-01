use esp_hal::{
    DriverMode,
    i2c::master::{Error as I2cError, I2c},
};

pub const DEFAULT_ADDRESS: u8 = 0x68;
pub const ALT_ADDRESS: u8 = 0x69;

const REG_PWR_MGMT_1: u8 = 0x6B;
const REG_WHO_AM_I: u8 = 0x75;
const REG_ACCEL_XOUT_H: u8 = 0x3B;
const WAKE_UP: u8 = 0x01;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawSample {
    pub accel: [i16; 3],
    pub temp_raw: i16,
    pub gyro: [i16; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RawAccel {
    pub xyz: [i16; 3],
}

pub struct Mpu6050 {
    address: u8,
}

impl Mpu6050 {
    pub const fn new(address: u8) -> Self {
        Self { address }
    }

    pub const fn address(&self) -> u8 {
        self.address
    }

    pub fn init<'d, Dm: DriverMode>(&self, i2c: &mut I2c<'d, Dm>) -> Result<u8, I2cError> {
        let who_am_i = self.read_who_am_i(i2c)?;
        self.write_register(i2c, REG_PWR_MGMT_1, WAKE_UP)?;
        Ok(who_am_i)
    }

    pub fn read_who_am_i<'d, Dm: DriverMode>(
        &self,
        i2c: &mut I2c<'d, Dm>,
    ) -> Result<u8, I2cError> {
        self.read_register(i2c, REG_WHO_AM_I)
    }

    pub fn read_sample<'d, Dm: DriverMode>(
        &self,
        i2c: &mut I2c<'d, Dm>,
    ) -> Result<RawSample, I2cError> {
        let mut raw = [0_u8; 14];
        i2c.write_read(self.address, &[REG_ACCEL_XOUT_H], &mut raw)?;

        Ok(RawSample {
            accel: [
                i16::from_be_bytes([raw[0], raw[1]]),
                i16::from_be_bytes([raw[2], raw[3]]),
                i16::from_be_bytes([raw[4], raw[5]]),
            ],
            temp_raw: i16::from_be_bytes([raw[6], raw[7]]),
            gyro: [
                i16::from_be_bytes([raw[8], raw[9]]),
                i16::from_be_bytes([raw[10], raw[11]]),
                i16::from_be_bytes([raw[12], raw[13]]),
            ],
        })
    }

    pub fn read_accel<'d, Dm: DriverMode>(
        &self,
        i2c: &mut I2c<'d, Dm>,
    ) -> Result<RawAccel, I2cError> {
        let mut raw = [0_u8; 6];
        i2c.write_read(self.address, &[REG_ACCEL_XOUT_H], &mut raw)?;

        Ok(RawAccel {
            xyz: [
                i16::from_be_bytes([raw[0], raw[1]]),
                i16::from_be_bytes([raw[2], raw[3]]),
                i16::from_be_bytes([raw[4], raw[5]]),
            ],
        })
    }

    fn read_register<'d, Dm: DriverMode>(
        &self,
        i2c: &mut I2c<'d, Dm>,
        register: u8,
    ) -> Result<u8, I2cError> {
        let mut value = [0_u8; 1];
        i2c.write_read(self.address, &[register], &mut value)?;
        Ok(value[0])
    }

    fn write_register<'d, Dm: DriverMode>(
        &self,
        i2c: &mut I2c<'d, Dm>,
        register: u8,
        value: u8,
    ) -> Result<(), I2cError> {
        i2c.write(self.address, &[register, value])
    }
}
