#![cfg_attr(not(test), no_std)]

use defmt::Format;
use embedded_hal::blocking::i2c;

/// MCP9601 I2C address
const ADDRESS: u8 = 0x61;//FIXME: check the dividers from the schematic for the actual address

/// MCP9601 I2C registers
const HOT_JUNCTION_REG: u8 = 0x00;
const DELTA_JUNCTION_REG: u8 = 0x01;
const COLD_JUNCTION_REG: u8 = 0x02;
const ADC_RAW_START_REG: u8 = 0x03;
const STATUS_REG: u8 = 0x04;
const THEMOCOUPLE_CONFIG_REG: u8 = 0x05;
const DEVICE_CONFIG_REG: u8 = 0x06;
const DEVICE_ID_REG: u8 = 0x20;

#[derive(Clone, Copy, Format)]
pub struct SensorData {
    pub remote_temperature_c: f32,
    pub local_temperature_c: f32,
}

/// A MCP9601 sensor on the I2C bus `I`
pub struct Mcp9601<I>(I)
where
    I: i2c::Read + i2c::Write;

/// A driver error
#[derive(Debug, PartialEq)]
pub enum Error<E> {
    /// I2C bus error
    I2c(E),
    /// wrong sensor
    WrongSensor,
    /// wire fault
    WiringFault,
    /// local out-of-range fault
    ColdJunctionFault,
    ConfigFault,
}

impl<E, I> Mcp9601<I>
where
    I: i2c::Read<Error = E> + i2c::Write<Error = E>,
{
    /// Initializes the MCP9601 driver.
    /// This consumes the I2C bus `I`
    pub fn init(i2c: I) -> Self {
        Mcp9601(i2c)
    }
    
    /// self-test
    pub fn self_check(&mut self) -> Result<(), Error<E>> {
        // first check the ID register
    	let mut rd_buffer = [0u8;1];
    	self.0.write(ADDRESS, &[DEVICE_ID_REG]).map_err(Error::I2c)?;
        self.0.read(ADDRESS, &mut rd_buffer).map_err(Error::I2c)?;
        
        match rd_buffer[0] {
        	0x40 => {}, //MCP9600
        	0x41 => {}, //MCP9601
        	_ => return Err(Error::WrongSensor),
        }
        
        // now check status register
    	self.0.write(ADDRESS, &[STATUS_REG]).map_err(Error::I2c)?;
        self.0.read(ADDRESS, &mut rd_buffer).map_err(Error::I2c)?;
        
        if (rd_buffer[0] & 0x30) != 0 {
            return Err(Error::WiringFault);
        }
        
        // and finally the local temperature
       	let mut t_rd_buffer = [0u8;2];
    	self.0.write(ADDRESS, &[COLD_JUNCTION_REG]).map_err(Error::I2c)?;
        self.0.read(ADDRESS, &mut t_rd_buffer).map_err(Error::I2c)?;
        // reassemble temperature
        let value = (i16::from_be_bytes(t_rd_buffer) as f32)*0.0625;

        
        if value > 85.0 || value < -40.0 {
            return Err(Error::ColdJunctionFault);
        }
        Ok(())  
    }
    
    /// get thermocouple temperature
    pub fn get_hot_junction(&mut self) -> Result<f32, Error<E>> {
        let mut t_rd_buffer = [0u8;2];
    	self.0.write(ADDRESS, &[HOT_JUNCTION_REG]).map_err(Error::I2c)?;
        self.0.read(ADDRESS, &mut t_rd_buffer).map_err(Error::I2c)?;
        // reassemble temperature
        let value = (i16::from_be_bytes(t_rd_buffer) as f32)*0.0625;

        Ok(value)
    }

    /// configure
    pub fn set_sensor(&mut self,thermocouple_type: char) -> Result<(), Error<E>> {
        // Thermocouple type
        let mut wr_buffer: [u8;2]=[THEMOCOUPLE_CONFIG_REG,0];
        match thermocouple_type {
          'K' => {wr_buffer[1] = 0x00;},
          'J' => {wr_buffer[1] = 0x10;},
          'T' => {wr_buffer[1] = 0x20;},
          'N' => {wr_buffer[1] = 0x30;},
          'S' => {wr_buffer[1] = 0x40;},
          'E' => {wr_buffer[1] = 0x50;},
          'B' => {wr_buffer[1] = 0x60;},
          'R' => {wr_buffer[1] = 0x70;},
          _   => {return Err(Error::ConfigFault);},
        }
        self.0.write(ADDRESS, &wr_buffer).map_err(Error::I2c)?;
        let wr_buffer: [u8;2]=[DEVICE_CONFIG_REG,0];
        self.0.write(ADDRESS, &wr_buffer).map_err(Error::I2c)?;
        Ok(())  
    }
    
    /// Destroys this driver and releases the I2C bus `I`
    pub fn destroy(self) -> I {
        self.0
    }
}


#[cfg(test)]
mod tests {
    use super::{Error, Mcp9601, ADDRESS,HOT_JUNCTION_REG};
    use embedded_hal_mock::i2c;

    // check temperature calculation from registers is done correctly
    #[test] 
    fn get_hot_junction() {
        let expectations = vec![
            i2c::Transaction::write(ADDRESS, vec![HOT_JUNCTION_REG]),
            i2c::Transaction::read(ADDRESS, vec![0x00,0x10]),
        ];
        let mock = i2c::Mock::new(&expectations);

        let mut mcp = Mcp9601::init(mock);
        let temperature = mcp.get_hot_junction().unwrap();
        assert!((temperature-1.0).abs() < 0.001);

        let mut mock = mcp.destroy();
        mock.done(); // verify expectations
    }

}
