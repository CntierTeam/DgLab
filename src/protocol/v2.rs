//! Coyote 2.0 BLE characteristic packing (little-endian bitfields).
//!
//! Strength / wave layout matches DGLAB-BT write path.
//! Strength decode uses correct 11-bit unpack (not the broken GetStrength.py byte/7).

use uuid::Uuid;

/// Channel A or B.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Channel {
    A,
    B,
}

impl Channel {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "A" | "a" | "1" => Some(Channel::A),
            "B" | "b" | "2" => Some(Channel::B),
            _ => None,
        }
    }
}

/// V2 GATT UUIDs and codecs.
pub struct V2;

impl V2 {
    pub const SERVICE: Uuid = Uuid::from_u128(0x955a180a_0fe2_f5aa_a094_84b8d4f3e8ad);
    pub const BATTERY: Uuid = Uuid::from_u128(0x955a1500_0fe2_f5aa_a094_84b8d4f3e8ad);
    pub const STRENGTH: Uuid = Uuid::from_u128(0x955a1504_0fe2_f5aa_a094_84b8d4f3e8ad);
    pub const WAVE_B: Uuid = Uuid::from_u128(0x955a1505_0fe2_f5aa_a094_84b8d4f3e8ad);
    pub const WAVE_A: Uuid = Uuid::from_u128(0x955a1506_0fe2_f5aa_a094_84b8d4f3e8ad);

    pub fn wave_uuid(channel: Channel) -> Uuid {
        match channel {
            Channel::A => Self::WAVE_A,
            Channel::B => Self::WAVE_B,
        }
    }

    /// Pack A/B strength (0..=2047 after *7 clamp to 11 bits) into 3 LE bytes.
    pub fn encode_strength(a: u16, b: u16) -> [u8; 3] {
        let a_bits = ((a as u32) * 7) & 0x7FF;
        let b_bits = (((b as u32) * 7) & 0x7FF) << 11;
        let packed = b_bits | a_bits;
        [
            (packed & 0xFF) as u8,
            ((packed >> 8) & 0xFF) as u8,
            ((packed >> 16) & 0xFF) as u8,
        ]
    }

    /// Unpack strength from 3 LE bytes using 11-bit fields / 7.
    pub fn decode_strength(data: &[u8]) -> Option<(u16, u16)> {
        if data.len() < 3 {
            return None;
        }
        let packed =
            u32::from(data[0]) | (u32::from(data[1]) << 8) | (u32::from(data[2]) << 16);
        let a = ((packed & 0x7FF) / 7) as u16;
        let b = (((packed >> 11) & 0x7FF) / 7) as u16;
        Some((a, b))
    }

    /// Pack wave x(5)/y(10)/z(5) into 3 LE bytes.
    pub fn encode_wave(x: u8, y: u16, z: u8) -> [u8; 3] {
        let x_bits = u32::from(x) & 0x1F;
        let y_bits = (u32::from(y) & 0x3FF) << 5;
        let z_bits = (u32::from(z) & 0x1F) << 15;
        let packed = z_bits | y_bits | x_bits;
        [
            (packed & 0xFF) as u8,
            ((packed >> 8) & 0xFF) as u8,
            ((packed >> 16) & 0xFF) as u8,
        ]
    }

    #[allow(dead_code)]
    pub fn decode_wave(data: &[u8]) -> Option<(u8, u16, u8)> {
        if data.len() < 3 {
            return None;
        }
        let packed =
            u32::from(data[0]) | (u32::from(data[1]) << 8) | (u32::from(data[2]) << 16);
        let x = (packed & 0x1F) as u8;
        let y = ((packed >> 5) & 0x3FF) as u16;
        let z = ((packed >> 15) & 0x1F) as u8;
        Some((x, y, z))
    }

    #[allow(dead_code)]
    pub fn decode_battery(data: &[u8]) -> Option<u8> {
        data.first().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn strength_roundtrip_and_known_vectors() {
        // a=10, b=20 -> ABits=70, BBits=140<<11
        assert_eq!(V2::encode_strength(10, 20), hex!("466004"));
        assert_eq!(V2::decode_strength(&hex!("466004")), Some((10, 20)));

        assert_eq!(V2::encode_strength(0, 0), hex!("000000"));
        assert_eq!(V2::decode_strength(&hex!("000000")), Some((0, 0)));

        // a=100, b=0 -> 700 = 0x2BC
        assert_eq!(V2::encode_strength(100, 0), hex!("bc0200"));
        assert_eq!(V2::decode_strength(&hex!("bc0200")), Some((100, 0)));
    }

    #[test]
    fn wave_roundtrip_and_known_vectors() {
        // x=5, y=135, z=20 (README example sendWave) → 0xA10E5 LE
        assert_eq!(V2::encode_wave(5, 135, 20), hex!("e5100a"));
        assert_eq!(V2::decode_wave(&hex!("e5100a")), Some((5, 135, 20)));

        assert_eq!(V2::encode_wave(0, 0, 0), hex!("000000"));
        assert_eq!(V2::decode_wave(&hex!("000000")), Some((0, 0, 0)));
    }

    #[test]
    fn battery_first_byte() {
        assert_eq!(V2::decode_battery(&[87, 0]), Some(87));
        assert_eq!(V2::decode_battery(&[]), None);
    }
}
