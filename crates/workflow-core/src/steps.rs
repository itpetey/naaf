use crate::budget::{ExecCtx, Services};
use crate::errors::{StepError, ValidationError};
use crate::route::RouteDecision;

pub type BoxError = Box<dyn std::error::Error + Send + Sync>;

pub trait Transformer: Send + Sync {
    fn name(&self) -> &'static str;
    fn transform(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        input: workflow_schema::state::StateEnvelope,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError>
    where
        Self: Sized;

    type Services: Services;
}

pub trait Router: Send + Sync {
    fn name(&self) -> &'static str;
    fn route(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<RouteDecision, StepError>
    where
        Self: Sized;

    type Services: Services;
}

pub trait Reducer: Send + Sync {
    fn name(&self) -> &'static str;
    fn reduce(
        &self,
        ctx: &mut ExecCtx<Self::Services>,
        inputs: Vec<workflow_schema::state::StateEnvelope>,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError>
    where
        Self: Sized;

    type Services: Services;
}

pub trait Validator: Send + Sync {
    fn name(&self) -> &'static str;
    fn validate(
        &self,
        ctx: &ExecCtx<Self::Services>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<(), ValidationError>
    where
        Self: Sized;

    type Services: Services;
}

pub struct BoxedTransformer<S: Services> {
    boxed: Box<dyn DynTransformer<S>>,
}

pub struct BoxedRouter<S: Services> {
    boxed: Box<dyn DynRouter<S>>,
}

pub struct BoxedReducer<S: Services> {
    boxed: Box<dyn DynReducer<S>>,
}

pub struct BoxedValidator<S: Services> {
    boxed: Box<dyn DynValidator<S>>,
}

pub trait DynTransformer<S: Services>: Send + Sync {
    fn name(&self) -> &'static str;
    fn transform(
        &self,
        ctx: &mut ExecCtx<S>,
        input: workflow_schema::state::StateEnvelope,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError>;
}

pub trait DynRouter<S: Services>: Send + Sync {
    fn name(&self) -> &'static str;
    fn route(
        &self,
        ctx: &mut ExecCtx<S>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<RouteDecision, StepError>;
}

pub trait DynReducer<S: Services>: Send + Sync {
    fn name(&self) -> &'static str;
    fn reduce(
        &self,
        ctx: &mut ExecCtx<S>,
        inputs: Vec<workflow_schema::state::StateEnvelope>,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError>;
}

pub trait DynValidator<S: Services>: Send + Sync {
    fn name(&self) -> &'static str;
    fn validate(
        &self,
        ctx: &ExecCtx<S>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<(), ValidationError>;
}

impl<T, S: Services> DynTransformer<S> for T
where
    T: Transformer<Services = S> + Send + Sync,
{
    fn name(&self) -> &'static str {
        Transformer::name(self)
    }

    fn transform(
        &self,
        ctx: &mut ExecCtx<S>,
        input: workflow_schema::state::StateEnvelope,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError> {
        Transformer::transform(self, ctx, input)
    }
}

impl<T, S: Services> DynRouter<S> for T
where
    T: Router<Services = S> + Send + Sync,
{
    fn name(&self) -> &'static str {
        Router::name(self)
    }

    fn route(
        &self,
        ctx: &mut ExecCtx<S>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        Router::route(self, ctx, state)
    }
}

impl<T, S: Services> DynReducer<S> for T
where
    T: Reducer<Services = S> + Send + Sync,
{
    fn name(&self) -> &'static str {
        Reducer::name(self)
    }

    fn reduce(
        &self,
        ctx: &mut ExecCtx<S>,
        inputs: Vec<workflow_schema::state::StateEnvelope>,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError> {
        Reducer::reduce(self, ctx, inputs)
    }
}

impl<T, S: Services> DynValidator<S> for T
where
    T: Validator<Services = S> + Send + Sync,
{
    fn name(&self) -> &'static str {
        Validator::name(self)
    }

    fn validate(
        &self,
        ctx: &ExecCtx<S>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<(), ValidationError> {
        Validator::validate(self, ctx, state)
    }
}

impl<S: Services> BoxedTransformer<S> {
    pub fn new<T>(inner: T) -> Self
    where
        T: Transformer<Services = S> + Send + Sync + 'static,
    {
        Self {
            boxed: Box::new(inner),
        }
    }

