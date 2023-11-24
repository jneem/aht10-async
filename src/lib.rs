//! A platform agnostic driver to interface with the AHT10 temperature and humidity sensor.
//!
//! This driver was built using [`embedded-hal-async`] traits.
//!
//! Unfortunately, the AHT10 datasheet is somewhat underspecified. There's a
//! FIFO mode as well as command data bytes which are briefly mentioned, but
//! I've found no documentation describing what they mean. Caveat emptor.
//!
//! [`embedded-hal`]: https://docs.rs/embedded-hal/~0.2

#![deny(missing_docs)]
#![no_std]

use embedded_hal_async::delay::DelayUs;
use embedded_hal_async::i2c::I2c;

const I2C_ADDRESS: u8 = 0x38;

#[derive(Copy, Clone)]
#[repr(u8)]
enum Command {
    Calibrate = 0b1110_0001,
    GetRaw = 0b1010_1000,
    GetCT = 0b1010_1100,
    Reset = 0b1011_1010,
}

#[macro_use]
extern crate bitflags;

bitflags! {
    struct StatusFlags: u8 {
        const BUSY = (1 << 7);
        const MODE = ((1 << 6) | (1 << 5));
        const CRC = (1 << 4);
        const CALIBRATION_ENABLE = (1 << 3);
        const FIFO_ENABLE = (1 << 2);
        const FIFO_FULL = (1 << 1);
        const FIFO_EMPTY = (1 << 0);
    }
}

/// AHT10 Error
#[derive(Debug, Copy, Clone)]
pub enum Error<E> {
    /// Device is not calibrated
    Uncalibrated(),
    /// Underlying bus error.
    BusError(E),
}

impl<E> core::convert::From<E> for Error<E> {
    fn from(e: E) -> Self {
        Error::BusError(e)
    }
}

/// AHT10 driver
pub struct AHT10<I2C, D> {
    i2c: I2C,
    delay: D,
}

/// Humidity reading from AHT10.
pub struct Humidity {
    h: u32,
}
impl Humidity {
    /// Humidity conveted to relative humidity.
    pub fn rh(&self) -> f32 {
        100.0 * (self.h as f32) / ((1 << 20) as f32)
    }
    /// Raw humidity reading.
    pub fn raw(&self) -> u32 {
        self.h
    }
}

/// Temperature reading from AHT10.
pub struct Temperature {
    t: u32,
}
impl Temperature {
    /// Temperature converted to celsius.
    pub fn celsius(&self) -> f32 {
        (200.0 * (self.t as f32) / ((1 << 20) as f32)) - 50.0
    }
    /// Raw temperature reading.
    pub fn raw(&self) -> u32 {
        self.t
    }
}

impl<I2C, D> AHT10<I2C, D>
where
    I2C: I2c,
    D: DelayUs,
{
    /// Creates a new AHT10 device from an I2C peripheral.
    pub async fn new(i2c: I2C, delay: D) -> Result<Self, I2C::Error> {
        let mut dev = AHT10 {
            i2c: i2c,
            delay: delay,
        };
        dev.write_cmd(Command::GetRaw, 0).await?;
        dev.delay.delay_ms(300).await;
        // MSB notes:
        // Bit 2 set => temperature is roughly doubled(?)
        // Bit 3 set => calibrated flag
        // Bit 4 => temperature is negative? (cyc mode?)
        dev.write_cmd(Command::Calibrate, 0x0800).await?;
        dev.delay.delay_ms(300).await;
        Ok(dev)
    }

    /// Soft reset the sensor.
    pub async fn reset(&mut self) -> Result<(), I2C::Error> {
        self.write_cmd(Command::Reset, 0).await?;
        self.delay.delay_ms(20).await;
        Ok(())
    }

    /// Read humidity and temperature.
    pub async fn read(&mut self) -> Result<(Humidity, Temperature), Error<I2C::Error>> {
        let buf: &mut [u8; 7] = &mut [0; 7];
        // Sort of reverse engineered the cmd data:
        // Bit 0 -> temperature calibration (0 => +0.5C)
        // Bit {1,2,3} -> refresh rate? (0 => slow refresh)
        self.i2c
            .write_read(I2C_ADDRESS, &[Command::GetCT as u8, 0b11111111, 0], buf).await?;
        let status = StatusFlags { bits: buf[0] };
        if !status.contains(StatusFlags::CALIBRATION_ENABLE) {
            return Err(Error::Uncalibrated());
        }
        let hum = ((buf[1] as u32) << 12) | ((buf[2] as u32) << 4) | ((buf[3] as u32) >> 4);
        let temp = (((buf[3] as u32) & 0x0f) << 16) | ((buf[4] as u32) << 8) | (buf[5] as u32);
        Ok((Humidity { h: hum }, Temperature { t: temp }))
    }

    async fn write_cmd(&mut self, cmd: Command, dat: u16) -> Result<(), I2C::Error> {
        self.i2c.write(
            I2C_ADDRESS,
            &[cmd as u8, (dat >> 8) as u8, (dat & 0xff) as u8],
        ).await
    }
}
