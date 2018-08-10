use escargot::CargoBuild;
use std::result::Result;
use std::error::Error;

#[test]
fn test_exec() -> Result<(), Box<Error>> {
    let runner = CargoBuild::new()
        .current_release()
        .run()?;

    let mut cmd = runner.command();
    assert!(cmd.output().is_ok());
    Ok(())
}