    pub fn name(&self) -> &'static str {
        self.boxed.name()
    }

    pub fn transform(
        &self,
        ctx: &mut ExecCtx<S>,
        input: workflow_schema::state::StateEnvelope,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError> {
        self.boxed.transform(ctx, input)
    }
}

impl<S: Services> BoxedRouter<S> {
    pub fn new<T>(inner: T) -> Self
    where
        T: Router<Services = S> + Send + Sync + 'static,
    {
        Self {
            boxed: Box::new(inner),
        }
    }

    pub fn name(&self) -> &'static str {
        self.boxed.name()
    }

    pub fn route(
        &self,
        ctx: &mut ExecCtx<S>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<RouteDecision, StepError> {
        self.boxed.route(ctx, state)
    }
}

impl<S: Services> BoxedReducer<S> {
    pub fn new<T>(inner: T) -> Self
    where
        T: Reducer<Services = S> + Send + Sync + 'static,
    {
        Self {
            boxed: Box::new(inner),
        }
    }

    pub fn name(&self) -> &'static str {
        self.boxed.name()
    }

    pub fn reduce(
        &self,
        ctx: &mut ExecCtx<S>,
        inputs: Vec<workflow_schema::state::StateEnvelope>,
    ) -> Result<workflow_schema::state::StateEnvelope, StepError> {
        self.boxed.reduce(ctx, inputs)
    }
}

impl<S: Services> BoxedValidator<S> {
    pub fn new<T>(inner: T) -> Self
    where
        T: Validator<Services = S> + Send + Sync + 'static,
    {
        Self {
            boxed: Box::new(inner),
        }
    }

    pub fn name(&self) -> &'static str {
        self.boxed.name()
    }

    pub fn validate(
        &self,
        ctx: &ExecCtx<S>,
        state: &workflow_schema::state::StateEnvelope,
    ) -> Result<(), ValidationError> {
        self.boxed.validate(ctx, state)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct DummyServices;
    impl Services for DummyServices {
        type Error = std::io::Error;
        async fn call(&self, _: &str, _: &[u8]) -> Result<Vec<u8>, Self::Error> {
            Ok(vec![])
        }
    }

    struct TestTransformer;
    impl Transformer for TestTransformer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "TestTransformer"
        }

        fn transform(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            input: workflow_schema::state::StateEnvelope,
        ) -> Result<workflow_schema::state::StateEnvelope, StepError> {
            Ok(input)
        }
    }

    #[test]
    fn boxed_transformer_new() {
        let bt = BoxedTransformer::<DummyServices>::new(TestTransformer);
        assert_eq!(bt.name(), "TestTransformer");
    }

    struct TestRouter;
    impl Router for TestRouter {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "TestRouter"
        }

        fn route(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            _state: &workflow_schema::state::StateEnvelope,
        ) -> Result<RouteDecision, StepError> {
            Ok(RouteDecision::Terminal)
        }
    }

    #[test]
    fn boxed_router_new() {
        let br = BoxedRouter::<DummyServices>::new(TestRouter);
        assert_eq!(br.name(), "TestRouter");
    }

    struct TestReducer;
    impl Reducer for TestReducer {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "TestReducer"
        }

        fn reduce(
            &self,
            _ctx: &mut ExecCtx<Self::Services>,
            inputs: Vec<workflow_schema::state::StateEnvelope>,
        ) -> Result<workflow_schema::state::StateEnvelope, StepError> {
            inputs
                .into_iter()
                .next()
                .ok_or_else(|| StepError::reducer("TestReducer", "empty inputs"))
        }
    }

    #[test]
    fn boxed_reducer_new() {
        let bred = BoxedReducer::<DummyServices>::new(TestReducer);
        assert_eq!(bred.name(), "TestReducer");
    }

    struct TestValidator;
    impl Validator for TestValidator {
        type Services = DummyServices;

        fn name(&self) -> &'static str {
            "TestValidator"
        }

        fn validate(
            &self,
            _ctx: &ExecCtx<Self::Services>,
            _state: &workflow_schema::state::StateEnvelope,
        ) -> Result<(), ValidationError> {
            Ok(())
        }
    }

    #[test]
    fn boxed_validator_new() {
        let bv = BoxedValidator::<DummyServices>::new(TestValidator);
        assert_eq!(bv.name(), "TestValidator");
    }
}
