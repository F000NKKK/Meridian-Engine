//! Minimal smoke-test example. Run with:
//!   ./build.sh run hello_engine -- --foo bar

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("Meridian-Engine hello_engine example");
    println!("args: {args:?}");
}
