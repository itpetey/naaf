use naaf_executors::executor::Executor;
use naaf_validators::validator::Validator;

pub struct Transition {
    pub id: &'static str,
    pub executor: Box<dyn Executor>,
    pub validators: Vec<Box<dyn Validator>>,
    pub max_attempts: u8,
}
