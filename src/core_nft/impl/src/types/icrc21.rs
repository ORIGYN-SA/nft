use icrc_ledger_types::icrc21::errors::Icrc21Error;
use icrc_ledger_types::icrc21::requests::ConsentMessageRequest;
use icrc_ledger_types::icrc21::responses::ConsentInfo;

pub type Args = ConsentMessageRequest;
pub type Response = Result<ConsentInfo, Icrc21Error>;
