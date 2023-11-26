use log::info;
use rapl::{RaplError, MsrFields, AmdUnitMasks, PowerInfo};

mod rapl;

pub fn power_info() -> Result<PowerInfo, RaplError> {
    let ctx = rapl::detect_packages()?;

    let mut package_powers = vec![];
    let mut core_powers = vec![];

    let mut cpus_seen = 0;
    for cores_in_package in ctx.package_map().iter().filter_map(|x| x.as_ref() ) {

        info!("cores in current package {}", cores_in_package);

        let mut core_buf: Vec<(f64, f64)> = vec![(0.0, 0.0); *cores_in_package];
        let mut package_buf: (f64, f64) = (0.0, 0.0);

        let core_energy_units = rapl::read(0, MsrFields::PowerUnit)?;

        //let time_unit = (core_energy_units & AmdUnitMasks::Time as u64) >> 16;
        let energy_unit = (core_energy_units & AmdUnitMasks::Energy as u64) >> 8;
        //let power_unit = core_energy_units & AmdUnitMasks::Power as u64;
    
        //let time_unit_d = 0.5f64.powf(time_unit as f64);
        let energy_unit_d = 0.5f64.powf(energy_unit as f64);
        //let power_unit_d = 0.5f64.powf(power_unit as f64);

        for buff in core_buf.iter_mut().enumerate().take(*cores_in_package).skip(cpus_seen) {
            let core_energy_raw = rapl::read(buff.0, MsrFields::CoreEnergy).unwrap();
            let package_raw = rapl::read(buff.0, MsrFields::PackageEnergy).unwrap();

            buff.1.0 = core_energy_raw as f64 * energy_unit_d;
            package_buf.0 = package_raw as f64 * energy_unit_d;
        }

        std::thread::sleep(std::time::Duration::from_micros(100000));

        for buff in core_buf.iter_mut().enumerate().take(*cores_in_package).skip(cpus_seen) {
            let core_energy_raw = rapl::read(buff.0, MsrFields::CoreEnergy).unwrap();
            let package_raw = rapl::read(buff.0, MsrFields::PackageEnergy).unwrap();

            buff.1.1 = core_energy_raw as f64 * energy_unit_d;
            package_buf.1 = package_raw as f64 * energy_unit_d;
        }

        for buff in core_buf.iter().take(*cores_in_package).skip(cpus_seen) {
            let core_diff = (buff.1 - buff.0) * 10.0;
            core_powers.push(core_diff);
        }

        let package_diff = (package_buf.1 - package_buf.0) * 10.0;
        package_powers.push(package_diff);

        if ctx.smt() {
            cpus_seen = *cores_in_package*2;
        } else {
            cpus_seen = *cores_in_package;
        }

        info!("cpus seen: {}", cpus_seen)
    }

    let power_info = PowerInfo::new(
        ctx.threads(),
        ctx.cores(),
        ctx.packages(),
        core_powers.iter().sum(),
        core_powers,
        package_powers.iter().sum(),
        package_powers
    );


    Ok(power_info)
}