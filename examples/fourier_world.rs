use supsearch::fourier_world::{m15_experiment, machine_record};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M15)");
    println!("world: recurrence-generated coordinates and exact cyclic-shift dynamics");
    let report = m15_experiment();
    for transfer in &report.transfers {
        println!(
            "{}: {} -> {} checks, exact error {}",
            transfer.task, transfer.unguided_checks, transfer.guided_checks, transfer.exact_error
        );
    }
    println!("{}", machine_record(&report));
}
