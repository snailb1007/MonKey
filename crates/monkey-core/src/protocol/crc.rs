use crc::{Crc, CRC_16_MODBUS, CRC_16_IBM_SDLC};

/// CRC16-MODBUS calculator (Shenzhen HFD vendor protocol variant).
pub const CRC_MODBUS: Crc<u16> = Crc::<u16>::new(&CRC_16_MODBUS);

/// CRC16-CCITT / IBM-SDLC calculator.
pub const CRC_CCITT: Crc<u16> = Crc::<u16>::new(&CRC_16_IBM_SDLC);

/// Calculate CRC16 checksum for buffer using MODBUS polynomial.
pub fn calculate_crc16(data: &[u8]) -> u16 {
    CRC_MODBUS.checksum(data)
}

/// Verify CRC16 checksum against expected value.
pub fn verify_crc16(data: &[u8], expected: u16) -> bool {
    calculate_crc16(data) == expected
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_crc16_calculation() {
        let test_data = b"123456789";
        let crc = calculate_crc16(test_data);
        assert_eq!(crc, 0x4B37); // Standard MODBUS check value for 123456789
        assert!(verify_crc16(test_data, 0x4B37));
        assert!(!verify_crc16(test_data, 0x1234));
    }
}
