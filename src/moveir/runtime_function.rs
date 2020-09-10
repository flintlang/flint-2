use super::ir::{MoveIRExpression, MoveIRFunctionCall};
use core::fmt;

#[allow(dead_code)]
#[derive(Debug)]
pub(crate) enum MoveRuntimeFunction {
    AppendToArrayInt,
    GetFromArrayInt,
    AssignToFixedArray,
    RevertIfGreater,
    Transfer,
    WithdrawAll,
    Power,
}

impl MoveRuntimeFunction {
    pub fn revert_if_greater(value: MoveIRExpression, max: MoveIRExpression) -> MoveIRExpression {
        MoveIRExpression::FunctionCall(MoveIRFunctionCall {
            identifier: MoveRuntimeFunction::RevertIfGreater.mangle_runtime(),
            arguments: vec![value, max],
        })
    }

    pub fn append_to_array_int(vec: MoveIRExpression, value: MoveIRExpression) -> MoveIRExpression {
        MoveIRExpression::FunctionCall(MoveIRFunctionCall {
            identifier: MoveRuntimeFunction::AppendToArrayInt.mangle_runtime(),
            arguments: vec![vec, value],
        })
    }

    pub fn get_from_array_int(vec: MoveIRExpression, value: MoveIRExpression) -> MoveIRExpression {
        MoveIRExpression::FunctionCall(MoveIRFunctionCall {
            identifier: MoveRuntimeFunction::GetFromArrayInt.mangle_runtime(),
            arguments: vec![vec, value],
        })
    }

    pub fn power(first: MoveIRExpression, second: MoveIRExpression) -> MoveIRExpression {
        MoveIRExpression::FunctionCall(MoveIRFunctionCall {
            identifier: MoveRuntimeFunction::Power.mangle_runtime(),
            arguments: vec![first, second],
        })
    }

    pub fn get_power() -> String {
        "_Power(b: u64, e: u64): u64 {
        let ret: u64;
        let i: u64;
        ret = 1;
        i = 0;
        while (copy(i) < copy(e)) { 
            ret = copy(ret) * copy(b); 
            i = copy(i) + 1; 
        } 
        _ = move(b);
        _ = move(e);
        _ = move(i);
        return move(ret); 
    }"
        .to_string()
    }

    pub fn mangle_runtime(&self) -> String {
        format!("Self._{}", self)
    }

    pub fn get_all_functions() -> Vec<String> {
        vec![
            MoveRuntimeFunction::get_power(),
            MoveRuntimeFunction::get_libra_internal(),
        ]
    }

    pub fn get_libra_internal() -> String {
        "
        public Flint_balanceOf(account: address): u64 {
            return LibraAccount.balance<LBR.LBR>(move(account));
        }

        public Flint_transfer(from: &signer, other: address, amount: u64) {
            let old_balance_sender: u64;
            let new_balance_sender: u64;
            let old_balance_recipient: u64;
            let new_balance_recipient: u64;
            let from_address: address;
            let with_cap: LibraAccount.WithdrawCapability;

            from_address = Signer.address_of(copy(from));

            old_balance_sender = LibraAccount.balance<LBR.LBR>(copy(from_address));
            old_balance_recipient = LibraAccount.balance<LBR.LBR>(copy(other));

            with_cap = LibraAccount.extract_withdraw_capability(copy(from));
            LibraAccount.pay_from<LBR.LBR>(&with_cap, copy(other), copy(amount), h\"\", h\"\");
            LibraAccount.restore_withdraw_capability(move(with_cap));

            new_balance_sender = LibraAccount.balance<LBR.LBR>(move(from_address));
            new_balance_recipient = LibraAccount.balance<LBR.LBR>(move(other));

            assert(copy(new_balance_sender) == copy(old_balance_sender) - copy(amount), 77);
            assert(copy(new_balance_recipient) == copy(old_balance_recipient) + move(amount), 77);

            return;
        }
        "
        .to_string()
    }
}

impl fmt::Display for MoveRuntimeFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
