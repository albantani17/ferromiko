use crate::DeviceType;

/// The `Platform` name ntc-templates' index uses; Telnet variants share the SSH platform's templates.
pub fn ntc_platform(device_type: DeviceType) -> &'static str {
    match device_type {
        DeviceType::CiscoIos => "cisco_ios",
        DeviceType::Zte | DeviceType::ZteTelnet => "zte_zxros",
        DeviceType::Huawei | DeviceType::HuaweiTelnet => "huawei_vrp",
        DeviceType::HuaweiOlt | DeviceType::HuaweiOltTelnet => "huawei_smartax",
        DeviceType::Junos => "juniper_junos",
        DeviceType::Mikrotik => "mikrotik_routeros",
        DeviceType::Linux => "linux",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_ssh_types_to_ntc_platform_names() {
        assert_eq!(ntc_platform(DeviceType::CiscoIos), "cisco_ios");
        assert_eq!(ntc_platform(DeviceType::Huawei), "huawei_vrp");
        assert_eq!(ntc_platform(DeviceType::HuaweiOlt), "huawei_smartax");
        assert_eq!(ntc_platform(DeviceType::Junos), "juniper_junos");
        assert_eq!(ntc_platform(DeviceType::Mikrotik), "mikrotik_routeros");
        assert_eq!(ntc_platform(DeviceType::Linux), "linux");
    }

    #[test]
    fn telnet_variants_use_the_ssh_platform() {
        assert_eq!(ntc_platform(DeviceType::ZteTelnet), ntc_platform(DeviceType::Zte));
        assert_eq!(ntc_platform(DeviceType::HuaweiTelnet), ntc_platform(DeviceType::Huawei));
        assert_eq!(ntc_platform(DeviceType::HuaweiOltTelnet), ntc_platform(DeviceType::HuaweiOlt));
    }
}
