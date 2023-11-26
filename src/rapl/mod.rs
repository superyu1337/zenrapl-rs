use log::info;

const MAX_CPUS: usize = 1024;
const MAX_PACKAGES: usize = 16;

#[derive(thiserror::Error, Debug)]
pub enum RaplError {
    #[error("utf8 error")]
    Utf8(#[from] std::string::FromUtf8Error),
    #[error("int parse error")]
    Parse(#[from] std::num::ParseIntError),
    #[error("io error")]
    Io(#[from] std::io::Error),
    #[error("smt check error")]
    SmtCheck(&'static str)
}

#[repr(u64)]
pub enum MsrFields {
    PowerUnit = 0xC0010299,
    CoreEnergy = 0xC001029A,
    PackageEnergy = 0xC001029B,
}

pub enum AmdUnitMasks {
    //Time = 0xF0000,
    Energy = 0x1F00,
    //Power = 0xF
}

pub struct PackageInfo {
    smt: bool,
    threads: usize,
    cores: usize,
    packages: usize,
    package_map: [Option<usize>; MAX_PACKAGES]
}

impl PackageInfo {
    pub fn smt(&self) -> bool {
        self.smt
    }

    pub fn threads(&self) -> usize {
        self.threads
    }

    pub fn cores(&self) -> usize {
        self.cores
    }

    pub fn packages(&self) -> usize {
        self.packages
    }

    pub fn package_map(&self) -> [Option<usize>; MAX_PACKAGES] {
        self.package_map
    }
}

impl Default for PackageInfo {
    fn default() -> Self {
        Self { smt: false, threads: 0, cores: 0, packages: 0, package_map: [None; MAX_PACKAGES] }
    }
}

#[derive(Default, Clone, Debug)]
pub struct PowerInfo {
    threads: usize,
    cores: usize,
    packages: usize,
    core_sum: f64,
    core_powers: Vec<f64>,
    package_sum: f64,
    package_powers: Vec<f64>
}

impl PowerInfo {
    pub fn new(threads: usize, cores: usize, packages: usize, core_sum: f64, core_powers: Vec<f64>, package_sum: f64, package_powers: Vec<f64>) -> Self {
        Self {
            threads,
            cores,
            packages,
            core_sum,
            core_powers,
            package_sum,
            package_powers,
        }
    }

    /// Returns the amount of threads in the system
    pub fn threads(&self) -> usize {
        self.threads
    }

    /// Returns the amount of cores in the system
    pub fn cores(&self) -> usize {
        self.cores
    }

    /// Returns the amount of packages in the system
    pub fn packages(&self) -> usize {
        self.packages
    }

    /// Returns the summarized power usage of all cores
    pub fn core_sum(&self) -> f64 {
        self.core_sum
    }

    /// Returns a Vec of power usages, where each index corresponds to a core id
    pub fn core_powers(&self) -> &Vec<f64> {
        &self.core_powers
    }

    /// Returns the summarized power usage of all packages
    pub fn package_sum(&self) -> f64 {
        self.package_sum
    }

    /// Returns a Vec of power usages, where each index corresponds to a package id
    pub fn package_powers(&self) -> &Vec<f64> {
        &self.package_powers
    }
}

fn check_smt() -> Result<bool, RaplError> {
    let c = std::fs::read("/sys/devices/system/cpu/smt/active")?;
    let byte = c.first().ok_or(RaplError::SmtCheck("Could not check for SMT"))?;

    Ok(*byte == 49)
}

pub fn detect_packages() -> Result<PackageInfo, RaplError> {
    let mut ctx = PackageInfo { smt: check_smt()?, .. PackageInfo::default() };

    let mut cores_this_package = 0;

    for i in 0..MAX_CPUS {
        let filename = format!("/sys/devices/system/cpu/cpu{}/topology/physical_package_id", i);
        let content = std::fs::read(filename);
        if content.is_err() {
            break;
        }

        let content = String::from_utf8(content.unwrap())?;
        let package = content.trim().parse::<usize>()?;
        ctx.threads += 1;

        info!("{i:02} ({package})");

        if ctx.package_map[package].is_none() {
            ctx.packages += 1;
            cores_this_package = 0;
        }

        cores_this_package += 1;

        if ctx.smt {
            ctx.package_map[package] = Some(cores_this_package / 2);
        } else {
            ctx.package_map[package] = Some(cores_this_package);
        }
    }

    info!("Detected {} threads in {} packages", ctx.threads, ctx.packages);

    if ctx.smt {
        ctx.cores = ctx.threads / 2;
        info!("SMT is on")
    } else {
        ctx.cores = ctx.threads;
        info!("SMT is off")
    }

    info!("There is {} threads and {} cores in the system", ctx.threads, ctx.cores);

    Ok(ctx)
}

pub fn read(core: usize, field: MsrFields) -> Result<u64, RaplError> {
    let filename = format!("/dev/cpu/{}/msr", core);
    let file = std::fs::File::open(filename)?;

    let mut buffer = [0u8; 8];
    std::os::unix::fs::FileExt::read_exact_at(&file, &mut buffer, field as u64)?;
    let data = u64::from_le_bytes(buffer);

    Ok(data)
}