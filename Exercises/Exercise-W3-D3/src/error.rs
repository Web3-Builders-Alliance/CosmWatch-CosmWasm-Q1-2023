use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Cannot send zero funds")]
    ZeroFunds {},

    #[error("Denom mismatch. Expected 'uluna'")]
    DenomMismatch {},

    #[error("Amount mismatch. Please check the amount sent and try again.")]
    AmountMismatch {},

    #[error("More than one token provided")]
    MoreThanOneToken {},
}
