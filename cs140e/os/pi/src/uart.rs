use core::fmt;

use volatile::prelude::*;
use volatile::{Volatile, ReadVolatile, Reserved};

use timer;
use common::IO_BASE;
use gpio::{Gpio, Function};

/// The base address for the `MU` registers.
const MU_REG_BASE: usize = IO_BASE + 0x215040;

/// The `AUXENB` register from page 9 of the BCM2837 documentation.
const AUX_ENABLES: *mut Volatile<u8> = (IO_BASE + 0x215004) as *mut Volatile<u8>;

/// Enum representing bit fields of the `AUX_MU_LSR_REG` register.
#[repr(u8)]
enum LsrStatus {
    DataReady = 1,
    TxAvailable = 1 << 5,
}

#[repr(C)]
#[allow(non_snake_case)]
struct Registers {
    // FIXME: Declare the "MU" registers from page 8.
    AUX_MU_IO_REG: Volatile<u32>,
    AUX_MU_IER_REG : Volatile<u32>,
    AUX_MU_IIR_REG : Volatile<u32>,
    AUX_MU_LCR_REG : Volatile<u32>,
    AUX_MU_MCR_REG : Volatile<u32>,
    AUX_MU_LSR_REG : Volatile<u32>,
    AUX_MU_MSR_REG : Volatile<u32>,
    AUX_MU_SCRATCH : Volatile<u32>,
    AUX_MU_CNTL_REG : Volatile<u32>,
    AUX_MU_STAT_REG : Volatile<u32>,
    AUX_MU_BAUD_REG : Volatile<u32>,
}

/// The Raspberry Pi's "mini UART".
pub struct MiniUart {
    registers: &'static mut Registers,
    timeout: Option<u32>,
}

impl MiniUart {
    /// Initializes the mini UART by enabling it as an auxiliary peripheral,
    /// setting the data size to 8 bits, setting the BAUD rate to ~115200 (baud
    /// divider of 270), setting GPIO pins 14 and 15 to alternative function 5
    /// (TXD1/RDXD1), and finally enabling the UART transmitter and receiver.
    ///
    /// By default, reads will never time out. To set a read timeout, use
    /// `set_read_timeout()`.
    pub fn new() -> MiniUart {
        let registers = unsafe {
            // Enable the mini UART as an auxiliary device.
            (*AUX_ENABLES).or_mask(1);
            &mut *(MU_REG_BASE as *mut Registers)
        };

        // 8-bit mode
        registers.AUX_MU_LCR_REG.write(3);

        // Baud Rate: 115200
        registers.AUX_MU_BAUD_REG.write(270);

        // Set GPIO14+15 to ALT5
        Gpio::new(14).into_alt(Function::Alt5);
        Gpio::new(15).into_alt(Function::Alt5);

        // Start UART tx + rx
        registers.AUX_MU_CNTL_REG.write(3);

        MiniUart {
            registers: registers,
            timeout: None,
        }
    }

    /// Set the read timeout to `milliseconds` milliseconds.
    pub fn set_read_timeout(&mut self, milliseconds: u32) {
        self.timeout = Some(milliseconds);
    }

    /// Write the byte `byte`. This method blocks until there is space available
    /// in the output FIFO.
    pub fn write_byte(&mut self, byte: u8) {
        while !self.registers.AUX_MU_LSR_REG.has_mask(LsrStatus::TxAvailable as u32) {}

        self.registers.AUX_MU_IO_REG.write(byte as u32);
    }

    /// Returns `true` if there is at least one byte ready to be read. If this
    /// method returns `true`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately. This method does not block.
    pub fn has_byte(&self) -> bool {
        self.registers.AUX_MU_LSR_REG.has_mask(LsrStatus::DataReady as u32)
    }

    /// Blocks until there is a byte ready to read. If a read timeout is set,
    /// this method blocks for at most that amount of time. Otherwise, this
    /// method blocks indefinitely until there is a byte to read.
    ///
    /// Returns `Ok(())` if a byte is ready to read. Returns `Err(())` if the
    /// timeout expired while waiting for a byte to be ready. If this method
    /// returns `Ok(())`, a subsequent call to `read_byte` is guaranteed to
    /// return immediately.
    pub fn wait_for_byte(&self) -> Result<(), ()> {
        let start_time: u64 = timer::current_time();

        while !self.has_byte() {
            // Check for timeout.
            if let Some(duration) = self.timeout {
                if timer::current_time() > start_time + (duration as u64) * 1000 {
                    return Err(());
                }
            }
        }
		Ok(())
    }

    /// Reads a byte. Blocks indefinitely until a byte is ready to be read.
    pub fn read_byte(&mut self) -> u8 {
        while !self.has_byte() {}

        (self.registers.AUX_MU_IO_REG.read() & 0xFF) as u8
    }
}

impl fmt::Write for MiniUart {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let bytes: &[u8] = s.as_bytes();
        for &byte in bytes {
            // Write a \r before a \n.
            if byte == b'\n' {
                self.write_byte(b'\r');
            }

            self.write_byte(byte);
        }
		Ok(())
    }
}

#[cfg(feature = "std")]
mod uart_io {
    use std::io;
    use super::MiniUart;

    // FIXME: Implement `io::Read` and `io::Write` for `MiniUart`.
    //
    // The `io::Read::read()` implementation must respect the read timeout by
    // waiting at most that time for the _first byte_. It should not wait for
    // any additional bytes but _should_ read as many bytes as possible. If the
    // read times out, an error of kind `TimedOut` should be returned.
	impl io::Read for MiniUart {
		fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
			if self.wait_for_byte().is_err() {
                Err(io::Error::new(io::ErrorKind::TimedOut, "Timeout Error!"))
            } else {
                let mut total_read = 0;
                while self.has_byte() && total_read < buf.len() {
                    buf[total_read] = self.read_byte();
                    total_read += 1;
                }
                Ok(total_read)
            }
		}
	}
    // The `io::Write::write()` method must write all of the requested bytes
    // before returning.
    impl io::Write for MiniUart {
        fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
            for &byte in buf {
                self.write_byte(byte);
            }
            Ok(buf.len())
        }

        fn flush(&mut self) -> io::Result<()>{
            Ok(())
        }
    }
}
