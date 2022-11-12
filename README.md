# replicate

replicate is a library that:

- copies the currently running program to a temporary file
- (Unix-only) makes that file executable
- returns a path object to the temporary file, that cleans up the temporary file when dropped.

It's intended to be used by musl-compiled programs which can run inside Docker containers; by
creating a copy and then volume-mounting that program within the Docker container.

# Examples

```rust
use replicate::Replicate;

fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let copy = Replicate::new()?;
    println!("My copy's path is {}", copy.display());
    Ok(())
}
```

Additional examples are in the [examples](examples) directory.