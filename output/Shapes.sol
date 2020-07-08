pragma solidity ^0.5.12;
contract Shapes {
  
  constructor() public {
    
    assembly {
      mstore(0x40, 0x60)
      
      init()
      
      function init() {
        codecopy(0x0), sub(codesize, 1), 32)
        let _rectangle := mload(0)
        let _caller := caller()
        Rectangle$init$Int_Int(add(0, 0), , Quartz$Mul(2, _rectangle), _rectangle)
      }
      ///////////////////////////////
      //STRUCT FUNCTIONS
      ///////////////////////////////
      function Wei$init$Int(_QuartzSelf, _QuartzSelf$isMem, _unsafeRawValue)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), 0, _QuartzSelf$isMem)
        switch iszero(eq(_unsafeRawValue, 0))
        case 1 {
          Quartz_Global$fatalError()
        }
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _unsafeRawValue, _QuartzSelf$isMem)
      }
      
      function Wei$init$$inoutWei(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), 0, _QuartzSelf$isMem)
        Wei$transfer$$inoutWei(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem)
      }
      
      function Wei$init$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, _amount)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), 0, _QuartzSelf$isMem)
        Wei$transfer$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, _amount)
      }
      
      function Wei$transfer$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, _amount)  {
        switch lt(Wei$getRawValue(_source, _source$isMem), _amount)
        case 1 {
          Quartz_Global$fatalError()
        }
        let _unused1 := Wei$setRawValue$Int(_source, _source$isMem, Quartz$Sub(Wei$getRawValue(_source, _source$isMem), _amount))
        let _unused2 := Wei$setRawValue$Int(_QuartzSelf, _QuartzSelf$isMem, Quartz$Add(Wei$getRawValue(_QuartzSelf, _QuartzSelf$isMem), _amount))
      }
      
      function Wei$transfer$$inoutWei(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem)  {
        Wei$transfer$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, Wei$getRawValue(_source, _source$isMem))
      }
      
      function Wei$setRawValue$Int(_QuartzSelf, _QuartzSelf$isMem, _value) -> ret {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _value, _QuartzSelf$isMem)
        ret := Quartz$Load(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _QuartzSelf$isMem)
      }
      
      function Wei$getRawValue(_QuartzSelf, _QuartzSelf$isMem) -> ret {
        ret := Quartz$Load(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _QuartzSelf$isMem)
      }
      
      function Quartz_Global$send$Address_$inoutWei(_address, _value$isMem, _value)  {
        let _w := Quartz$AllocateMemory(32)
        Wei$init$$inoutWei(_w, 1, _value, _value$isMem)
        Quartz$Send(Wei$getRawValue(_w, 1), _address)
      }
      
      function Quartz_Global$fatalError()  {
        Quartz$FatalError()
      }
      
      function Quartz_Global$assert$Bool(_condition)  {
        switch eq(_condition, 0)
        case 1 {
          Quartz$FatalError()
        }
      }
      
      function Rectangle$init$Int_Int(_QuartzSelf, _QuartzSelf$isMem, _width, _height)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _width, _QuartzSelf$isMem)
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 1, _QuartzSelf$isMem), _height, _QuartzSelf$isMem)
      }
      
      function Rectangle$diagonal$Int_Int(_QuartzSelf, _QuartzSelf$isMem, _wideness, _tallness) -> ret {
        ret := Quartz$Power(Quartz$Add(Quartz$Power(_wideness, 2), Quartz$Power(_tallness, 2)), 0)
      }
      
      ///////////////////////////////
      //RUNTIME FUNCTIONS
      ///////////////////////////////
      function Quartz$Add(a, b) -> ret {
        let c := add(a, b)
        if lt(c, a) { revert(0, 0) }
        ret := c
      }
      
      function Quartz$Sub(a, b) -> ret {
        if gt(b, a) { revert(0, 0) }
        ret := sub(a, b)
      }
      
      function Quartz$Mul(a, b) -> ret {
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
      }
      
      function Quartz$Div(a, b) -> ret {
        if eq(b, 0) {
          revert(0, 0)
        }
        ret := div(a, b)
      }
      
      function Quartz$Power(b, e) -> ret {
        ret := 1
        for { let i := 0 } lt(i, e) { i := add(i, 1)}{
          ret := Quartz$Mul(ret, b)
        }
      }
      
      function Quartz$RevertIfGreater(a, b) -> ret {
        if gt(a, b) {
          revert(0, 0)
        }
        ret := a
      }
      
      function Quartz$FatalError() {
        revert(0, 0)
      }
      
      function Quartz$Send(_value, _address) {
        let ret := call(gas(), _address, _value, 0, 0, 0, 0)
        if iszero(ret) {
          revert(0, 0)
        }
      }
      
      function Quartz$DecodeAsAddress(offset) -> ret {
        ret := Quartz$DecodeAsUInt(offset)
      }
      
      function Quartz$DecodeAsUInt(offset) -> ret {
        ret := calldataload(add(4, mul(offset, 0x20)))
      }
      
      function Quartz$Selector() -> ret {
        ret := div(calldataload(0), 0x100000000000000000000000000000000000000000000000000000000)
      }
      
      function Quartz$Store(ptr, val, mem) {
        switch iszero(mem)
        case 0 {
          mstore(ptr, val)
        }
        default {
          sstore(ptr, val)
        }
      }
      
      function Quartz$StorageDictionaryKeysArrayOffset(dictionaryOffset) -> ret {
        mstore(0, dictionaryOffset)
        ret := keccak256(0, 32)
      }
      
      function Quartz$StorageOffsetForKey(offset, key) -> ret {
        mstore(0, key)
        mstore(32, offset)
        ret := keccak256(0, 64)
      }
      
      function Quartz$StorageDictionaryOffsetForKey(dictionaryOffset, key) -> ret {
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
      }
      
      function Quartz$StorageArrayOffset(arrayOffset, index) -> ret {
        let arraySize := sload(arrayOffset)
        
        switch eq(arraySize, index)
        case 0 {
          if Quartz$IsInvalidSubscriptExpression(index, arraySize) { revert(0, 0) }
        }
        default {
          sstore(arrayOffset, Quartz$Add(arraySize, 1))
        }
        ret := Quartz$StorageOffsetForKey(arrayOffset, index)
      }
      
      function Quartz$IsInvalidSubscriptExpression(index, arraySize) -> ret {
        ret := or(iszero(arraySize), or(lt(index, 0), gt(index, Quartz$Sub(arraySize, 1))))
      }
      
      function Quartz$Return32Bytes(v) {
        mstore(0, v)
        return(0, 0x20)
      }
      
      function Quartz$IsCallerProtectionInDictionary(dictionaryOffset) -> ret {
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
      }
      
      function Quartz$IsCallerProtectionInArray(arrayOffset) -> ret {
        let size := sload(arrayOffset)
        let found := 0
        let _caller := caller()
        for { let i := 0 } and(lt(i, size), iszero(found)) { i := add(i, 1) } {
          if eq(sload(Quartz$StorageOffsetForKey(arrayOffset, i)), _caller) {
            found := 1
          }
        }
        ret := found
      }
      
      function Quartz$IsValidCallerProtection(_address) -> ret {
        ret := eq(_address, caller())
      }
      
      function Quartz$CheckNoValue(_value) {
        if iszero(iszero(_value)) {
          Quartz$FatalError()
        }
      }
      
      function Quartz$AllocateMemory(size) -> ret {
        ret := mload(0x40)
        mstore(0x40, add(ret, size))
      }
      
      function Quartz$ComputeOffset(base, offset, mem) -> ret {
        switch iszero(mem)
        case 0 {
          ret := add(base, mul(offset, 32))
        }
        default {
          ret := add(base, offset)
        }
      }
      
      function Quartz$Load(ptr, mem) -> ret {
        switch iszero(mem)
        case 0 {
          ret := mload(ptr)
        }
        default {
          ret := sload(ptr)
        }
      }
    }
  }
  
  function () external payable {
    assembly {
      
      mstore(0x40, 0x60)
      
      ///////////////////////////////
      //SELECTOR
      ///////////////////////////////
      if eq(sload(add(0, 2)), 10000) { revert(0, 0)}
      switch Quartz$Selector()
      case 0x4d41892f {
        Quartz$CheckNoValue(callvalue())
        Quartz$Return32Bytes(Shapes$area()) }
      case 0x5cba7ea6 {
        Quartz$CheckNoValue(callvalue())
        Quartz$Return32Bytes(Shapes$semiPerimeter()) }
      case 0x2cf6598f {
        Quartz$CheckNoValue(callvalue())
        Quartz$Return32Bytes(Shapes$perimeter()) }
      case 0xdea641b3 {
        Quartz$CheckNoValue(callvalue())
        Quartz$Return32Bytes(Shapes$smallerWidth$Int(Quartz$DecodeAsUInt(0))) }
      default {
        revert(0, 0)
      }
      
      ///////////////////////////////
      //USER DEFINED FUNCTIONS
      ///////////////////////////////
      function Shapes$area() -> ret {
        let _caller := caller()
        ret := Quartz$Mul(sload(add(0, 0)), sload(add(0, 1)))
      }
      function Shapes$semiPerimeter() -> ret {
        let _caller := caller()
        ret := Quartz$Add(sload(add(0, 0)), sload(add(0, 1)))
      }
      function Shapes$perimeter() -> ret {
        let _caller := caller()
        ret := Quartz$Mul(2, Shapes$semiPerimeter(add(0, add(0, add(0, add(0, 2)))), ))
      }
      function Shapes$smallerWidth$Int(_otherRectWidth) -> ret {
        let _caller := caller()
        ret := lt(sload(add(add(0, 0), 0)), _otherRectWidth)
      }
      
      ///////////////////////////////
      //WRAPPER FUNCTIONS
      ///////////////////////////////
      ///////////////////////////////
      //STRUCT FUNCTIONS
      ///////////////////////////////
      function Wei$init$Int(_QuartzSelf, _QuartzSelf$isMem, _unsafeRawValue)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), 0, _QuartzSelf$isMem)
        switch iszero(eq(_unsafeRawValue, 0))
        case 1 {
          Quartz_Global$fatalError()
        }
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _unsafeRawValue, _QuartzSelf$isMem)
      }
      
      function Wei$init$$inoutWei(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), 0, _QuartzSelf$isMem)
        Wei$transfer$$inoutWei(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem)
      }
      
      function Wei$init$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, _amount)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), 0, _QuartzSelf$isMem)
        Wei$transfer$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, _amount)
      }
      
      function Wei$transfer$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, _amount)  {
        switch lt(Wei$getRawValue(_source, _source$isMem), _amount)
        case 1 {
          Quartz_Global$fatalError()
        }
        let _unused1 := Wei$setRawValue$Int(_source, _source$isMem, Quartz$Sub(Wei$getRawValue(_source, _source$isMem), _amount))
        let _unused2 := Wei$setRawValue$Int(_QuartzSelf, _QuartzSelf$isMem, Quartz$Add(Wei$getRawValue(_QuartzSelf, _QuartzSelf$isMem), _amount))
      }
      
      function Wei$transfer$$inoutWei(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem)  {
        Wei$transfer$$inoutWei_Int(_QuartzSelf, _QuartzSelf$isMem, _source, _source$isMem, Wei$getRawValue(_source, _source$isMem))
      }
      
      function Wei$setRawValue$Int(_QuartzSelf, _QuartzSelf$isMem, _value) -> ret {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _value, _QuartzSelf$isMem)
        ret := Quartz$Load(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _QuartzSelf$isMem)
      }
      
      function Wei$getRawValue(_QuartzSelf, _QuartzSelf$isMem) -> ret {
        ret := Quartz$Load(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _QuartzSelf$isMem)
      }
      
      function Quartz_Global$send$Address_$inoutWei(_address, _value$isMem, _value)  {
        let _w := Quartz$AllocateMemory(32)
        Wei$init$$inoutWei(_w, 1, _value, _value$isMem)
        Quartz$Send(Wei$getRawValue(_w, 1), _address)
      }
      
      function Quartz_Global$fatalError()  {
        Quartz$FatalError()
      }
      
      function Quartz_Global$assert$Bool(_condition)  {
        switch eq(_condition, 0)
        case 1 {
          Quartz$FatalError()
        }
      }
      
      function Rectangle$init$Int_Int(_QuartzSelf, _QuartzSelf$isMem, _width, _height)  {
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 0, _QuartzSelf$isMem), _width, _QuartzSelf$isMem)
        Quartz$Store(Quartz$ComputeOffset(_QuartzSelf, 1, _QuartzSelf$isMem), _height, _QuartzSelf$isMem)
      }
      
      function Rectangle$diagonal$Int_Int(_QuartzSelf, _QuartzSelf$isMem, _wideness, _tallness) -> ret {
        ret := Quartz$Power(Quartz$Add(Quartz$Power(_wideness, 2), Quartz$Power(_tallness, 2)), 0)
      }
      
      ///////////////////////////////
      //RUNTIME FUNCTIONS
      ///////////////////////////////
      function Quartz$Add(a, b) -> ret {
        let c := add(a, b)
        if lt(c, a) { revert(0, 0) }
        ret := c
      }
      
      function Quartz$Sub(a, b) -> ret {
        if gt(b, a) { revert(0, 0) }
        ret := sub(a, b)
      }
      
      function Quartz$Mul(a, b) -> ret {
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
      }
      
      function Quartz$Div(a, b) -> ret {
        if eq(b, 0) {
          revert(0, 0)
        }
        ret := div(a, b)
      }
      
      function Quartz$Power(b, e) -> ret {
        ret := 1
        for { let i := 0 } lt(i, e) { i := add(i, 1)}{
          ret := Quartz$Mul(ret, b)
        }
      }
      
      function Quartz$RevertIfGreater(a, b) -> ret {
        if gt(a, b) {
          revert(0, 0)
        }
        ret := a
      }
      
      function Quartz$FatalError() {
        revert(0, 0)
      }
      
      function Quartz$Send(_value, _address) {
        let ret := call(gas(), _address, _value, 0, 0, 0, 0)
        if iszero(ret) {
          revert(0, 0)
        }
      }
      
      function Quartz$DecodeAsAddress(offset) -> ret {
        ret := Quartz$DecodeAsUInt(offset)
      }
      
      function Quartz$DecodeAsUInt(offset) -> ret {
        ret := calldataload(add(4, mul(offset, 0x20)))
      }
      
      function Quartz$Selector() -> ret {
        ret := div(calldataload(0), 0x100000000000000000000000000000000000000000000000000000000)
      }
      
      function Quartz$Store(ptr, val, mem) {
        switch iszero(mem)
        case 0 {
          mstore(ptr, val)
        }
        default {
          sstore(ptr, val)
        }
      }
      
      function Quartz$StorageDictionaryKeysArrayOffset(dictionaryOffset) -> ret {
        mstore(0, dictionaryOffset)
        ret := keccak256(0, 32)
      }
      
      function Quartz$StorageOffsetForKey(offset, key) -> ret {
        mstore(0, key)
        mstore(32, offset)
        ret := keccak256(0, 64)
      }
      
      function Quartz$StorageDictionaryOffsetForKey(dictionaryOffset, key) -> ret {
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
      }
      
      function Quartz$StorageArrayOffset(arrayOffset, index) -> ret {
        let arraySize := sload(arrayOffset)
        
        switch eq(arraySize, index)
        case 0 {
          if Quartz$IsInvalidSubscriptExpression(index, arraySize) { revert(0, 0) }
        }
        default {
          sstore(arrayOffset, Quartz$Add(arraySize, 1))
        }
        ret := Quartz$StorageOffsetForKey(arrayOffset, index)
      }
      
      function Quartz$IsInvalidSubscriptExpression(index, arraySize) -> ret {
        ret := or(iszero(arraySize), or(lt(index, 0), gt(index, Quartz$Sub(arraySize, 1))))
      }
      
      function Quartz$Return32Bytes(v) {
        mstore(0, v)
        return(0, 0x20)
      }
      
      function Quartz$IsCallerProtectionInDictionary(dictionaryOffset) -> ret {
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
      }
      
      function Quartz$IsCallerProtectionInArray(arrayOffset) -> ret {
        let size := sload(arrayOffset)
        let found := 0
        let _caller := caller()
        for { let i := 0 } and(lt(i, size), iszero(found)) { i := add(i, 1) } {
          if eq(sload(Quartz$StorageOffsetForKey(arrayOffset, i)), _caller) {
            found := 1
          }
        }
        ret := found
      }
      
      function Quartz$IsValidCallerProtection(_address) -> ret {
        ret := eq(_address, caller())
      }
      
      function Quartz$CheckNoValue(_value) {
        if iszero(iszero(_value)) {
          Quartz$FatalError()
        }
      }
      
      function Quartz$AllocateMemory(size) -> ret {
        ret := mload(0x40)
        mstore(0x40, add(ret, size))
      }
      
      function Quartz$ComputeOffset(base, offset, mem) -> ret {
        switch iszero(mem)
        case 0 {
          ret := add(base, mul(offset, 32))
        }
        default {
          ret := add(base, offset)
        }
      }
      
      function Quartz$Load(ptr, mem) -> ret {
        switch iszero(mem)
        case 0 {
          ret := mload(ptr)
        }
        default {
          ret := sload(ptr)
        }
      }
    }
  }
}
interface _InterfaceShapes {
  function area() view external returns ( uint256 ret);
  function semiPerimeter() view external returns ( uint256 ret);
  function perimeter() view external returns ( uint256 ret);
  function smallerWidth(uint256 _otherRectWidth) view external returns ( uint256 ret);
}
