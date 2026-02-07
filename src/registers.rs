//! Register definitions for the BMA425 sensor.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub(crate) enum Register {
    ChipId = 0x00,
    ErrReg = 0x02,
    Status = 0x03,
    Data8 = 0x12,
    IntStatus0 = 0x1c,
    InternalStatus = 0x2a,
    AccConf = 0x40,
    Int1Map = 0x56,
    Int2Map = 0x57,
    InitCtrl = 0x59,
    AsicLsb = 0x5b,
    AsicMsb = 0x5c,
    FeaturesIn = 0x5e,
    PwrConf = 0x7c,
    PwrCtrl = 0x7d,
}

impl From<Register> for u8 {
    fn from(reg: Register) -> Self {
        reg as u8
    }
}

/// Interrupt mapping options for the BMA425 sensor.
#[repr(u8)]
#[derive(Debug, Clone, Copy)]
pub enum InterruptMap {
    /// Map interrupt to Error
    Error = 0x80,
    /// Map interrupt to TODO
    AnyNoMotion = 0x40,
    /// Map interrupt to wakeup
    Wakeup = 0x20,
    /// Map interrupt to wrist tilt
    WristTilt = 0x08,
    /// Map interrupt to activity type
    ActivityType = 0x04,
    /// Map interrupt to step counter
    StepCounter = 0x02,
}