use std::str::FromStr;

use super::profile::{self, Profile};
use crate::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DeviceType {
    CiscoIos,
    Zte,
    ZteTelnet,
    Huawei,
    HuaweiTelnet,
    HuaweiOlt,
    HuaweiOltTelnet,
    Junos,
    Mikrotik,
    Linux,
}

impl DeviceType {
    pub fn profile(self) -> Profile {
        match self {
            DeviceType::CiscoIos => profile::CISCO_IOS,
            DeviceType::Zte => profile::ZTE,
            DeviceType::ZteTelnet => profile::ZTE_TELNET,
            DeviceType::Huawei => profile::HUAWEI_VRP,
            DeviceType::HuaweiTelnet => profile::HUAWEI_VRP_TELNET,
            DeviceType::HuaweiOlt => profile::HUAWEI_OLT,
            DeviceType::HuaweiOltTelnet => profile::HUAWEI_OLT_TELNET,
            DeviceType::Junos => profile::JUNOS,
            DeviceType::Mikrotik => profile::MIKROTIK,
            DeviceType::Linux => profile::LINUX,
        }
    }
}

/// Accepts netmiko's `device_type` names.
impl FromStr for DeviceType {
    type Err = Error;

    fn from_str(name: &str) -> Result<Self, Self::Err> {
        match name {
            "cisco_ios" | "cisco_xe" => Ok(DeviceType::CiscoIos),
            "zte_zxros" | "zte" => Ok(DeviceType::Zte),
            "zte_zxros_telnet" | "zte_telnet" => Ok(DeviceType::ZteTelnet),
            "huawei" | "huawei_vrp" => Ok(DeviceType::Huawei),
            "huawei_telnet" => Ok(DeviceType::HuaweiTelnet),
            "huawei_olt" | "huawei_smartax" => Ok(DeviceType::HuaweiOlt),
            "huawei_olt_telnet" => Ok(DeviceType::HuaweiOltTelnet),
            "juniper_junos" | "juniper" => Ok(DeviceType::Junos),
            "mikrotik_routeros" | "mikrotik" => Ok(DeviceType::Mikrotik),
            "linux" => Ok(DeviceType::Linux),
            other => Err(Error::UnknownDeviceType(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_netmiko_device_type_names() {
        assert_eq!("cisco_ios".parse::<DeviceType>().unwrap(), DeviceType::CiscoIos);
        assert_eq!("cisco_xe".parse::<DeviceType>().unwrap(), DeviceType::CiscoIos);
        assert_eq!("zte_zxros".parse::<DeviceType>().unwrap(), DeviceType::Zte);
        assert_eq!("zte_zxros_telnet".parse::<DeviceType>().unwrap(), DeviceType::ZteTelnet);
        assert_eq!("huawei".parse::<DeviceType>().unwrap(), DeviceType::Huawei);
        assert_eq!("huawei_vrp".parse::<DeviceType>().unwrap(), DeviceType::Huawei);
        assert_eq!("huawei_telnet".parse::<DeviceType>().unwrap(), DeviceType::HuaweiTelnet);
        assert_eq!("huawei_olt".parse::<DeviceType>().unwrap(), DeviceType::HuaweiOlt);
        assert_eq!("huawei_smartax".parse::<DeviceType>().unwrap(), DeviceType::HuaweiOlt);
        assert_eq!("huawei_olt_telnet".parse::<DeviceType>().unwrap(), DeviceType::HuaweiOltTelnet);
        assert_eq!("juniper_junos".parse::<DeviceType>().unwrap(), DeviceType::Junos);
        assert_eq!("mikrotik_routeros".parse::<DeviceType>().unwrap(), DeviceType::Mikrotik);
        assert_eq!("linux".parse::<DeviceType>().unwrap(), DeviceType::Linux);
    }

    #[test]
    fn rejects_unknown_device_type() {
        assert!(matches!("hp_comware".parse::<DeviceType>(), Err(Error::UnknownDeviceType(_))));
    }

    #[test]
    fn only_mikrotik_changes_the_login_name() {
        for device_type in [
            DeviceType::CiscoIos,
            DeviceType::Zte,
            DeviceType::ZteTelnet,
            DeviceType::Huawei,
            DeviceType::HuaweiTelnet,
            DeviceType::HuaweiOlt,
            DeviceType::HuaweiOltTelnet,
            DeviceType::Junos,
            DeviceType::Linux,
        ] {
            assert_eq!(device_type.profile().username_suffix, "", "{device_type:?}");
        }
        assert_eq!(DeviceType::Mikrotik.profile().username_suffix, "+ct511w4098h");
    }

    #[test]
    fn zte_telnet_uses_port_23_and_crlf_with_zte_commands() {
        let zte_telnet = DeviceType::ZteTelnet.profile();
        assert_eq!(zte_telnet.protocol.default_port(), 23);
        assert_eq!(zte_telnet.enter, "\r\n");
        assert_eq!(zte_telnet.save_command, DeviceType::Zte.profile().save_command);
    }

    #[test]
    fn huawei_telnet_variants_keep_ssh_commands() {
        let vrp = DeviceType::Huawei.profile();
        let vrp_telnet = DeviceType::HuaweiTelnet.profile();
        assert_eq!(vrp_telnet.protocol.default_port(), 23);
        assert_eq!(vrp_telnet.telnet_password_enter, "\r");
        assert_eq!(vrp_telnet.config_enter_command, vrp.config_enter_command);
        let olt_telnet = DeviceType::HuaweiOltTelnet.profile();
        assert_eq!(olt_telnet.protocol.default_port(), 23);
        assert_eq!(olt_telnet.paging_commands, DeviceType::HuaweiOlt.profile().paging_commands);
    }

    #[test]
    fn zte_inherits_cisco_modes_but_saves_with_write() {
        let zte = DeviceType::Zte.profile();
        assert_eq!(zte.config_enter_command, Some("configure terminal"));
        assert_eq!(zte.save_command, Some("write"));
    }
}
