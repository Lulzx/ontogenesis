use supsearch::generator_reconstruction::{machine_record, sh2_experiment};

fn main() {
    let report = sh2_experiment();
    println!("ontogenesis: SH2b generator reconstruction");
    println!("selected schema: {:?}", report.discovery.selected);
    println!("generator status: {}", report.generator_status);
    println!("M29 reached: {}", report.m29_reached);
    println!("{}", machine_record(&report));
}
