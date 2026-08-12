//! SH2: reconstruct a reusable proof-obligation generator from anonymous graphs.

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum AtomKind {
    Relation,
    Property,
    Target,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Atom {
    pub kind: AtomKind,
    pub left: u8,
    pub right: u8,
}

impl Atom {
    fn relation(left: u8, right: u8) -> Self {
        Self {
            kind: AtomKind::Relation,
            left,
            right,
        }
    }

    fn property(role: u8) -> Self {
        Self {
            kind: AtomKind::Property,
            left: role,
            right: role,
        }
    }

    fn target(role: u8) -> Self {
        Self {
            kind: AtomKind::Target,
            left: role,
            right: role,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct HornSchema {
    pub relation_from: u8,
    pub relation_to: u8,
    pub property_role: u8,
    pub target_role: u8,
}

impl HornSchema {
    fn premises(self) -> [Atom; 2] {
        [
            Atom::relation(self.relation_from, self.relation_to),
            Atom::property(self.property_role),
        ]
    }

    fn conclusion(self) -> Atom {
        Atom::target(self.target_role)
    }
}

#[derive(Clone, Debug)]
struct ProofGraph {
    permutation: [u8; 3],
    facts: Vec<Atom>,
    conclusion: Atom,
}

fn permute(atom: Atom, permutation: [u8; 3]) -> Atom {
    Atom {
        kind: atom.kind,
        left: permutation[atom.left as usize],
        right: permutation[atom.right as usize],
    }
}

fn training_graphs() -> Vec<ProofGraph> {
    let canonical = HornSchema {
        relation_from: 0,
        relation_to: 1,
        property_role: 0,
        target_role: 1,
    };
    [[0, 1, 2], [1, 2, 0], [2, 0, 1]]
        .into_iter()
        .map(|permutation| ProofGraph {
            permutation,
            facts: canonical
                .premises()
                .into_iter()
                .map(|atom| permute(atom, permutation))
                .collect(),
            conclusion: permute(canonical.conclusion(), permutation),
        })
        .collect()
}

fn schemas() -> Vec<HornSchema> {
    let mut output = Vec::new();
    for relation_from in 0..3 {
        for relation_to in 0..3 {
            if relation_from == relation_to {
                continue;
            }
            for property_role in 0..3 {
                for target_role in 0..3 {
                    output.push(HornSchema {
                        relation_from,
                        relation_to,
                        property_role,
                        target_role,
                    });
                }
            }
        }
    }
    output
}

fn instantiates(schema: HornSchema, graph: &ProofGraph) -> bool {
    let premises = schema
        .premises()
        .into_iter()
        .map(|atom| permute(atom, graph.permutation))
        .collect::<Vec<_>>();
    let conclusion = permute(schema.conclusion(), graph.permutation);
    premises.iter().all(|atom| graph.facts.contains(atom)) && conclusion == graph.conclusion
}

fn closure_derives(schema: HornSchema, facts: &[Atom], target: Atom) -> bool {
    let premises = schema.premises();
    premises.iter().all(|premise| facts.contains(premise)) && schema.conclusion() == target
}

fn ablations_fail(schema: HornSchema) -> bool {
    training_graphs().iter().all(|graph| {
        let [relation, property] = schema
            .premises()
            .map(|atom| permute(atom, graph.permutation));
        let instantiated = HornSchema {
            relation_from: graph.permutation[schema.relation_from as usize],
            relation_to: graph.permutation[schema.relation_to as usize],
            property_role: graph.permutation[schema.property_role as usize],
            target_role: graph.permutation[schema.target_role as usize],
        };
        !closure_derives(instantiated, &[relation], graph.conclusion)
            && !closure_derives(instantiated, &[property], graph.conclusion)
    })
}

#[derive(Clone, Debug)]
pub struct Discovery {
    pub selected: Option<HornSchema>,
    pub candidate_space: usize,
    pub schemas_checked: usize,
    pub training_graphs: usize,
    pub ablations_declined: bool,
}

fn discover() -> Discovery {
    let candidates = schemas();
    let graphs = training_graphs();
    for (index, schema) in candidates.iter().copied().enumerate() {
        if graphs.iter().all(|graph| instantiates(schema, graph)) && ablations_fail(schema) {
            return Discovery {
                selected: Some(schema),
                candidate_space: candidates.len(),
                schemas_checked: index + 1,
                training_graphs: graphs.len(),
                ablations_declined: true,
            };
        }
    }
    Discovery {
        selected: None,
        candidate_space: candidates.len(),
        schemas_checked: candidates.len(),
        training_graphs: graphs.len(),
        ablations_declined: false,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Domain {
    FiniteGraphSpectrum,
    RealXi,
    DirectArithmetic,
}

#[derive(Clone, Debug)]
struct Application {
    domain: Domain,
    has_witness_slot: bool,
    target_role: u8,
}

fn application_graph(application: &Application) -> Option<ProofGraph> {
    application.has_witness_slot.then_some(ProofGraph {
        permutation: [0, 1, 2],
        facts: vec![Atom::relation(0, 1), Atom::property(0)],
        conclusion: Atom::target(application.target_role),
    })
}

fn baseline_search(application: &Application) -> (bool, usize) {
    let Some(graph) = application_graph(application) else {
        return (false, 1);
    };
    for (index, schema) in schemas().into_iter().enumerate() {
        if instantiates(schema, &graph) && ablations_fail(schema) {
            return (true, index + 1);
        }
    }
    (false, schemas().len())
}

#[derive(Clone, Debug)]
pub struct Transfer {
    pub task: &'static str,
    pub compatible: bool,
    pub generated_obligations: Vec<&'static str>,
    pub exact: bool,
    pub baseline_proposal_checks: usize,
    pub acquired_proposal_checks: usize,
    pub false_positive: bool,
    pub negative_transfer: bool,
}

fn apply(task: &'static str, application: Application, schema: HornSchema) -> Transfer {
    let compatible = application.has_witness_slot
        && application.target_role == schema.target_role
        && application.domain != Domain::DirectArithmetic;
    let generated_obligations = if compatible {
        match application.domain {
            Domain::FiniteGraphSpectrum => vec![
                "relate witness values to observed graph spectrum",
                "certify the witness structural property",
            ],
            Domain::RealXi => vec![
                "relate witness spectrum to nontrivial xi zeros",
                "certify the witness self-adjointness property",
            ],
            Domain::DirectArithmetic => Vec::new(),
        }
    } else {
        Vec::new()
    };
    let (baseline_found, baseline_proposal_checks) = baseline_search(&application);
    let acquired_proposal_checks = if compatible {
        generated_obligations.iter().count()
    } else {
        usize::from(application_graph(&application).is_some())
    };
    Transfer {
        task,
        compatible,
        exact: !compatible || (baseline_found && generated_obligations.len() == 2),
        generated_obligations,
        baseline_proposal_checks,
        acquired_proposal_checks,
        false_positive: !compatible && acquired_proposal_checks > 1,
        negative_transfer: acquired_proposal_checks > baseline_proposal_checks,
    }
}

#[derive(Clone, Debug)]
pub struct Sh2Experiment {
    pub discovery: Discovery,
    pub transfers: Vec<Transfer>,
    pub acquisition_cost: usize,
    pub baseline_ops: usize,
    pub acquired_reuse_ops: usize,
    pub amortized_ops: usize,
    pub amortization_horizon: usize,
    pub measured_gain_at_horizon: usize,
    pub false_positive_uses: usize,
    pub negative_transfer_tasks: usize,
    pub real_xi_obligations_regenerated: bool,
    pub obligations_proved: bool,
    pub generator_status: &'static str,
    pub sh2_passed: bool,
    pub m29_reached: bool,
    pub claim_level: &'static str,
}

pub fn sh2_experiment() -> Sh2Experiment {
    let discovery = discover();
    let selected = discovery.selected.unwrap_or(HornSchema {
        relation_from: 2,
        relation_to: 2,
        property_role: 2,
        target_role: 2,
    });
    let transfers = vec![
        apply(
            "unseen_finite_graph_spectrum",
            Application {
                domain: Domain::FiniteGraphSpectrum,
                has_witness_slot: true,
                target_role: 1,
            },
            selected,
        ),
        apply(
            "real_xi_M29_retry",
            Application {
                domain: Domain::RealXi,
                has_witness_slot: true,
                target_role: 1,
            },
            selected,
        ),
        apply(
            "direct_arithmetic_control",
            Application {
                domain: Domain::DirectArithmetic,
                has_witness_slot: false,
                target_role: 1,
            },
            selected,
        ),
    ];
    let acquisition_cost = discovery.schemas_checked;
    let compatible: Vec<_> = transfers.iter().filter(|task| task.compatible).collect();
    let per_horizon_baseline: usize = compatible
        .iter()
        .map(|task| task.baseline_proposal_checks)
        .sum();
    let per_horizon_acquired: usize = compatible
        .iter()
        .map(|task| task.acquired_proposal_checks)
        .sum();
    let amortization_horizon = (1..=100)
        .find(|horizon| {
            acquisition_cost + horizon * per_horizon_acquired < horizon * per_horizon_baseline
        })
        .unwrap_or(0);
    let baseline_ops = amortization_horizon * per_horizon_baseline;
    let acquired_reuse_ops = amortization_horizon * per_horizon_acquired;
    let amortized_ops = acquisition_cost + acquired_reuse_ops;
    let false_positive_uses = transfers.iter().filter(|task| task.false_positive).count();
    let negative_transfer_tasks = transfers
        .iter()
        .filter(|task| task.negative_transfer)
        .count();
    let real_xi_obligations_regenerated = transfers
        .iter()
        .find(|task| task.task == "real_xi_M29_retry")
        .is_some_and(|task| task.exact && task.generated_obligations.len() == 2);
    let obligations_proved = false;
    let m29_reached = false;
    let sh2_passed = discovery.selected.is_some()
        && discovery.ablations_declined
        && transfers.iter().all(|task| task.exact)
        && false_positive_uses == 0
        && negative_transfer_tasks == 0
        && real_xi_obligations_regenerated
        && amortization_horizon > 0
        && amortized_ops < baseline_ops
        && !obligations_proved
        && !m29_reached;
    Sh2Experiment {
        discovery,
        transfers,
        acquisition_cost,
        baseline_ops,
        acquired_reuse_ops,
        amortized_ops,
        amortization_horizon,
        measured_gain_at_horizon: baseline_ops.saturating_sub(amortized_ops),
        false_positive_uses,
        negative_transfer_tasks,
        real_xi_obligations_regenerated,
        obligations_proved,
        generator_status: if sh2_passed {
            "reconstructed"
        } else {
            "supplied"
        },
        sh2_passed,
        m29_reached,
        claim_level: if sh2_passed {
            "L3_reconstructed_meta_ontology_with_measured_transfer"
        } else {
            "L1_exact_reconstruction_without_utility"
        },
    }
}

pub fn machine_record(report: &Sh2Experiment) -> String {
    let proposal_counts = report
        .transfers
        .iter()
        .map(|task| {
            format!(
                "{}:{}>{}",
                task.task, task.baseline_proposal_checks, task.acquired_proposal_checks
            )
        })
        .collect::<Vec<_>>()
        .join(";");
    format!(
        "SH2b|space={}|checked={}|schema={:?}|training={}|ablations={}|transfers={}|proposal_counts={}|acquisition={}|baseline={}|reuse={}|amortized={}|horizon={}|gain={}|false_positive={}|negative_transfer={}|real_xi_regenerated={}|obligations_proved={}|status={}|pass={}|m29_reached={}|claim={}",
        report.discovery.candidate_space,
        report.discovery.schemas_checked,
        report.discovery.selected,
        report.discovery.training_graphs,
        report.discovery.ablations_declined,
        report.transfers.len(),
        proposal_counts,
        report.acquisition_cost,
        report.baseline_ops,
        report.acquired_reuse_ops,
        report.amortized_ops,
        report.amortization_horizon,
        report.measured_gain_at_horizon,
        report.false_positive_uses,
        report.negative_transfer_tasks,
        report.real_xi_obligations_regenerated,
        report.obligations_proved,
        report.generator_status,
        report.sh2_passed,
        report.m29_reached,
        report.claim_level
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn discovers_anonymous_two_premise_generator() {
        let report = sh2_experiment();
        assert_eq!(
            report.discovery.selected,
            Some(HornSchema {
                relation_from: 0,
                relation_to: 1,
                property_role: 0,
                target_role: 1,
            })
        );
        assert!(report.discovery.ablations_declined);
    }

    #[test]
    fn transfers_and_declines_incompatible_domain() {
        let report = sh2_experiment();
        assert!(report.transfers[0].compatible && report.transfers[0].exact);
        assert!(report.transfers[1].compatible && report.transfers[1].exact);
        assert!(!report.transfers[2].compatible);
        assert!(!report.transfers[2].false_positive);
    }

    #[test]
    fn corrected_accounting_rejects_non_amortizing_reconstruction() {
        let report = sh2_experiment();
        assert!(!report.sh2_passed, "{report:#?}");
        assert_eq!(report.generator_status, "supplied");
        assert!(report.real_xi_obligations_regenerated);
        assert!(!report.obligations_proved);
        assert!(!report.m29_reached);
        assert_eq!(machine_record(&report), machine_record(&sh2_experiment()));
    }
}
