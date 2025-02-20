//! Builds the SP1 program

#[cfg(feature = "sp1")]
use sp1_helper::build_program;

#[cfg(feature = "sp1")]
fn main() {
    build_program("./src/sp1/program");
}
