use cosmwasm_std::StdError;
use cw_utils::ParseReplyError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    ParseReplyError(#[from] ParseReplyError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Amount mismatch")]
    AmountMismatch {},

    #[error("No funds sent")]
    NoFundsSent {},

    #[error("Amount has to be greater than zero")]
    InvalidAmount {},

    #[error("Invalid reply")]
    InvalidReply {},
}
