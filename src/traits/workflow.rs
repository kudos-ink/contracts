use openbrush::{
    contracts::traits::ownable::*,
    modifiers,
    traits::AccountId,
};

#[cfg(feature = "std")]

/// Type alias for hashes.
pub type HashValue = [u8; 32];

#[openbrush::wrapper]
pub type WorkflowdRef = dyn Workflow + Ownable;

#[openbrush::trait_definition]
pub trait Workflow: Ownable {
    /// Register an aspiring contributor.
    #[ink(message)]
    fn register_identity(
        &mut self,
        account: AccountId,
        identity: HashValue,
    ) -> Result<(), WorkflowError>;

    /// Approve contribution. This is triggered by a workflow run.
    #[ink(message)]
    #[modifiers(only_owner)]
    fn approve(
        &mut self,
        contribution_id: u64,
        contributor_identity: HashValue,
    ) -> Result<(), WorkflowError>;

    /// Claim reward for a contributor.
    #[ink(message)]
    fn claim(&self, contribution_id: u64) -> Result<bool, WorkflowError>;
}

/// Errors that can occur upon calling this contract.
#[derive(Debug, PartialEq, Eq, scale::Encode, scale::Decode)]
#[cfg_attr(feature = "std", derive(::scale_info::TypeInfo))]
pub enum WorkflowError {
    OwnableError(OwnableError),
    /// An aspiring contributor identity is already registered in the DB.
    IdentityAlreadyRegistered,
    /// Contribution is already approved in the DB.
    ContributionAlreadyApproved,
    // Run id is already used in the DB.
    RunIdAlreadyUsed,
    /// Contributor identity is not registered in the DB.
    UnknownContributor,
    /// Contribution is not in the DB.
    UnknownContribution,
    /// Attempted reward payment to a contributor failed.
    PaymentFailed,
    /// Returned if caller is not the workflow `owner` while required to.
    CallerIsNotWorkflowOwner,
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
