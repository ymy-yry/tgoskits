//! Feetech actuator protocol primitives and three-wheel base kinematics.
//!
//! This crate owns neither a serial port nor a device handle.  It provides
//! checked wire frames and deterministic command-to-wheel conversion so a
//! platform serial driver can retain transport and lifecycle ownership.

#![no_std]

/// STS/SMS protocol instruction code for writing a register.
pub const WRITE_INSTRUCTION: u8 = 0x03;

/// Builds a protocol-1 write frame for one 16-bit register value.
///
/// The returned frame is `[FF, FF, id, length, instruction, address, lo, hi,
/// checksum]`. The caller supplies the transport and is responsible for retry,
/// timeout and bus ownership.
pub fn write_u16_frame(id: u8, address: u8, value: u16) -> [u8; 9] {
    let [lo, hi] = value.to_le_bytes();
    let length = 5;
    let checksum = !(id
        .wrapping_add(length)
        .wrapping_add(WRITE_INSTRUCTION)
        .wrapping_add(address)
        .wrapping_add(lo)
        .wrapping_add(hi));
    [
        0xff,
        0xff,
        id,
        length,
        WRITE_INSTRUCTION,
        address,
        lo,
        hi,
        checksum,
    ]
}

/// Body-space velocity command for a three-wheel omnidirectional base.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BodyVelocity {
    pub forward: f32,
    pub lateral: f32,
    pub yaw: f32,
}

/// Velocity setpoints for the physical left, back and right wheels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WheelVelocities {
    pub left: f32,
    pub back: f32,
    pub right: f32,
}

/// Converts body-space velocity into the left/back/right wheel convention.
///
/// `yaw_gain` represents the calibrated wheel speed contribution of a unit
/// yaw request. Calibration and saturation belong to the platform controller.
pub fn three_omni_wheel_velocities(command: BodyVelocity, yaw_gain: f32) -> WheelVelocities {
    const HALF: f32 = 0.5;
    const SQRT_3_OVER_2: f32 = 0.866_025_4;
    let yaw = command.yaw * yaw_gain;
    WheelVelocities {
        left: -HALF * command.forward - SQRT_3_OVER_2 * command.lateral + yaw,
        back: command.forward + yaw,
        right: -HALF * command.forward + SQRT_3_OVER_2 * command.lateral + yaw,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_frame_uses_protocol_checksum() {
        assert_eq!(
            write_u16_frame(7, 42, 0x1234),
            [0xff, 0xff, 7, 5, WRITE_INSTRUCTION, 42, 0x34, 0x12, 0x80]
        );
    }

    #[test]
    fn forward_motion_preserves_three_wheel_geometry() {
        let wheels = three_omni_wheel_velocities(
            BodyVelocity {
                forward: 2.0,
                lateral: 0.0,
                yaw: 0.0,
            },
            1.0,
        );
        assert_eq!(wheels.left, -1.0);
        assert_eq!(wheels.back, 2.0);
        assert_eq!(wheels.right, -1.0);
    }

    #[test]
    fn yaw_motion_drives_each_wheel_equally() {
        let wheels = three_omni_wheel_velocities(
            BodyVelocity {
                forward: 0.0,
                lateral: 0.0,
                yaw: 3.0,
            },
            2.0,
        );
        assert_eq!(
            wheels,
            WheelVelocities {
                left: 6.0,
                back: 6.0,
                right: 6.0
            }
        );
    }
}
