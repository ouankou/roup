use roup::parser::{parse, OpenMPVersion};

fn main() {
    let input = "#pragma omp for schedule(monotonic,simd:runtime,2)";
    let parsed = parse(input, OpenMPVersion::V52).unwrap();
    println!("{}", parsed);
}