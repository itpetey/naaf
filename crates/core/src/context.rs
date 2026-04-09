use crate::run::Run;

pub struct RunContext<'a> {
    pub run: &'a mut Run,
}
