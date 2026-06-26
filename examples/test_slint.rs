slint::slint! { export component Test inherits Window { Text { text: "Hello"; } } }
fn main() { Test::new().unwrap().run().unwrap(); }
