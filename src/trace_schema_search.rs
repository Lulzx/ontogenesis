//! SH6b: reconstruct and transfer an exact finite trace schema.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Schema {
    DiagonalEntryPowers,
    AllEntryPowers,
    ClosedWalkProducts,
    AllWalkProducts,
}

const SCHEMAS: [Schema; 4] = [
    Schema::DiagonalEntryPowers,
    Schema::AllEntryPowers,
    Schema::ClosedWalkProducts,
    Schema::AllWalkProducts,
];

#[derive(Clone, Debug)]
struct Matrix {
    rows: usize,
    cols: usize,
    entries: Vec<i64>,
}

impl Matrix {
    fn square(rows: Vec<Vec<i64>>) -> Self {
        let size = rows.len();
        assert!(rows.iter().all(|row| row.len() == size));
        Self {
            rows: size,
            cols: size,
            entries: rows.into_iter().flatten().collect(),
        }
    }

    fn get(&self, row: usize, col: usize) -> i64 {
        self.entries[row * self.cols + col]
    }
}

fn path(size: usize) -> Matrix {
    Matrix::square(
        (0..size)
            .map(|row| {
                (0..size)
                    .map(|col| i64::from(row.abs_diff(col) == 1))
                    .collect()
            })
            .collect(),
    )
}

fn cycle(size: usize) -> Matrix {
    Matrix::square(
        (0..size)
            .map(|row| {
                (0..size)
                    .map(|col| i64::from((row + 1) % size == col || (col + 1) % size == row))
                    .collect()
            })
            .collect(),
    )
}

fn directed() -> Matrix {
    Matrix::square(vec![
        vec![0, 1, 1, 0],
        vec![0, 0, 1, 0],
        vec![1, 0, 0, 1],
        vec![0, 1, 0, 0],
    ])
}

fn exact_trace_power(matrix: &Matrix, power: usize) -> Option<i64> {
    if matrix.rows != matrix.cols {
        return None;
    }
    let size = matrix.rows;
    let mut product = vec![vec![0_i64; size]; size];
    for (index, row) in product.iter_mut().enumerate() {
        row[index] = 1;
    }
    for _ in 0..power {
        let mut next = vec![vec![0_i64; size]; size];
        for (row, next_row) in next.iter_mut().enumerate() {
            for (middle, product_value) in product[row].iter().enumerate() {
                for (col, value) in next_row.iter_mut().enumerate() {
                    *value += product_value * matrix.get(middle, col);
                }
            }
        }
        product = next;
    }
    Some((0..size).map(|index| product[index][index]).sum())
}

fn enumerate_products(matrix: &Matrix, power: usize, closed: bool) -> i64 {
    let size = matrix.rows;
    let mut total = 0;
    let sequence_count = size.pow((power + 1) as u32);
    for encoded in 0..sequence_count {
        let mut code = encoded;
        let mut vertices = vec![0; power + 1];
        for vertex in &mut vertices {
            *vertex = code % size;
            code /= size;
        }
        if closed && vertices[0] != vertices[power] {
            continue;
        }
        total += (0..power)
            .map(|step| matrix.get(vertices[step], vertices[step + 1]))
            .product::<i64>();
    }
    total
}

fn evaluate(schema: Schema, matrix: &Matrix, power: usize) -> Option<i64> {
    if matrix.rows != matrix.cols {
        return None;
    }
    Some(match schema {
        Schema::DiagonalEntryPowers => (0..matrix.rows)
            .map(|index| matrix.get(index, index).pow(power as u32))
            .sum(),
        Schema::AllEntryPowers => matrix
            .entries
            .iter()
            .map(|entry| entry.pow(power as u32))
            .sum(),
        Schema::ClosedWalkProducts => enumerate_products(matrix, power, true),
        Schema::AllWalkProducts => enumerate_products(matrix, power, false),
    })
}

fn agrees(schema: Schema, tasks: &[(Matrix, usize)]) -> bool {
    tasks.iter().all(|(matrix, power)| {
        evaluate(schema, matrix, *power) == exact_trace_power(matrix, *power)
    })
}

#[derive(Clone, Debug)]
pub struct Sh6bExperiment {
    pub training_tasks: usize,
    pub proposal_checks: usize,
    pub retained_schema: &'static str,
    pub transfers: [bool; 3],
    pub rectangular_declined: bool,
    pub cold_checks: usize,
    pub acquired_checks: usize,
    pub net_saved_checks: isize,
    pub sh6b_passed: bool,
    pub m29_reached: bool,
}

pub fn sh6b_experiment() -> Sh6bExperiment {
    let training = (3..=6)
        .flat_map(|size| (2..=4).flat_map(move |power| [(path(size), power), (cycle(size), power)]))
        .collect::<Vec<_>>();
    let (retained_index, retained) = SCHEMAS
        .iter()
        .copied()
        .enumerate()
        .find(|(_, schema)| agrees(*schema, &training))
        .expect("frozen grammar contains an exact schema");
    let transfer_tasks = [
        vec![(path(7), 4), (path(8), 3)],
        vec![(cycle(7), 3), (cycle(8), 4)],
        vec![(directed(), 2), (directed(), 3), (directed(), 4)],
    ];
    let transfers = transfer_tasks.map(|tasks| agrees(retained, &tasks));
    let rectangular = Matrix {
        rows: 2,
        cols: 3,
        entries: vec![1, 0, 1, 0, 1, 0],
    };
    let rectangular_declined = evaluate(retained, &rectangular, 2).is_none()
        && exact_trace_power(&rectangular, 2).is_none();
    let proposal_checks = retained_index + 1;
    let cold_checks = proposal_checks * transfers.len();
    let acquired_checks = proposal_checks + transfers.len();
    let net_saved_checks = cold_checks as isize - acquired_checks as isize;
    let sh6b_passed =
        transfers.iter().all(|passed| *passed) && rectangular_declined && net_saved_checks > 0;
    Sh6bExperiment {
        training_tasks: training.len(),
        proposal_checks,
        retained_schema: "cyclic_closed_walk_products",
        transfers,
        rectangular_declined,
        cold_checks,
        acquired_checks,
        net_saved_checks,
        sh6b_passed,
        m29_reached: false,
    }
}

pub fn machine_record(report: &Sh6bExperiment) -> String {
    format!(
        "SH6b|training_tasks={}|proposal_checks={}|retained_schema={}|transfers={:?}|rectangular_declined={}|cold_checks={}|acquired_checks={}|net_saved_checks={}|passed={}|m29_reached=false|claim=finite_trace_schema_calibration_only",
        report.training_tasks,
        report.proposal_checks,
        report.retained_schema,
        report.transfers,
        report.rectangular_declined,
        report.cold_checks,
        report.acquired_checks,
        report.net_saved_checks,
        report.sh6b_passed,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn independently_matches_closed_walks_to_trace_powers() {
        let matrix = directed();
        for power in 1..=4 {
            assert_eq!(
                evaluate(Schema::ClosedWalkProducts, &matrix, power),
                exact_trace_power(&matrix, power)
            );
        }
    }

    #[test]
    fn reconstructs_transfers_and_amortizes_the_schema() {
        let report = sh6b_experiment();
        assert!(report.sh6b_passed, "{report:#?}");
        assert_eq!(report.proposal_checks, 3);
        assert_eq!(report.transfers, [true; 3]);
        assert!(report.rectangular_declined);
        assert_eq!(report.net_saved_checks, 3);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh6b_experiment()));
    }
}
