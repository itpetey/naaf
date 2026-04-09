use naaf_schema::{artifact::ArtifactRef, finding::Finding};

use crate::validator::Validator;

pub struct DiffSizeValidator;

impl Validator for DiffSizeValidator {
    fn id(&self) -> &'static str {
        "diff_size"
    }

    fn validate(&self, _artifact: &ArtifactRef) -> Result<Vec<Finding>, String> {
        // stub
        Ok(vec![])
    }
}
