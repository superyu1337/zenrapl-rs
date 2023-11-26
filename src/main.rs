fn main() {
    simple_logger::SimpleLogger::new().init().unwrap();
    let power_info = rapl_ryzen::power_info().unwrap();
    println!("{:#?}", power_info);
}