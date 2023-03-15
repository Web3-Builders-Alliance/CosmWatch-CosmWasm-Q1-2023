use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Only accepts tokens in the cw20_whitelist")]
    NotInWhitelist {},

    #[error("Escrow is expired")]
    Expired {},

    #[error("Escrow not found")]
    NotFound {},

    #[error("Address is invalid")]
    InvalidAddress {},

    #[error("Send some coins to create an escrow")]
    EmptyBalance {},

    #[error("Escrow id already in use")]
    AlreadyInUse {},

    #[error("Recipient is not set")]
    RecipientNotSet {},

    #[error("Milestone not found")]
    MilestoneNotFound,

    #[error("Milestones can't be empty")]
    EmptyMilestones,
}
