//! Builds the SP1 program

fn main() {
    #[cfg(feature = "sp1")]
    sp1_helper::build_program("./src/sp1/program");
}
