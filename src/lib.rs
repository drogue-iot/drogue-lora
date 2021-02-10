#![no_std]

#[derive(Debug, Clone, Copy)]
pub enum QoS {
    Unconfirmed,
    Confirmed,
}

#[derive(Debug, Clone, Copy)]
pub enum ResetMode {
    Restart,
    Reload,
}

#[derive(Debug, Clone, Copy)]
pub enum ConnectMode {
    OTAA,
    ABP,
}

#[derive(Debug, Clone, Copy)]
#[repr(u8)]
pub enum LoraMode {
    WAN = 0,
    P2P = 1,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LoraRegion {
    EU868,
    US915,
    AU915,
    KR920,
    AS923,
    IN865,
    UNKNOWN,
}

pub type Port = u8;
pub type DevAddr = [u8; 4];
pub type EUI = [u8; 8];
pub type AppKey = [u8; 16];
pub type NwksKey = [u8; 16];
pub type AppsKey = [u8; 16];

#[derive(Debug, Clone, Copy)]
pub struct LoraConfig {
    pub connect_mode: Option<ConnectMode>,
    pub band: Option<LoraRegion>,
    pub lora_mode: Option<LoraMode>,
    pub device_address: Option<DevAddr>,
    pub device_eui: Option<EUI>,
    pub app_eui: Option<EUI>,
    pub app_key: Option<AppKey>,
}

impl LoraConfig {
    pub fn new() -> Self {
        Self {
            connect_mode: None,
            band: None,
            lora_mode: None,
            device_address: None,
            device_eui: None,
            app_eui: None,
            app_key: None,
        }
    }

    pub fn connect_mode(mut self, mode: ConnectMode) -> Self {
        self.connect_mode.replace(mode);
        self
    }

    pub fn band(mut self, band: LoraRegion) -> Self {
        self.band.replace(band);
        self
    }

    pub fn lora_mode(mut self, lora_mode: LoraMode) -> Self {
        self.lora_mode.replace(lora_mode);
        self
    }

    pub fn device_address(mut self, device_address: &DevAddr) -> Self {
        self.device_address.replace(device_address.clone());
        self
    }

    pub fn device_eui(mut self, device_eui: &EUI) -> Self {
        self.device_eui.replace(device_eui.clone());
        self
    }

    pub fn app_eui(mut self, app_eui: &EUI) -> Self {
        self.app_eui.replace(app_eui.clone());
        self
    }

    pub fn app_key(mut self, app_key: &AppKey) -> Self {
        self.app_key.replace(app_key.clone());
        self
    }
}
