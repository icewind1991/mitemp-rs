fn main() -> Result<(), btleplug::Error> {
    mitemp::test()?;
    Ok(())
}