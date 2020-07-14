use super::*;

#[allow(dead_code)]
#[derive(Debug)]
pub enum SolidityRuntimeFunction {
    Selector,
    CheckNoValue,
    CallValue,
    ComputeOffset,
    DecodeAsAddress,
    DecodeAsUInt,
    Return32Bytes,
    RevertIfGreater,
    StorageArrayOffset,
    StorageFixedSizeArrayOffset,
    StorageDictionaryOffsetForKey,
    AllocateMemory,
    Load,
    Store,
    Add,
    Sub,
    Mul,
    Div,
    Power,
    IsValidCallerProtection,
}

impl SolidityRuntimeFunction {
    pub fn mangle_runtime(&self) -> String {
        format!("Quartz${}", self)
    }

    pub fn call_value() -> String {
        "callvalue()".to_string()
    }

    pub fn selector() -> String {
        format!("{}()", SolidityRuntimeFunction::Selector.mangle_runtime())
    }

    pub fn is_valid_caller_protection(address: String) -> String {
        format!(
            "{name}({address})",
            name = SolidityRuntimeFunction::IsValidCallerProtection.mangle_runtime(),
            address = address
        )
    }

    pub fn revert_if_greater(value: YulExpression, max: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::RevertIfGreater.mangle_runtime(),
            arguments: vec![value, max],
        })
    }

    pub fn add_offset(
        expression: YulExpression,
        offset: YulExpression,
        in_mem: String,
    ) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::ComputeOffset.mangle_runtime(),
            arguments: vec![expression, offset, YulExpression::Identifier(in_mem)],
        })
    }

    pub fn decode_as_uint(offset: u64) -> String {
        format!(
            "{func}({offset})",
            func = SolidityRuntimeFunction::DecodeAsUInt.mangle_runtime(),
            offset = offset
        )
    }

    pub fn decode_as_address(offset: u64) -> String {
        format!(
            "{func}({offset})",
            func = SolidityRuntimeFunction::DecodeAsAddress.mangle_runtime(),
            offset = offset
        )
    }

    pub fn return_32_bytes(input: String) -> String {
        format!(
            "{func}({input})",
            func = SolidityRuntimeFunction::Return32Bytes.mangle_runtime(),
            input = input
        )
    }

    pub fn add_offset_bool(
        expression: YulExpression,
        offset: YulExpression,
        in_mem: bool,
    ) -> YulExpression {
        let offset = if in_mem {
            YulExpression::FunctionCall(YulFunctionCall {
                name: "mul".to_string(),
                arguments: vec![YulExpression::Literal(YulLiteral::Num(32)), offset],
            })
        } else {
            offset
        };
        YulExpression::FunctionCall(YulFunctionCall {
            name: "add".to_string(),
            arguments: vec![expression, offset],
        })
    }

    pub fn storage_array_offset(offset: YulExpression, index: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::StorageArrayOffset.mangle_runtime(),
            arguments: vec![offset, index],
        })
    }

    pub fn storage_fixed_array_offset(
        offset: YulExpression,
        index: YulExpression,
        size: u64,
    ) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::StorageFixedSizeArrayOffset.mangle_runtime(),
            arguments: vec![offset, index, YulExpression::Literal(YulLiteral::Num(size))],
        })
    }

    pub fn storage_dictionary_offset_key(
        offset: YulExpression,
        index: YulExpression,
    ) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::StorageDictionaryOffsetForKey.mangle_runtime(),
            arguments: vec![offset, index],
        })
    }

    pub fn allocate_memory(size: u64) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::AllocateMemory.mangle_runtime(),
            arguments: vec![YulExpression::Literal(YulLiteral::Num(size))],
        })
    }

    pub fn load(address: YulExpression, in_mem: String) -> YulExpression {
        let identifier = YulExpression::Identifier(in_mem);
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Load.mangle_runtime(),
            arguments: vec![address, identifier],
        })
    }

    pub fn load_bool(address: YulExpression, in_mem: bool) -> YulExpression {
        let name = if in_mem {
            "mload".to_string()
        } else {
            "sload".to_string()
        };
        YulExpression::FunctionCall(YulFunctionCall {
            name,
            arguments: vec![address],
        })
    }

    pub fn store(address: YulExpression, value: YulExpression, in_mem: String) -> YulExpression {
        let identifier = YulExpression::Identifier(in_mem);
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Store.mangle_runtime(),
            arguments: vec![address, value, identifier],
        })
    }

    pub fn store_bool(address: YulExpression, value: YulExpression, in_mem: bool) -> YulExpression {
        let name = if in_mem {
            "mstore".to_string()
        } else {
            "sstore".to_string()
        };
        YulExpression::FunctionCall(YulFunctionCall {
            name,
            arguments: vec![address, value],
        })
    }

    pub fn mul(a: YulExpression, b: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Mul.mangle_runtime(),
            arguments: vec![a, b],
        })
    }

    pub fn div(a: YulExpression, b: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Div.mangle_runtime(),
            arguments: vec![a, b],
        })
    }

    pub fn add(a: YulExpression, b: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Add.mangle_runtime(),
            arguments: vec![a, b],
        })
    }

    pub fn sub(a: YulExpression, b: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Sub.mangle_runtime(),
            arguments: vec![a, b],
        })
    }

    pub fn power(a: YulExpression, b: YulExpression) -> YulExpression {
        YulExpression::FunctionCall(YulFunctionCall {
            name: SolidityRuntimeFunction::Power.mangle_runtime(),
            arguments: vec![a, b],
        })
    }

    pub fn get_all_functions() -> Vec<String> {
        vec![
            SolidityRuntimeFunction::add_function(),
            SolidityRuntimeFunction::sub_function(),
            SolidityRuntimeFunction::mul_function(),
            SolidityRuntimeFunction::div_function(),
            SolidityRuntimeFunction::power_function(),
            SolidityRuntimeFunction::revert_if_greater_function(),
            SolidityRuntimeFunction::fatal_error_function(),
            SolidityRuntimeFunction::send_function(),
            SolidityRuntimeFunction::decode_address_function(),
            SolidityRuntimeFunction::decode_uint_function(),
            SolidityRuntimeFunction::selector_function(),
            SolidityRuntimeFunction::store_function(),
            SolidityRuntimeFunction::storage_dictionary_keys_array_offset_function(),
            SolidityRuntimeFunction::storage_offset_for_key_function(),
            SolidityRuntimeFunction::storage_dictionary_offset_for_key_function(),
            SolidityRuntimeFunction::storage_array_offset_function(),
            SolidityRuntimeFunction::is_invalid_subscript_expression_function(),
            SolidityRuntimeFunction::return_32_bytes_function(),
            SolidityRuntimeFunction::is_caller_protection_in_dictionary_function(),
            SolidityRuntimeFunction::is_caller_protection_in_array_function(),
            SolidityRuntimeFunction::is_valid_caller_protection_function(),
            SolidityRuntimeFunction::check_no_value_function(),
            SolidityRuntimeFunction::allocate_memory_function(),
            SolidityRuntimeFunction::compute_offset_function(),
            SolidityRuntimeFunction::load_function(),
        ]
    }

    pub fn add_function() -> String {
        "function Quartz$Add(a, b) -> ret { \n let c := add(a, b) \n if lt(c, a) { revert(0, 0) } \n ret := c \n }".to_string()
    }

    pub fn sub_function() -> String {
        "function Quartz$Sub(a, b) -> ret { \n if gt(b, a) { revert(0, 0) } \n ret := sub(a, b) \n }".to_string()
    }

    pub fn mul_function() -> String {
        "function Quartz$Mul(a, b) -> ret {
            switch iszero(a)
                case 1 {
                    ret := 0
                }
                default {
                    let c := mul(a, b)
                    if iszero(eq(div(c, a), b)) {
                        revert(0, 0)
                    }
                    ret := c
                }
        }"
        .to_string()
    }

    pub fn div_function() -> String {
        "function Quartz$Div(a, b) -> ret {
            if eq(b, 0) {
                revert(0, 0)
            }
            ret := div(a, b)
        }"
        .to_string()
    }

    pub fn power_function() -> String {
        "function Quartz$Power(b, e) -> ret {
            ret := 1
            for { let i := 0 } lt(i, e) { i := add(i, 1)}{
                ret := Quartz$Mul(ret, b)
            }
        }"
        .to_string()
    }

    pub fn revert_if_greater_function() -> String {
        "function Quartz$RevertIfGreater(a, b) -> ret {
            if gt(a, b) {
                revert(0, 0)
            }
            ret := a
        }"
        .to_string()
    }

    pub fn fatal_error_function() -> String {
        "function Quartz$FatalError() {
            revert(0, 0)
        }"
        .to_string()
    }

    pub fn send_function() -> String {
        "function Quartz$Send(_value, _address) {
            let ret := call(gas(), _address, _value, 0, 0, 0, 0)
            if iszero(ret) {
                revert(0, 0)
            }
        }"
        .to_string()
    }

    pub fn storage_dictionary_keys_array_offset_function() -> String {
        "function Quartz$StorageDictionaryKeysArrayOffset(dictionaryOffset) -> ret {
            mstore(0, dictionaryOffset)
            ret := keccak256(0, 32)
        }"
        .to_string()
    }

    pub fn storage_offset_for_key_function() -> String {
        "function Quartz$StorageOffsetForKey(offset, key) -> ret {
            mstore(0, key)
            mstore(32, offset)
            ret := keccak256(0, 64)
         }"
        .to_string()
    }

    pub fn storage_dictionary_offset_for_key_function() -> String {
        "function Quartz$StorageDictionaryOffsetForKey(dictionaryOffset, key) -> ret {
            let offsetForKey := Quartz$StorageOffsetForKey(dictionaryOffset, key)
            mstore(0, offsetForKey)
            let indexOffset := keccak256(0, 32)
            switch eq(sload(indexOffset), 0)
                case 1 {
                    let keysArrayOffset := Quartz$StorageDictionaryKeysArrayOffset(dictionaryOffset)
                    let index := add(sload(dictionaryOffset), 1)
                    sstore(indexOffset, index)
                    sstore(Quartz$StorageOffsetForKey(keysArrayOffset, index), key)
                    sstore(dictionaryOffset, index)
                }
            ret := offsetForKey
        }"
        .to_string()
    }

    pub fn storage_array_offset_function() -> String {
        "function Quartz$StorageArrayOffset(arrayOffset, index) -> ret {
            let arraySize := sload(arrayOffset)

            switch eq(arraySize, index)
            case 0 {
                if Quartz$IsInvalidSubscriptExpression(index, arraySize) { revert(0, 0) }
            }
            default {
                sstore(arrayOffset, Quartz$Add(arraySize, 1))
            }
            ret := Quartz$StorageOffsetForKey(arrayOffset, index)
        }"
        .to_string()
    }

    pub fn is_invalid_subscript_expression_function() -> String {
        "function Quartz$IsInvalidSubscriptExpression(index, arraySize) -> ret {
            ret := or(iszero(arraySize), or(lt(index, 0), gt(index, Quartz$Sub(arraySize, 1))))
        }"
        .to_string()
    }

    pub fn return_32_bytes_function() -> String {
        "function Quartz$Return32Bytes(v) {
            mstore(0, v)
            return(0, 0x20)
        }"
        .to_string()
    }

    pub fn is_caller_protection_in_dictionary_function() -> String {
        "function Quartz$IsCallerProtectionInDictionary(dictionaryOffset) -> ret {
            let size := sload(dictionaryOffset)
            let arrayOffset := Quartz$StorageDictionaryKeysArrayOffset(dictionaryOffset)
            let found := 0
            let _caller := caller()
            for { let i := 0 } and(lt(i, size), iszero(found)) { i := add(i, i) } {
                let key := sload(Quartz$StorageOffsetForKey(arrayOffset, i))
                if eq(sload(Quartz$StorageOffsetForKey(dictionaryOffset, key)), _caller) {
                    found := 1
                }
            }
            ret := found
        }"
        .to_string()
    }

    pub fn is_caller_protection_in_array_function() -> String {
        "function Quartz$IsCallerProtectionInArray(arrayOffset) -> ret {
            let size := sload(arrayOffset)
            let found := 0
            let _caller := caller()
            for { let i := 0 } and(lt(i, size), iszero(found)) { i := add(i, 1) } {
                if eq(sload(Quartz$StorageOffsetForKey(arrayOffset, i)), _caller) {
                found := 1
                }
            }
            ret := found
        }"
        .to_string()
    }

    pub fn is_valid_caller_protection_function() -> String {
        "function Quartz$IsValidCallerProtection(_address) -> ret {
            ret := eq(_address, caller())
         }"
        .to_string()
    }

    pub fn check_no_value_function() -> String {
        "function Quartz$CheckNoValue(_value) {
            if iszero(iszero(_value)) {
                Quartz$FatalError()
            }
        }"
        .to_string()
    }

    pub fn allocate_memory_function() -> String {
        "function Quartz$AllocateMemory(size) -> ret {
            ret := mload(0x40)
            mstore(0x40, add(ret, size))
        }"
        .to_string()
    }

    pub fn compute_offset_function() -> String {
        "function Quartz$ComputeOffset(base, offset, mem) -> ret {
            switch iszero(mem)
            case 0 {
                ret := add(base, mul(offset, 32))
            }
            default {
                ret := add(base, offset)
            }
        }"
        .to_string()
    }

    pub fn load_function() -> String {
        "function Quartz$Load(ptr, mem) -> ret {
            switch iszero(mem)
                case 0 {
                    ret := mload(ptr)
                }
                default {
                    ret := sload(ptr)
                }
        }"
        .to_string()
    }

    pub fn decode_address_function() -> String {
        "function Quartz$DecodeAsAddress(offset) -> ret { \n ret := Quartz$DecodeAsUInt(offset) \n }".to_string()
    }

    pub fn decode_uint_function() -> String {
        "function Quartz$DecodeAsUInt(offset) -> ret { \n ret := calldataload(add(4, mul(offset, 0x20))) \n }".to_string()
    }

    pub fn selector_function() -> String {
        "function Quartz$Selector() -> ret { \n ret := div(calldataload(0), 0x100000000000000000000000000000000000000000000000000000000) \n }".to_string()
    }

    pub fn store_function() -> String {
        "function Quartz$Store(ptr, val, mem) { \n switch iszero(mem) \n case 0 { \n mstore(ptr, val) \n } \n default { \n sstore(ptr, val) \n } \n  }".to_string()
    }
}

impl fmt::Display for SolidityRuntimeFunction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}