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
        //TODO: change function name?
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
        vec![MoveRuntimeFunction::get_power()]
        /* TURN OFF LIBRA
        vec![
            MoveRuntimeFunction::get_revert_if_greater(),
            MoveRuntimeFunction::get_array_funcs(),
            MoveRuntimeFunction::get_libra_internal(),
        ]
        */
    }

    /* TURN OFF LIBRA
      pub fn get_revert_if_greater() -> String {
          "Quartz_RevertIfGreater(a: u64, b: u64): u64 {  \n \
               assert(copy(a) <= move(b), 1); \n \
               return move(a); \n }"
              .to_string()
      }

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

      pub fn get_libra_internal() -> String {
          "Quartz_Self_Create_Libra(input: Libra.Libra<LBR.LBR>) : Self.Libra {
              return Self.Libra_produce(move(input));
          }

          public Libra_Coin_init(zero: address): Self.Libra_Coin {
          if (move(zero) != 0x0) {
            assert(false, 9001);
          }
          return Libra_Coin {
            coin: Libra.zero<LBR.LBR>()
          };
        }

        public Libra_Coin_getValue(this: &mut Self.Libra_Coin): u64 {
          let coin: &Libra.Libra<LBR.LBR>;
          coin = &move(this).coin;
          return Libra.value<LBR.LBR>(move(coin));
        }

        public Libra_Coin_withdraw(this: &mut Self.Libra_Coin, \
        amount: u64): Self.Libra_Coin {
          let coin: &mut Libra.Libra<LBR.LBR>;
          coin = &mut move(this).coin;
          return Libra_Coin {
            coin: Libra.withdraw<LBR.LBR>(move(coin), move(amount))
          };
        }

        public Libra_Coin_transfer(this: &mut Self.Libra_Coin, \
        other: &mut Self.Libra_Coin, amount: u64) {
          let coin: &mut Libra.Libra<LBR.LBR>;
          let other_coin: &mut Libra.Libra<LBR.LBR>;
          let temporary: Libra.Libra<LBR.LBR>;
          coin = &mut move(this).coin;
          temporary = Libra.withdraw<LBR.LBR>(move(coin), move(amount));
          other_coin = &mut move(other).coin;
          Libra.deposit<LBR.LBR>(move(other_coin), move(temporary));
          return;
        }
        public Libra_Coin_transfer_value(this: &mut Self.Libra_Coin, other: Self.Libra) {
          let coin: &mut Libra.Libra<LBR.LBR>;
          let temp: Self.Libra_Coin;
          let temporary: Libra.Libra<LBR.LBR>;
          coin = &mut move(this).coin;
          Libra {temp} = move(other);
          Libra_Coin {temporary} = move(temp);
          Libra.deposit<LBR.LBR>(move(coin), move(temporary));
          return;
      }

      public Libra_Coin_send(coin: &mut Self.Libra_Coin, payee: address, amount: u64) {
      let temporary: Libra.Libra<LBR.LBR>;
      let coin_ref: &mut Libra.Libra<LBR.LBR>;
      coin_ref = &mut move(coin).coin;
      temporary = Libra.withdraw<LBR.LBR>(move(coin_ref), move(amount));
      LibraAccount.deposit<LBR.LBR>(copy(payee), move(temporary));
      return;
    }

      Libra_Coin_produce (input: Libra.Libra<LBR.LBR>): Self.Libra_Coin {
          return Libra_Coin {
              coin: move(input)
          };
      }

      Libra_produce (input: Libra.Libra<LBR.LBR>): Self.Libra {
      return Libra {
        libra: Self.Libra_Coin_produce(move(input))
      };
    }

    init (): Self.Libra {
      return Self.Libra_init();
    }

    Quartz_Libra_send (this: &mut Self.Libra, _payee: address, _amount: u64)  {
      let _temp__5: &mut Self.Libra_Coin;
      _temp__5 = &mut copy(this).libra;
      Self.Libra_Coin_send(copy(_temp__5), copy(_payee), copy(_amount));
      _ = move(_temp__5);
      _ = move(this);
      return;
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
