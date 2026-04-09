use naaf_schema::{artifact::ArtifactRef, finding::Finding};

pub trait Validator {
    fn id(&self) -> &'static str;

    fn validate(&self, artifact: &ArtifactRef) -> Result<Vec<Finding>, String>;
}
