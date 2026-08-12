use supsearch::fourier_world::{
    m15_experiment, m15b_experiment, m15b_machine_record, machine_record, Route,
};

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
    println!("ontogenesis: mathematical ontogenesis (M15b)");
    println!("world: conditional routing of closed-shift coordinate priority");
    let conditional = m15b_experiment();
    for transfer in &conditional.transfers {
        println!(
            "{:?} {}: {} -> {} checks, exact {}, route {}",
            transfer.route,
            transfer.task,
            transfer.baseline_checks,
            transfer.acquired_checks,
            transfer.exact_winner,
            if transfer.route == Route::Routed {
                "guided"
            } else {
                "declined"
            }
        );
    }
    println!("{}", m15b_machine_record(&conditional));
}
