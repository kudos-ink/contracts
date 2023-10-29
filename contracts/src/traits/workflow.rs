use openbrush::{contracts::traits::ownable::*, modifiers};
use super::types::HashValue;

#[openbrush::wrapper]
pub type WorkflowdRef = dyn Workflow + Ownable;

#[openbrush::trait_definition]
pub trait Workflow: Ownable {
    /// Register the caller as an aspiring contributor.
    #[ink(message)]
    fn register_identity(&mut self, identity: HashValue) -> Result<(), WorkflowError>;

    /// Approve contribution. This is triggered by a workflow run.
    #[ink(message)]
    #[modifiers(only_owner)]
    fn approve(
        &mut self,
        contribution_id: u64,
        contributor_identity: HashValue,
    ) -> Result<(), WorkflowError>;

    /// Check the ability to claim for a given `contribution_id`.
    #[ink(message)]
    fn can_claim(&self, contribution_id: u64) -> Result<bool, WorkflowError>;

    /// Claim reward for a given `contribution_id`.
    #[ink(message)]
    fn claim(&mut self, contribution_id: u64) -> Result<(), WorkflowError>;
}

/// Errors that can occur upon calling this contract.
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
pub enum WorkflowError {
    OwnableError(OwnableError),
    /// An aspiring contributor identity is already registered in the DB.
    IdentityAlreadyRegistered,
    /// A `contribution` is already approved in the DB.
    ContributionAlreadyApproved,
    /// No `contribution` is approved yet in the DB.
    NoContributionApprovedYet,
    /// Contributor identity is not registered in the DB.
    UnknownContributor,
    /// Contribution is not in the DB.
    UnknownContribution,
    /// Attempted reward payment to a contributor failed.
    PaymentFailed,
    /// Returned if caller is not the `contributor` while required to.
    CallerIsNotContributor,
    /// Returned when attempting to claim an already claimed reward.
    AlreadyClaimed,
}

impl From<OwnableError> for WorkflowError {
    fn from(error: OwnableError) -> Self {
        WorkflowError::OwnableError(error)
    }
}
