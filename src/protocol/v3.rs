//! Coyote 3.0 B0 / B1 / BF codecs (big-endian protocol, no host endian swap).
//!
//! Layout from DG-LAB-OPENSOURCE coyote/v3 README.
//! B0 frequency compression + hex vectors cross-checked against dungeonctl tests.

use uuid::Uuid;

/// How B0 interprets the A/B strength bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
#[allow(dead_code)]
pub enum IntensityMode {
    DoNotChange = 0b00,
    RelativeIncrease = 0b01,
    RelativeDecrease = 0b10,
    Absolute = 0b11,
}

/// One 25 ms pulse slot (frequency in Hz, intensity 0..=100).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Pulse {
    pub frequency_hz: u8,
    pub intensity: u8,
}

impl Pulse {
    pub const IDLE: Pulse = Pulse {
        frequency_hz: 10,
        intensity: 0,
    };

    /// Compress Hz → wire frequency byte (10..=240) via period = 1000/f.
    pub fn compressed_frequency(self) -> u8 {
        if self.frequency_hz == 0 {
            return 0;
        }
        let t = 1000.0 / f32::from(self.frequency_hz);
        let compressed = match t {
            t if t < 5.0 => 5.0,
            t if t < 100.0 => t,
            t if t < 600.0 => (t - 100.0) / 5.0 + 100.0,
            t if t < 1000.0 => (t - 600.0) / 10.0 + 200.0,
            _ => 240.0,
        };
        compressed as u8
    }

    #[allow(dead_code)]
    pub fn clamped_intensity(self) -> u8 {
        self.intensity.min(100)
    }
}

/// V3 GATT UUIDs and packet builders.
pub struct V3;

impl V3 {
    pub const SERVICE: Uuid = Uuid::from_u128(0x0000180c_0000_1000_8000_00805f9b34fb);
    pub const WRITE: Uuid = Uuid::from_u128(0x0000150a_0000_1000_8000_00805f9b34fb);
    pub const NOTIFY: Uuid = Uuid::from_u128(0x0000150b_0000_1000_8000_00805f9b34fb);
    #[allow(dead_code)]
    pub const BATTERY_SERVICE: Uuid = Uuid::from_u128(0x0000180a_0000_1000_8000_00805f9b34fb);
    pub const BATTERY: Uuid = Uuid::from_u128(0x00001500_0000_1000_8000_00805f9b34fb);

    /// Build 20-byte B0: strength change + 4×25ms pulses for both channels.
    pub fn encode_b0(
        seq: u8,
        mode_a: IntensityMode,
        mode_b: IntensityMode,
        strength_a: u8,
        strength_b: u8,
        pulses_a: [Pulse; 4],
        pulses_b: [Pulse; 4],
    ) -> [u8; 20] {
        let mode = ((mode_a as u8) << 2) | (mode_b as u8);
        let mut out = [0u8; 20];
        out[0] = 0xB0;
        out[1] = ((seq & 0x0F) << 4) | (mode & 0x0F);
        out[2] = if strength_a <= 200 { strength_a } else { 0 };
        out[3] = if strength_b <= 200 { strength_b } else { 0 };
        for i in 0..4 {
            out[4 + i] = pulses_a[i].compressed_frequency();
            // Intensity is written raw: 0..=100 active; >100 disables that channel (official).
            out[8 + i] = pulses_a[i].intensity;
            out[12 + i] = pulses_b[i].compressed_frequency();
            out[16 + i] = pulses_b[i].intensity;
        }
        out
    }

    /// Absolute strength zero B0 with idle pulses (emergency stop).
    pub fn encode_emergency_zero(seq: u8) -> [u8; 20] {
        let idle = [Pulse::IDLE; 4];
        // Make B channel invalid intensity so only A is “valid” idle — both idle is fine.
        Self::encode_b0(
            seq,
            IntensityMode::Absolute,
            IntensityMode::Absolute,
            0,
            0,
            idle,
            idle,
        )
    }

    /// BF soft limits + balance params (7 bytes). Re-send after every reconnect.
    pub fn encode_bf(
        limit_a: u8,
        limit_b: u8,
        freq_balance_a: u8,
        freq_balance_b: u8,
        intensity_balance_a: u8,
        intensity_balance_b: u8,
    ) -> [u8; 7] {
        [
            0xBF,
            limit_a.min(200),
            limit_b.min(200),
            freq_balance_a,
            freq_balance_b,
            intensity_balance_a,
            intensity_balance_b,
        ]
    }

    /// Parse B1 notify: `[0xB1, seq, a, b]`.
    pub fn decode_b1(data: &[u8]) -> Option<(u8, u8, u8)> {
        if data.len() < 4 || data[0] != 0xB1 {
            return None;
        }
        Some((data[1], data[2], data[3]))
    }

