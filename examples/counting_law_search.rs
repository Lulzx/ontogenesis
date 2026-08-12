use supsearch::counting_law_search::{machine_record, sh10_experiment};

fn main() {
    let report = sh10_experiment();
    println!("ontogenesis: SH10 counting-law-guided grammar repair");
    println!("selected: {:?}", report.selected);
    println!("{}", machine_record(&report));
}
