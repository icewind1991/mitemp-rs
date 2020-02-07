fn main() -> Result<(), btleplug::Error> {
    env_logger::init();
    mitemp::test()?;
    Ok(())
}
