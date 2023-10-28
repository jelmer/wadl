This crate contains a parser for the [Web Application Description Language (WADL)](https://www.w3.org/submissions/wadl/).

It can also generate basic rust bindings based on WADL files, if the ``codegen`` feature is enabled.

## Example usage

### Simply parsing the ast

```rust
let app: wadl::ast::Application = wadl::parse_file("1.0.wadl").unwrap();

println!("{:#}", app);
```

### Generating code

Create a build.rs that generates rust code:

```rust
fn main() {
    let config = wadl::codegen::Config {
        // Set extra options here to influence code generation,
        // e.g. to rename functions.
        ..Default::default()
    };

    let wadl = std::fs::read_to_string(
            concat!(env!("CARGO_MANIFEST_DIR"), "/x.wadl")).unwrap();

    let wadl_app = wadl::parse_string(wadl.as_str()).unwrap();
    let code = wadl::codegen::generate(&wadl_app, &config);
    let target_dir = std::path::PathBuf::from(std::env::var("OUT_DIR").unwrap())
        .canonicalize()
        .unwrap();
    let generated = target_dir.join("generated");
    std::fs::create_dir_all(&generated).unwrap();
    let path = generated.join("x.wadl");
    std::fs::write(path, code).unwrap();
}
```

Then, you can include the generated code from your rust code:

```rust
include!(concat!(env!("OUT_DIR"), "/generated/x.rs"));
```
