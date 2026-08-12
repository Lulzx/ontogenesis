use supsearch::stall_diagnosis::{machine_record, sh1_diagnosis};

fn main() {
    let report = sh1_diagnosis();
    println!("ontogenesis: SH1 M29 stall diagnosis");
    println!("load-bearing: {:?}", report.load_bearing);
    println!("next target: {}", report.first_self_hosting_target);
    println!("{}", machine_record(&report));
}
