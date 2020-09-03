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

    /* TURN OFF LIBRA
      #[allow(dead_code)]
      pub fn get_deposit() -> String {
          "Quartz_send(money: &mut Libra.Libra<LBR.LBR>, addr: address) { \n \
               LibraAccount.deposit(move(addr), Quartz_withdrawAll(move(money))); \n \
               return; \n }"
              .to_string()
      }

      pub fn get_array_funcs() -> String {
          "

          _GetFromArrayInt(vec: &mut vector<u64>, index: u64):u64 {
              return  *Vector.borrow<u64>(freeze(move(vec)), move(index));
          }


          _insert_array_index_u64(vec: &mut vector<u64>, index: u64, value: u64) {
      let length: u64;
      let temp: u64;
      length = Vector.length<u64>(freeze(copy(vec)));
      Vector.push_back<u64>(copy(vec), move(value));
      if (copy(length) == copy(index)) {
        Vector.swap<u64>(copy(vec), copy(index), copy(length));
        temp = Vector.pop_back<u64>(copy(vec));
        _ = move(temp);
      };
      _ = move(vec);
      return;
    }


    _insert_array_index_bool(vec: &mut vector<bool>, index: u64, value: bool) {
      let length: u64;
      let temp: bool;
      length = Vector.length<bool>(freeze(copy(vec)));
      Vector.push_back<bool>(copy(vec), move(value));
      if (copy(length) == copy(index)) {
        Vector.swap<bool>(copy(vec), copy(index), copy(length));
        temp = Vector.pop_back<bool>(copy(vec));
        _ = move(temp);
      };
      _ = move(vec);
      return;
    }"
          .to_string()
      }

      pub fn get_revert_if_greater() -> String {
          "Flint$RevertIfGreater(a: u64, b: u64): u64 {  \n \
               assert(copy(a) <= move(b), 1); \n \
               return move(a); \n \
           }"
              .to_string()
      }

      */
}

impl fmt::Display for MoveRuntimeFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