    #[allow(dead_code)]
    pub fn decode_battery(data: &[u8]) -> Option<u8> {
        data.first().copied()
    }

    /// Simple repeating pulse pattern for TUI / sendWave mapping.
    pub fn pattern_from_xyz(x: u8, y: u16, z: u8) -> ([Pulse; 4], [Pulse; 4]) {
        let freq = x.clamp(1, 100);
        let intensity = ((y.min(100)) as u8).min(100);
        // z unused for V3 mapping beyond keeping a gentle pattern
        let _ = z;
        let a = [Pulse {
            frequency_hz: freq,
            intensity,
        }; 4];
        // Invalid B intensity (>100) disables B channel output.
        let mut b = [Pulse::IDLE; 4];
        b[3].intensity = 101;
        (a, b)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use hex_literal::hex;

    #[test]
    fn b0_matches_dungeonctl_vectors() {
        let pulse_a = Pulse {
            frequency_hz: 100,
            intensity: 0,
        };
        let pulse_b = Pulse {
            frequency_hz: 30,
            intensity: 0,
        };
        let got = V3::encode_b0(
            0,
            IntensityMode::Absolute,
            IntensityMode::Absolute,
            10,
            0,
            [pulse_a; 4],
            [pulse_b; 4],
        );
        assert_eq!(got, hex!("b00f0a000a0a0a0a000000002121212100000000"));

        let pulse_a = Pulse {
            frequency_hz: 100,
            intensity: 100,
        };
        let pulse_b = Pulse {
            frequency_hz: 30,
            intensity: 100,
        };
        let got = V3::encode_b0(
            0,
            IntensityMode::Absolute,
            IntensityMode::Absolute,
            10,
            0,
            [pulse_a; 4],
            [pulse_b; 4],
        );
        assert_eq!(got, hex!("b00f0a000a0a0a0a646464642121212164646464"));
    }

    #[test]
    fn b0_official_idle_a_only_hex() {
        // Official No.1 #1 wire freqs already compressed (10); 100Hz → compressed 10.
        let a = [
            Pulse {
                frequency_hz: 100,
                intensity: 0,
            },
            Pulse {
                frequency_hz: 100,
                intensity: 10,
            },
            Pulse {
                frequency_hz: 50,
                intensity: 20,
            },
            Pulse {
                frequency_hz: 33,
                intensity: 30,
            },
        ];
        let mut b = [Pulse {
            frequency_hz: 10,
            intensity: 0,
        }; 4];
        b[3].intensity = 101;
        let pkt = V3::encode_b0(
            0,
            IntensityMode::DoNotChange,
            IntensityMode::DoNotChange,
            0,
            0,
            a,
            b,
        );
        // 100Hz→10, 50Hz→20, 33Hz→30 (1000/33≈30.3)
        assert_eq!(pkt[0], 0xB0);
        assert_eq!(pkt[1], 0x00);
        assert_eq!(&pkt[4..8], &[10, 10, 20, 30]);
        assert_eq!(&pkt[8..12], &[0, 10, 20, 30]);
        assert_eq!(pkt[19], 101);
    }

    #[test]
    fn bf_and_b1() {
        assert_eq!(
            V3::encode_bf(70, 70, 160, 160, 0, 0),
            hex!("bf4646a0a00000")
        );
        assert_eq!(V3::decode_b1(&hex!("b1010a14")), Some((1, 10, 20)));
        assert_eq!(V3::decode_b1(&hex!("b0")), None);
    }

    #[test]
    fn relative_modes_nibble() {
        let idle = [Pulse::IDLE; 4];
        let pkt = V3::encode_b0(
            2,
            IntensityMode::RelativeIncrease,
            IntensityMode::RelativeDecrease,
            5,
            8,
            idle,
            idle,
        );
        // seq=2, mode = (0b01<<2)|0b10 = 0b0110 → byte1 = 0x26
        assert_eq!(pkt[1], 0x26);
        assert_eq!(pkt[2], 5);
        assert_eq!(pkt[3], 8);
        assert_eq!(Pulse::IDLE.clamped_intensity(), 0);
    }

    #[test]
    fn emergency_zero_absolute() {
        let pkt = V3::encode_emergency_zero(1);
        assert_eq!(pkt[0], 0xB0);
        assert_eq!(pkt[1] & 0x0F, 0x0F); // both absolute
        assert_eq!(pkt[1] >> 4, 1);
        assert_eq!(pkt[2], 0);
        assert_eq!(pkt[3], 0);
    }
}
