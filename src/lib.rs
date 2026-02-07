#![no_std]
#![no_main]
#![doc = include_str!("../README.md")]
#![warn(missing_docs)]

use embedded_hal_async::digital as digital_async;
use embedded_hal_async::i2c::I2c;
use embedded_hal_async::delay::DelayNs;

use crate::config::BMA425_CONFIG_FILE;
use crate::registers::{InterruptMap, Register};

mod config;
pub mod registers;

const BMA425_CHIP_ID: u8 = 0x13;
const PRIMARY_I2C_ADDRESS: u8 = 0x18;
const SECONDARY_I2C_ADDRESS: u8 = 0x19;

/// BMA425 driver errors
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum Error<E> {
    /// I2C communication error
    I2c(E),
    /// The chip ID read from the sensor was not 0x13
    InvalidChipId(u8),
    /// Initialization error
    InitializationError(u8),
    /// Interrupt pin error
    InterruptPin(E),
}



/// Driver for the BMA425 accelerometer
pub struct BMA425<I2C, InterruptPin> {
    address: u8,
    i2c: I2C,
    interrupt_pin: InterruptPin,
}

impl<I2C, InterruptPin, E, E2> BMA425<I2C, InterruptPin>
where 
    I2C: I2c<Error = E>,
    InterruptPin: digital_async::Wait<Error = E2>,
{
    /// Create a new BMA425 driver instance with the default I2C address: 0x18
    pub fn new(
        i2c: I2C,
        interrupt_pin: InterruptPin,
    ) -> Self {
        BMA425 {
            address: PRIMARY_I2C_ADDRESS,
            i2c,
            interrupt_pin,
        }
    }

    /// Create a new BMA425 driver instance with the alternate I2C address: 0x19
    pub fn new_using_alt_address(
        i2c: I2C,
        interrupt_pin: InterruptPin,
    ) -> Self {
        BMA425 {
            address: SECONDARY_I2C_ADDRESS,
            i2c,
            interrupt_pin,
        }
    }

    /// Initialize the BMA425 sensor
    pub async fn init(&mut self, delay: &mut impl DelayNs) -> Result<(), Error<E>> {
        let mut buffer = [0u8; 1];
        self.write_read(Register::ChipId, &mut buffer).await.map_err(Error::I2c)?;
        if buffer[0] != BMA425_CHIP_ID {
            return Err(Error::InvalidChipId(buffer[0]));
        }
        self.write(Register::PwrConf, 0x00).await.map_err(Error::I2c)?;
        delay.delay_us(500).await;
        self.write(Register::InitCtrl, 0x00).await.map_err(Error::I2c)?;
        self.write_config_file().await.map_err(Error::I2c)?;
        self.write(Register::InitCtrl, 0x01).await.map_err(Error::I2c)?;
        delay.delay_ms(150).await;
        self.write_read(Register::InternalStatus, &mut buffer).await.map_err(Error::I2c)?;
        if buffer[0] & 0x01 == 0 {
            return Err(Error::InitializationError(buffer[0]));
        }
        Ok(())
    }

    pub async fn test(&mut self) -> Result<(), I2C::Error> {
        self.write(Register::PwrCtrl, 0x04).await?;
        self.write(Register::AccConf, 0x17).await?;
        self.write(Register::PwrConf, 0x03).await?;
        Ok(())
    }

    pub async fn get_accel_data(&mut self) -> Result<(i16, i16, i16), I2C::Error> {
        let mut buffer = [0u8; 6];
        self.write_read(Register::Data8, &mut buffer).await?;
        let x = self.convert_accel_data(buffer[0], buffer[1]);
        let y = self.convert_accel_data(buffer[2], buffer[3]);
        let z = self.convert_accel_data(buffer[4], buffer[5]);
        Ok((x, y, z))
    }

    fn convert_accel_data(&self, lsb: u8, msb: u8) -> i16 {
        let lsb: u16 = (lsb as u16 & 0xF0) >> 4;
        let msb: u16 = (msb as u16) << 4;
        let value: u16 = msb | lsb;
        ((value << 4) as i16) >> 4 // Convert from 12-bit two's complement to i16
    }

    pub async fn read_features(&mut self) -> Result<[u8; 6144], I2C::Error> {
        const READ_WRITE_LENGTH: usize = 8;
        let mut data = [0u8; 6144];
        for (i, chunk) in data.chunks_mut(READ_WRITE_LENGTH).enumerate() {
            let index = i * READ_WRITE_LENGTH;

            // Write the index where the next chunk will be read
            let asic_msb: u8 = ((index / 2) >> 4) as u8;
            let asic_lsb: u8 = ((index / 2) & 0x0F) as u8;
            self.write(Register::AsicLsb, asic_lsb).await?;
            self.write(Register::AsicMsb, asic_msb).await?;

            self.write_read(Register::FeaturesIn, chunk).await?;
        }
        Ok(data)
    }

    pub async fn get_interrupt_map(&mut self) -> Result<u8, I2C::Error> {
        let mut buffer = [0u8; 1];
        self.write_read(Register::Int1Map, &mut buffer).await?;
        Ok(buffer[0])
    }

    pub async fn configure_interrupt1(&mut self, map: InterruptMap) -> Result<(), I2C::Error> {
        self.write(Register::Int1Map, map as u8).await?;
        Ok(())
    }

    pub async fn wait_for_interrupt(&mut self) -> Result<(), InterruptPin::Error> {
        self.interrupt_pin.wait_for_high().await?;
        Ok(())
    }

    async fn write_raw(&mut self, data: &[u8]) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, data).await?;
        Ok(())
    }

    async fn write(&mut self, register: Register, data: u8) -> Result<(), I2C::Error> {
        self.i2c.write(self.address, &[register as u8, data]).await?;
        Ok(())
    }

    async fn write_read(&mut self, register: Register, read: &mut [u8]) -> Result<(), I2C::Error> {
        self.i2c.write_read(self.address, &[register as u8], read).await?;
        Ok(())
    }

    async fn write_config_file(&mut self) -> Result<(), I2C::Error> {
        const READ_WRITE_LENGTH: usize = 8;
        let data = BMA425_CONFIG_FILE.as_slice();
        for (i, chunk) in data.chunks(READ_WRITE_LENGTH).enumerate() {
            let index = i * READ_WRITE_LENGTH;
            let mut buf: [u8; 9] = [0; 9];
            buf[0] = Register::FeaturesIn as u8;
            buf[1..1 + chunk.len()].copy_from_slice(chunk);

            // Write the index where the next chunk will be written
            let asic_msb: u8 = ((index / 2) >> 4) as u8;
            let asic_lsb: u8 = ((index / 2) & 0x0F) as u8;
            self.write(Register::AsicLsb, asic_lsb).await?;
            self.write(Register::AsicMsb, asic_msb).await?;

            self.write_raw(&buf).await?;
        }
        Ok(())
    }
}