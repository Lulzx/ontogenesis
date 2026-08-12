use supsearch::operator_search::{machine_record, sh5_experiment};

fn main() {
    let report = sh5_experiment();
    println!("ontogenesis: SH5 prime-derived operator proof search");
    println!("best candidate: {:?}", report.best.candidate);
    println!("frontier: {:?}", report.best.frontier);
    println!("M29 reached: {}", report.m29_reached);
    println!("{}", machine_record(&report));
}
