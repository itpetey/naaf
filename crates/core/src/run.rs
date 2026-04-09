use std::collections::HashMap;

use naaf_schema::artifact::ArtifactRef;

use crate::{ids::*, journal::RunEvent};

pub struct Run {
    pub id: RunId,
    pub workflow: &'static str,
    pub artifacts: HashMap<ArtifactId, ArtifactRef>,
    pub journal: Vec<RunEvent>,
}
