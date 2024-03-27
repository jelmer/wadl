#[test]
fn parse_sample_wadl() {
    wadl::parse_file("tests/sample-wadl.xml").unwrap();
}

#[test]
fn parse_yahoo_wadl() {
    wadl::parse_file("tests/yahoo-wadl.xml").unwrap();
}

#[test]
fn parse_fish_eye_wadl() {
    wadl::parse_file("tests/fish-eye-wadl.xml").unwrap();
}
