use cosmwasm_std::Response;
use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, StdError, StdResult, Uint128};
use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg, TokenInfoResponse};

use crate::msg::TokenInstantiateInfo;
use crate::state::{BALANCES, TOKEN_INFO};
use crate::ContractError;

impl TokenInstantiateInfo {
    pub fn validate(&self) -> StdResult<()> {
        if !is_valid_name(&self.name) {
            return Err(StdError::generic_err(
                "Name is not in the expected format (3-50 UTF-8 bytes)",
            ));
        }
        if !is_valid_symbol(&self.symbol) {
            return Err(StdError::generic_err(
                "Ticker symbol is not in expected format [a-zA-Z\\-]{3,12}",
            ));
        }
        if self.decimals > 18 {
            return Err(StdError::generic_err("Decimals must not exceed 18"));
        }
        Ok(())
    }
}

pub fn create_accounts(deps: &mut DepsMut, accounts: &[Cw20Coin]) -> StdResult<Uint128> {
    let mut total_supply = Uint128::zero();
    for row in accounts {
        let address = deps.api.addr_validate(&row.address)?;
        BALANCES.save(deps.storage, &address, &row.amount)?;
        total_supply += row.amount;
    }
    Ok(total_supply)
}

pub fn execute_transfer(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let rcpt_addr = deps.api.addr_validate(&recipient)?;

    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", info.sender)
        .add_attribute("to", recipient)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_burn(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    amount: Uint128,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    // lower balance
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    // reduce total_supply
    TOKEN_INFO.update(deps.storage, |mut info| -> StdResult<_> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })?;

    let res = Response::new()
        .add_attribute("action", "burn")
        .add_attribute("from", info.sender)
        .add_attribute("amount", amount);
    Ok(res)
}

pub fn execute_send(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    contract: String,
    amount: Uint128,
    msg: Binary,
) -> Result<Response, ContractError> {
    if amount == Uint128::zero() {
        return Err(ContractError::InvalidZeroAmount {});
    }

    let rcpt_addr = deps.api.addr_validate(&contract)?;

    // move the tokens to the contract
    BALANCES.update(
        deps.storage,
        &info.sender,
        |balance: Option<Uint128>| -> StdResult<_> {
            Ok(balance.unwrap_or_default().checked_sub(amount)?)
        },
    )?;
    BALANCES.update(
        deps.storage,
        &rcpt_addr,
        |balance: Option<Uint128>| -> StdResult<_> { Ok(balance.unwrap_or_default() + amount) },
    )?;

    let res = Response::new()
        .add_attribute("action", "send")
        .add_attribute("from", &info.sender)
        .add_attribute("to", &contract)
        .add_attribute("amount", amount)
        .add_message(
            Cw20ReceiveMsg {
                sender: info.sender.into(),
                amount,
                msg,
            }
            .into_cosmos_msg(contract)?,
        );
    Ok(res)
}

pub fn query_balance(deps: Deps, address: String) -> StdResult<BalanceResponse> {
    let address = deps.api.addr_validate(&address)?;
    let balance = BALANCES
        .may_load(deps.storage, &address)?
        .unwrap_or_default();
    Ok(BalanceResponse { balance })
}

pub fn query_token_info(deps: Deps) -> StdResult<TokenInfoResponse> {
    let info = TOKEN_INFO.load(deps.storage)?;
    let res = TokenInfoResponse {
        name: info.name,
        symbol: info.symbol,
        decimals: info.decimals,
        total_supply: info.total_supply,
    };
    Ok(res)
}

fn is_valid_name(name: &str) -> bool {
    // The `len` method on a Rust string returns the number of bytes
    // in the string:
    // https://doc.rust-lang.org/std/primitive.str.html#method.len
    if name.len() < 3 || name.len() > 50 {
        return false;
    }
    true
}

fn is_valid_symbol(symbol: &str) -> bool {
    if symbol.len() < 3 || symbol.len() > 12 {
        return false;
    }
    let bytes = symbol.as_bytes();
    for byte in bytes.iter() {
        if (*byte != 45) && (*byte < 65 || *byte > 90) && (*byte < 97 || *byte > 122) {
            return false;
        }
    }
    true
}
