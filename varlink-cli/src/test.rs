use escargot::CargoBuild;
use std::error::Error;
use std::result::Result;

#[test]
fn test_exec() -> Result<(), Box<Error>> {
    let runner = CargoBuild::new().current_release().run()?;

    let mut cmd = runner.command();
    assert!(cmd.output().is_ok());
    Ok(())
}
