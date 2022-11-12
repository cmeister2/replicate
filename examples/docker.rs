use std::env::args;

use replicate::Replicate;

// Run inside alpine:3, which requires this example to be built with musl (assuming it
// has not been built on alpine)
const IMAGE: &str = "alpine:3";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if args().len() == 1 {
        // Running without arguments - replicate and run Docker
        let copy = Replicate::new()?;
        println!("My copy's path is {}", copy.display());

        let pathstr = copy.display().to_string();
        let map = format!("{0}:{0}", pathstr);
        let mut child = std::process::Command::new("docker")
            .args(["run", "-t", "-v", &map, IMAGE, &pathstr, "inside"])
            .spawn()?;
        let ecode = child.wait()?;
        assert!(ecode.success());
    } else {
        println!("Called inside \"{}\" with arguments {:?}", IMAGE, args())
    }

    Ok(())
}
