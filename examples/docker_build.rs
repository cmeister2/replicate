use std::env::args;

use replicate::Replicate;

// Run inside alpine:3, which requires this example to be built with musl (assuming it
// has not been built on alpine)
const IMAGE: &str = "alpine:3";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if args().len() == 1 {
        // Running without arguments - replicate and run Docker
        let copy = Replicate::same_name()?;
        println!("My copy's path is {}", copy.display());

        // Use the parent path of the copied executable as a build-context for Docker
        let args: Vec<String> = vec![
            "build".into(),
            "--build-context".into(),
            format!("replicate={}", copy.parent().display()),
            "-f".into(),
            "examples/Dockerfile".into(),
            ".".into(),
        ];

        let mut child = std::process::Command::new("docker").args(args).spawn()?;
        let ecode = child.wait()?;
        assert!(ecode.success());
    } else {
        println!("Called inside \"{}\" with arguments {:?}", IMAGE, args())
    }

    Ok(())
}
