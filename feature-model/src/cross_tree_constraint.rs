pub enum CrossTreeConstraint {
    Implies(String, String),
    Exclusive(String, String),
    Not(String),
}