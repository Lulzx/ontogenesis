use supsearch::symmetry_world::{
    m14_experiment, m14c_experiment, m14c_machine_record, machine_record,
};

fn main() {
    println!("ontogenesis: mathematical ontogenesis (M14)");
    println!("world: generic input transformations, output responses, exact action checking");
    let report = m14_experiment();
    println!(
        "invented action: {} inverse {}",
        report.action.transformation.render("input"),
        report.action.inverse.render("input")
    );
    for transfer in &report.transfers {
        println!(
            "transfer {} ({:?}): {} -> {} checks, response {}",
            transfer.task,
            transfer.domain,
            transfer.baseline_checks,
            transfer.retained_checks,
            transfer.response.render("output")
        );
    }
    println!("{}", machine_record(&report));
    let conditional = m14c_experiment();
    println!("m14c conditional transfer:");
    for transfer in &conditional.transfers {
        println!(
            "{}: route {:?}, {} -> {} checks",
            transfer.task,
            transfer.applicability,
            transfer.baseline_checks,
            transfer.acquired_checks
        );
    }
    println!("{}", m14c_machine_record(&conditional));
}
